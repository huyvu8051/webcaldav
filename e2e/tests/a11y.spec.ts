import AxeBuilder from '@axe-core/playwright'
import { test, expect, seedSession, waitForIdle, waitForHydration, isoDay, dayCell, localDateTime, CREDS } from '../fixtures/app'

/**
 * Accessibility.
 *
 * Two halves. The axe sweep catches the systematic things across every state
 * the page can be in; the handwritten tests cover what a scanner cannot see --
 * tab order, what a day cell actually announces, whether a failed save is
 * audible at all.
 *
 * This file runs on both projects, and the desktop one is not optional:
 * WebKit will not move focus with Tab unless Full Keyboard Access is enabled,
 * so a keyboard-order assertion there passes by never moving at all.
 */

// Real Tailwind, because contrast and focus rings are computed styles.
test.use({ realTailwind: true })

const WCAG = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'best-practice']

// Nothing is excluded. `color-contrast` and `select-name` used to be, while
// A1 and A2 were open; both are fixed, so the sweep is unfiltered again.

async function gotoCalendar(page: import('@playwright/test').Page) {
  await seedSession(page)
  await page.goto('/calendar')
  await waitForIdle(page)
}

test.describe('axe sweep', () => {
  test('the connect page has no untracked violations', async ({ mock, page }) => {
    void mock
    await page.goto('/')
    await expect(page.getByTestId('connect-submit')).toBeVisible()
    const r = await new AxeBuilder({ page }).withTags(WCAG).analyze()
    expect(summarise(r.violations)).toEqual([])
  })

  for (const state of ['bare', 'day selected', 'form open', 'picker open'] as const) {
    test(`the calendar has no untracked violations: ${state}`, async ({ mock, page }) => {
      void mock
      await gotoCalendar(page)
      if (state !== 'bare') await dayCell(page, isoDay(0)).click()
      if (state === 'form open' || state === 'picker open') await page.getByTestId('add-event').click()
      if (state === 'picker open') await page.getByTestId('month-title').click()

      const r = await new AxeBuilder({ page }).withTags(WCAG).analyze()
      expect(summarise(r.violations)).toEqual([])
    })
  }

  test('an error banner introduces no violations', async ({ mock, page }) => {
    mock.fail({ match: 'REPORT', status: 500 })
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('status-error')).toBeVisible()
    const r = await new AxeBuilder({ page }).withTags(WCAG).analyze()
    expect(summarise(r.violations)).toEqual([])
  })
})

test.describe('document basics', () => {
  test('the page declares a language and a title', async ({ mock, page }) => {
    void mock
    await page.goto('/')
    await expect(page.locator('html')).toHaveAttribute('lang', /^[a-z]{2}/)
    expect((await page.title()).trim().length).toBeGreaterThan(0)
  })

  test('the landmarks are present and unique', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    // header/main/footer map to banner/main/contentinfo without needing roles.
    await expect(page.locator('header')).toHaveCount(1)
    await expect(page.locator('main')).toHaveCount(1)
    await expect(page.locator('footer')).toHaveCount(1)
  })
})

test.describe('keyboard', () => {
  test('tab reaches every control in the order they are read', async ({ mock, page, browserName }) => {
    // Not measurable on WebKit: Safari only tabs to buttons when Full Keyboard
    // Access is on, so asserting an order there would assert that focus never
    // moved. Chromium is the honest place to check this.
    test.skip(browserName === 'webkit', 'WebKit does not tab to buttons by default')
    void mock
    await gotoCalendar(page)
    const seen: string[] = []
    for (let i = 0; i < 6; i++) {
      await page.keyboard.press('Tab')
      seen.push(
        await page.evaluate(() => (document.activeElement as HTMLElement)?.getAttribute('data-testid') ?? 'none'),
      )
    }
    expect(seen).toEqual(['disconnect', 'month-title', 'today-button', 'month-prev', 'month-next', 'add-event'])
  })

  test('day cells are reachable and both Enter and Space pick a day', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    // A real <button>, not a div with a click handler -- which is the whole
    // reason this works without any roving-tabindex machinery.
    const first = dayCell(page, isoDay(0))
    await first.focus()
    await page.keyboard.press('Enter')
    await expect(first).toHaveAttribute('data-selected', 'true')

    const second = dayCell(page, isoDay(1))
    await second.focus()
    await page.keyboard.press('Space')
    await expect(second).toHaveAttribute('data-selected', 'true')
  })

  test('every control shows a focus indicator when tabbed to', async ({ mock, page, browserName }) => {
    test.skip(browserName === 'webkit', 'WebKit does not tab to buttons by default')
    void mock
    await gotoCalendar(page)
    await page.getByTestId('add-event').click()

    // Measured by actually tabbing. A programmatic .focus() does not set
    // :focus-visible, so reading the computed style after one would report
    // "no outline" for every element on the page -- which is how the first
    // version of this test flagged 62 false positives.
    const bare: string[] = []
    for (let i = 0; i < 14; i++) {
      await page.keyboard.press('Tab')
      const info = await page.evaluate(() => {
        const e = document.activeElement as HTMLElement | null
        if (!e || e === document.body) return null
        const s = getComputedStyle(e)
        const outlined = s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) > 0
        const ringed = s.boxShadow !== 'none'
        return {
          id: e.getAttribute('data-testid') ?? e.tagName.toLowerCase(),
          visible: outlined || ringed,
        }
      })
      if (info && !info.visible) bare.push(info.id)
    }
    expect(bare).toEqual([])
  })

  test('the month picker announces whether it is open', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await expect(page.getByTestId('month-title')).toHaveAttribute('aria-expanded', 'false')
    await page.getByTestId('month-title').click()
    await expect(page.getByTestId('month-title')).toHaveAttribute('aria-expanded', 'true')
  })
})

test.describe('names and states', () => {
  test('every icon-only button has a label', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await dayCell(page, isoDay(0)).click()

    const unnamed = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLElement>('button')]
        .filter((e) => e.getBoundingClientRect().width > 0)
        // A button whose only content is a glyph span has no readable text.
        .filter((e) => !!e.querySelector('.material-symbols-outlined'))
        .filter((e) => {
          const text = (e.textContent ?? '').replace(/[a-z_]+/g, (m) =>
            e.querySelector('.material-symbols-outlined')?.textContent === m ? '' : m,
          )
          return !text.trim() && !e.getAttribute('aria-label') && !e.getAttribute('aria-labelledby')
        })
        .map((e) => e.outerHTML.slice(0, 90)),
    )
    expect(unnamed).toEqual([])
  })

  test('buttons report their busy state while they work', async ({ mock, page }) => {
    await gotoCalendar(page)
    const release = mock.hold('PUT')
    await page.getByTestId('add-event').click()
    await page.getByTestId('form-summary').fill('Busy')
    await page.getByTestId('form-start').fill(localDateTime(0, 9))
    await page.getByTestId('form-end').fill(localDateTime(0, 10))
    await page.getByTestId('form-submit').click()

    await expect(page.getByTestId('form-submit')).toHaveAttribute('aria-busy', 'true')
    await expect(page.getByTestId('form-submit')).toBeDisabled()
    release()
    await expect(page.getByTestId('form-submit')).toHaveCount(0)
  })

  test('the spinner is hidden from assistive technology', async ({ mock, page }) => {
    await gotoCalendar(page)
    const release = mock.hold('REPORT')
    await page.getByTestId('refresh').click()
    // The adjacent text says "Updating…"; a spinner announcing itself as well
    // would be the same fact twice.
    await expect(page.getByTestId('spinner').first()).toHaveAttribute('aria-hidden', 'true')
    release()
  })

  test('form fields are described by something', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await page.getByTestId('add-event').click()
    const bare = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLElement>('input, select, textarea')]
        .filter((e) => e.getBoundingClientRect().width > 0)
        .filter(
          (e) =>
            !e.getAttribute('aria-label') &&
            !e.getAttribute('aria-labelledby') &&
            !e.closest('label') &&
            !(e.id && document.querySelector(`label[for="${e.id}"]`)) &&
            !e.getAttribute('placeholder'),
        )
        .map((e) => `${e.tagName.toLowerCase()}[${e.getAttribute('data-testid')}]`),
    )
    // Tracked below: the selects fail this, and a placeholder is a weak pass.
    expect(bare.every((b) => b.startsWith('select'))).toBe(true)
  })
})

test.describe('what a screen reader is told', () => {
  test('every select has an accessible name', async ({ mock, page }) => {
    // They used to lean on their first <option> as a pseudo-label -- "Priority",
    // "Status", "No reminder" -- which a screen reader never announces as a
    // label, only as the current value. axe graded that critical.
    void mock
    await gotoCalendar(page)
    await page.getByTestId('add-event').click()
    await page.getByTestId('form-reminder-preset').selectOption('custom')
    await page.getByTestId('form-travel-preset').selectOption('custom')

    const unnamed = await page.evaluate(() =>
      [...document.querySelectorAll<HTMLSelectElement>('select')]
        .filter((e) => !e.getAttribute('aria-label') && !e.getAttribute('aria-labelledby') && !e.closest('label'))
        .map((e) => e.getAttribute('data-testid') ?? e.outerHTML.slice(0, 60)),
    )
    expect(unnamed).toEqual([])
  })

  test('a day cell says which day it is and what is on it', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)

    const today = dayCell(page, isoDay(0))
    const label = await today.getAttribute('aria-label')
    expect(label).toContain('today')
    // Two events are seeded on today.
    expect(label).toContain('2 events')
    expect(label).toMatch(/\d{4}/)
    await expect(today).toHaveAttribute('aria-current', 'date')

    // Only today is the current date.
    await expect(page.locator('[data-testid="day-cell"][aria-current="date"]')).toHaveCount(1)

    const quiet = dayCell(page, isoDay(5))
    const quietLabel = await quiet.getAttribute('aria-label')
    expect(quietLabel).not.toContain('event')
    expect(quietLabel).not.toContain('today')
  })

  test('a day cell reports whether it is the selected one', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    const today = dayCell(page, isoDay(0))
    await expect(today).toHaveAttribute('aria-pressed', 'false')
    await today.click()
    await expect(today).toHaveAttribute('aria-pressed', 'true')
    await dayCell(page, isoDay(1)).click()
    await expect(today).toHaveAttribute('aria-pressed', 'false')
  })

  test('a day with one event says "1 event", not "1 events"', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await expect(dayCell(page, isoDay(2))).toHaveAttribute('aria-label', /, 1 event$/)
  })

  test('errors are announced, not just displayed', async ({ mock, page }) => {
    mock.fail({ match: 'REPORT', status: 500 })
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('status-error')).toBeVisible()
    // role="alert" carries an implicit assertive live region, so inserting the
    // node is what announces it.
    await expect(page.getByTestId('status-error')).toHaveAttribute('role', 'alert')
  })

  test('a failed connection is announced too', async ({ mock, page }) => {
    mock.fail({ match: 'PROPFIND /', status: 401 })
    await page.goto('/')
    await waitForHydration(page)
    await page.getByTestId('server-url').fill(CREDS.server)
    await page.getByTestId('username').fill(CREDS.username)
    await page.getByTestId('password').fill(CREDS.password)
    await page.getByTestId('connect-submit').click()
    await expect(page.getByTestId('connect-error')).toHaveAttribute('role', 'alert')
  })

  test('the sync line is a polite live region', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    // Polite, not assertive: "Updated 14:32" should wait for a gap rather
    // than interrupt whatever is being read.
    await expect(page.getByTestId('sync-line')).toHaveAttribute('aria-live', 'polite')
  })

  test('the page has headings to navigate by, in order', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await dayCell(page, isoDay(0)).click()

    const levels = await page.evaluate(() =>
      [...document.querySelectorAll('h1,h2,h3,h4,h5,h6')].map((e) => ({
        level: Number(e.tagName[1]),
        text: (e.textContent ?? '').trim().slice(0, 30),
      })),
    )
    expect(levels.length).toBeGreaterThan(3)
    expect(levels[0].level).toBe(1)
    // No skipped levels -- h1 to h3 with nothing in between is a real failure
    // for anyone navigating by heading.
    for (let i = 1; i < levels.length; i++) {
      expect(levels[i].level - levels[i - 1].level, `jump at "${levels[i].text}"`).toBeLessThanOrEqual(1)
    }
    expect(levels.map((l) => l.text)).toContain('Calendars')
  })

  test('wrapping the month title in a heading did not move it', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    // Tailwind's preflight zeroes heading margins and inherits the size, so the
    // h2 is semantics only. Worth asserting, because a reset that stopped doing
    // that would push the whole header down.
    const h2 = page.locator('h2').filter({ has: page.getByTestId('month-title') })
    const box = await h2.boundingBox()
    const bar = await page.getByTestId('topbar').boundingBox()
    expect(box!.y).toBeLessThan(bar!.y + bar!.height + 40)
    expect(box!.height).toBeLessThanOrEqual(48)
  })

  test('the month disclosure points at the thing it opens', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    const id = await page.getByTestId('month-title').getAttribute('aria-controls')
    expect(id).toBeTruthy()
    // The target must exist even while closed, or the idref dangles.
    await expect(page.locator(`#${id}`)).toHaveCount(1)
    await page.getByTestId('month-title').click()
    await expect(page.locator(`#${id}`).getByTestId('month-select')).toBeVisible()
  })

  test('muted text meets WCAG AA on every surface it sits on', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    // #7a7a7a from DESIGN.md was 3.94:1 on the parchment background. DESIGN.md
    // scopes that token to disabled text and fine print, where contrast is
    // exempt; this app uses it for weekday initials, event times and the sync
    // line, which are information. Asserted on the computed value so a token
    // edit cannot quietly put it back.
    const muted = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="footer-timezone"]') ?? document.querySelector('.text-fine')
      return el ? getComputedStyle(el).color : null
    })
    expect(muted).toBe('rgb(110, 110, 115)')
  })
})

/** Violations as `rule: node, node` lines -- readable in a failure message. */
function summarise(violations: { id: string; impact?: string | null; nodes: { target: unknown[] }[] }[]): string[] {
  return violations.map(
    (v) => `[${v.impact}] ${v.id} x${v.nodes.length}: ${v.nodes.slice(0, 3).map((n) => n.target.join(' ')).join(', ')}`,
  )
}

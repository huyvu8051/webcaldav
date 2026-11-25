import AxeBuilder from '@axe-core/playwright'
import { test, expect, seedSession, waitForIdle, isoDay, dayCell, localDateTime } from '../fixtures/app'

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

/** Rules with their own tracked test below; excluded so the sweep stays sharp. */
const TRACKED = ['color-contrast', 'select-name']

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
    const r = await new AxeBuilder({ page }).withTags(WCAG).disableRules(TRACKED).analyze()
    expect(summarise(r.violations)).toEqual([])
  })

  for (const state of ['bare', 'day selected', 'form open', 'picker open'] as const) {
    test(`the calendar has no untracked violations: ${state}`, async ({ mock, page }) => {
      void mock
      await gotoCalendar(page)
      if (state !== 'bare') await dayCell(page, isoDay(0)).click()
      if (state === 'form open' || state === 'picker open') await page.getByTestId('add-event').click()
      if (state === 'picker open') await page.getByTestId('month-title').click()

      const r = await new AxeBuilder({ page }).withTags(WCAG).disableRules(TRACKED).analyze()
      expect(summarise(r.violations)).toEqual([])
    })
  }

  test('an error banner introduces no violations', async ({ mock, page }) => {
    mock.fail({ match: 'REPORT', status: 500 })
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('status-error')).toBeVisible()
    const r = await new AxeBuilder({ page }).withTags(WCAG).disableRules(TRACKED).analyze()
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

// ---------------------------------------------------------------------------
// Known gaps. Each is a test.fail asserting what should hold, paired with a
// passing test pinning what happens now.
// ---------------------------------------------------------------------------

test.describe('known accessibility gaps', () => {
  // KNOWN BUG, critical by axe's grading. The five selects in the event form
  // use their first <option> as a pseudo-label -- "Priority", "Status", "No
  // reminder", "No travel time", and the calendar list uses the first calendar
  // name. A screen reader announces the current value and nothing about what
  // the control is for. The month/year picker selects do have aria-label.
  test.fail('every select has an accessible name', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await page.getByTestId('add-event').click()
    const r = await new AxeBuilder({ page }).withTags(WCAG).analyze()
    expect(summarise(r.violations.filter((v) => v.id === 'select-name'))).toEqual([])
  })

  test('five selects currently have no accessible name', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await page.getByTestId('add-event').click()
    const r = await new AxeBuilder({ page }).withTags(WCAG).analyze()
    const v = r.violations.find((x) => x.id === 'select-name')
    expect(v?.nodes.length).toBe(5)
  })

  // KNOWN BUG. DESIGN.md's muted text token is #7a7a7a, and on the #f5f5f7
  // page background that is 3.94:1 -- WCAG AA wants 4.5:1 for text this size.
  // It is used for fine print, the weekday initials, the connected username,
  // the sync line and the event times, so it is roughly twenty nodes per page
  // rather than one stray element. Fixing it means moving the token, which is
  // a design decision, not a patch.
  test.fail('text meets WCAG AA contrast', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    const r = await new AxeBuilder({ page }).withTags(WCAG).analyze()
    expect(summarise(r.violations.filter((v) => v.id === 'color-contrast'))).toEqual([])
  })

  test('the muted text token currently fails contrast at 3.94:1', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    const r = await new AxeBuilder({ page }).withTags(WCAG).analyze()
    const v = r.violations.find((x) => x.id === 'color-contrast')
    expect(v, 'expected the known contrast failure').toBeTruthy()
    expect(v!.nodes.length).toBeGreaterThan(5)
    expect(JSON.stringify(v!.nodes[0])).toContain('#7a7a7a')
  })

  // KNOWN BUG. A day cell's entire accessible name is its number: "25,
  // button". No month, no year, no "today", no count of what is on it, and no
  // indication that it is the selected one -- the three things the design
  // communicates purely through a filled disc, a blue number and coloured
  // dots. This is the app's main interaction and it is the least legible part
  // of it without sight.
  test.fail('a day cell says which day it is and what is on it', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    const today = dayCell(page, isoDay(0))
    await expect(today).toHaveAttribute('aria-current', 'date')

    const name = await today.evaluate((e) => e.getAttribute('aria-label') ?? e.textContent?.trim() ?? '')
    expect(name).toMatch(/\d{1,2}/)
    expect(name.length, `day cell announces only "${name}"`).toBeGreaterThan(4)
  })

  test('a day cell currently announces only its number', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    const today = dayCell(page, isoDay(0))
    await expect(today).not.toHaveAttribute('aria-current', /.*/)
    expect((await today.textContent())?.trim()).toBe(String(new Date().getDate()))
  })

  // KNOWN BUG. There is no live region anywhere, so nothing the app does
  // asynchronously is announced: not "Updating…", not "Updated 14:32", and --
  // worst -- not the error banner. A save that fails is already silent to a
  // sighted user on some paths (BACKLOG.md #1); to a screen reader user it is
  // silent on all of them.
  test.fail('errors are announced, not just displayed', async ({ mock, page }) => {
    mock.fail({ match: 'REPORT', status: 500 })
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('status-error')).toBeVisible()

    const announced = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="status-error"]')
      if (!el) return false
      return !!el.closest('[aria-live], [role="alert"], [role="status"]')
    })
    expect(announced, 'the error banner is in no live region').toBe(true)
  })

  test('the page currently has no live regions at all', async ({ mock, page }) => {
    mock.fail({ match: 'REPORT', status: 500 })
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('status-error')).toBeVisible()
    const count = await page.locator('[aria-live], [role="alert"], [role="status"]').count()
    expect(count).toBe(0)
  })

  // KNOWN BUG, minor. The calendar page has exactly one heading: the "Calendar"
  // h1 in the top bar. The month name used to be an h2 and became a button
  // when the picker was added; "Calendars" and the selected date are spans.
  // There is nothing to navigate by heading, which is how screen reader users
  // skim a page.
  test.fail('the page has headings to navigate by', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await dayCell(page, isoDay(0)).click()
    const headings = await page.locator('h1, h2, h3, h4, h5, h6').count()
    expect(headings).toBeGreaterThan(1)
  })

  test('the calendar page currently has exactly one heading', async ({ mock, page }) => {
    void mock
    await gotoCalendar(page)
    await dayCell(page, isoDay(0)).click()
    await expect(page.locator('h1, h2, h3, h4, h5, h6')).toHaveCount(1)
  })
})

/** Violations as `rule: node, node` lines -- readable in a failure message. */
function summarise(violations: { id: string; impact?: string | null; nodes: { target: unknown[] }[] }[]): string[] {
  return violations.map(
    (v) => `[${v.impact}] ${v.id} x${v.nodes.length}: ${v.nodes.slice(0, 3).map((n) => n.target.join(' ')).join(', ')}`,
  )
}

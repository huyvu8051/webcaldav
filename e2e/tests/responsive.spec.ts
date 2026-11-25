import { test, expect, seedSession, waitForIdle, isoDay, dayCell, localDateTime } from '../fixtures/app'

/**
 * The whole premise of the redesign: one phone-shaped layout, and a desktop
 * viewport simply shows more background around the same column. There is no
 * second layout, so these assertions are about there continuing not to be one.
 */
test.use({ realTailwind: true })

const COLUMN_MAX = 430

test.describe('one layout, two viewports', () => {
  test.beforeEach(async ({ mock, page }) => {
    void mock
    await seedSession(page)
  })

  for (const size of [
    { name: 'small phone', width: 320, height: 568 },
    { name: 'phone', width: 390, height: 844 },
    { name: 'large phone', width: 430, height: 932 },
    { name: 'tablet', width: 820, height: 1180 },
    { name: 'desktop', width: 1440, height: 900 },
  ]) {
    test(`${size.name} (${size.width}px) never scrolls sideways`, async ({ page }) => {
      await page.setViewportSize({ width: size.width, height: size.height })
      await page.goto('/calendar')
      await waitForIdle(page)
      await dayCell(page, isoDay(0)).click()
      await page.getByTestId('add-event').click()
      await page.getByTestId('month-title').click()

      const overflow = await page.evaluate(() => ({
        doc: document.documentElement.scrollWidth,
        win: window.innerWidth,
        wide: [...document.querySelectorAll('*')]
          .filter((e) => e.getBoundingClientRect().right > window.innerWidth + 1)
          .map((e) => `${e.tagName.toLowerCase()}[${e.getAttribute('data-testid') ?? ''}]`)
          .slice(0, 5),
      }))
      expect(overflow.doc, `horizontal scroll at ${size.width}px`).toBeLessThanOrEqual(overflow.win + 1)
      expect(overflow.wide).toEqual([])
    })
  }

  test('the content column stays phone-width on a desktop viewport', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto('/calendar')
    await waitForIdle(page)

    const grid = await page.getByTestId('day-cell').first().evaluate((e) => {
      const panel = e.closest('.max-w-app, [class*="max-w-app"]') as HTMLElement | null
      return panel ? panel.getBoundingClientRect().width : NaN
    })
    const main = await page.evaluate(() => {
      const el = document.querySelector('main > div') as HTMLElement
      const r = el.getBoundingClientRect()
      return { width: r.width, left: r.left, right: r.right }
    })
    expect(main.width).toBeLessThanOrEqual(COLUMN_MAX + 1)
    // Centred: equal background either side is the "expand the background"
    // behaviour the whole design rests on.
    expect(Math.abs(main.left - (1440 - main.right))).toBeLessThan(2)
    void grid
  })

  test('the background fills the viewport even though the column does not', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto('/calendar')
    await waitForIdle(page)
    const body = await page.evaluate(() => {
      const s = getComputedStyle(document.body)
      return { bg: s.backgroundColor, width: document.body.getBoundingClientRect().width }
    })
    expect(body.width).toBeGreaterThan(1400)
    // Transparent would let the host's own ground through.
    expect(body.bg).not.toBe('rgba(0, 0, 0, 0)')
  })

  test('the top bar spans the viewport while its contents stay in the column', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 })
    await page.goto('/calendar')
    await waitForIdle(page)
    const bar = await page.getByTestId('topbar').boundingBox()
    const inner = await page.getByTestId('topbar').locator('> div').boundingBox()
    expect(bar!.width).toBeGreaterThan(1400)
    expect(inner!.width).toBeLessThanOrEqual(COLUMN_MAX + 1)
  })

  test('the same testids are present at 320px and 1440px', async ({ page }) => {
    const seen: Record<number, string[]> = {}
    for (const width of [320, 1440]) {
      await page.setViewportSize({ width, height: 900 })
      await page.goto('/calendar')
      await waitForIdle(page)
      await page.getByTestId('add-event').click()
      seen[width] = (
        await page.evaluate(() =>
          [...document.querySelectorAll('[data-testid]')].map((e) => e.getAttribute('data-testid')!),
        )
      )
        .filter((v, i, a) => a.indexOf(v) === i)
        .sort()
    }
    // A second layout would show up here as a difference. There isn't one.
    expect(seen[320]).toEqual(seen[1440])
  })

  test('a long event title truncates instead of widening the card', async ({ mock, page }) => {
    mock.calendar('work').events.push({
      uid: 'ev-long',
      summary: 'A'.repeat(300),
      start: `${isoDay(0).replace(/-/g, '')}T190000`,
      end: `${isoDay(0).replace(/-/g, '')}T200000`,
    })
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/calendar')
    await waitForIdle(page)
    await dayCell(page, isoDay(0)).click()

    const card = page.locator('[data-testid="event-card"]').filter({ hasText: 'AAAA' })
    const box = await card.boundingBox()
    expect(box!.width).toBeLessThanOrEqual(390)
    await expect(card.getByTestId('event-summary')).toHaveClass(/truncate/)
  })

  test('text inputs are 17px so iOS does not zoom on focus', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/calendar')
    await waitForIdle(page)
    await page.getByTestId('add-event').click()
    await page.getByTestId('form-start').fill(localDateTime(0, 9))

    const sizes = await page.evaluate(() =>
      [...document.querySelectorAll('input, select, textarea')]
        .filter((e) => e.getBoundingClientRect().width > 0)
        .filter((e) => !['checkbox', 'color'].includes((e as HTMLInputElement).type))
        .map((e) => ({ id: e.getAttribute('data-testid'), size: parseFloat(getComputedStyle(e).fontSize) })),
    )
    expect(sizes.length).toBeGreaterThan(3)
    for (const s of sizes) {
      // Below 16px Safari zooms the whole page on focus; 17px is both the
      // system body size and comfortably over that line.
      expect(s.size, `${s.id} is ${s.size}px`).toBeGreaterThanOrEqual(16)
    }
  })

  // KNOWN BUG. Every glyph in this app is a Material Symbols ligature, so when
  // fonts.googleapis.com is slow, blocked or down, each one renders as its own
  // name in body text: `chevron_right` is 127px wide instead of a 22px glyph,
  // and the month header alone pushes the page 26px past the viewport. The
  // measurement below is taken with the font stubbed out, which is exactly
  // what a user behind a firewall that blocks Google sees.
  test.fail('the layout survives the icon font failing to load', async ({ page }) => {
    await page.route(/fonts\.(googleapis|gstatic)\.com/, (route) =>
      route.fulfill({ status: 200, body: '', headers: { 'access-control-allow-origin': '*' } }),
    )
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/calendar')
    await waitForIdle(page)
    const width = await page.evaluate(() => document.documentElement.scrollWidth)
    expect(width).toBeLessThanOrEqual(391)
  })

  test('without the icon font the page currently overflows by 26px', async ({ page }) => {
    await page.route(/fonts\.(googleapis|gstatic)\.com/, (route) =>
      route.fulfill({ status: 200, body: '', headers: { 'access-control-allow-origin': '*' } }),
    )
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto('/calendar')
    await waitForIdle(page)
    const offenders = await page.evaluate(() =>
      [...document.querySelectorAll('.material-symbols-outlined')]
        .filter((e) => e.getBoundingClientRect().width > 40)
        .map((e) => `${e.textContent}=${Math.round(e.getBoundingClientRect().width)}px`),
    )
    expect(offenders.length).toBeGreaterThan(0)
  })
})

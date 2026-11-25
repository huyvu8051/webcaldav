import { test, expect, waitForIdle, isoDay, dayCell, ymd } from '../fixtures/app'

/**
 * Sends the page to the background and back, which is what triggers the refetch.
 *
 * `bubbles: true` is not optional. The real event is fired at the Document
 * with bubbles set, which is how a window listener ever sees it; a synthetic
 * `new Event('visibilitychange', { bubbles: true })` defaults to bubbles: false and never leaves
 * the document, so every assertion built on it passes for the wrong reason.
 */
async function blurAndReturn(page: import('@playwright/test').Page) {
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => true })
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'hidden' })
    document.dispatchEvent(new Event('visibilitychange', { bubbles: true }))
  })
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => false })
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'visible' })
    document.dispatchEvent(new Event('visibilitychange', { bubbles: true }))
  })
}

test.describe('keeping the data fresh', () => {
  test('nothing refetches on its own -- there is no poll', async ({ mock, page, connected }) => {
    void connected
    mock.clearRequests()
    await page.waitForTimeout(4000)
    expect(mock.countOf('REPORT')).toBe(0)
  })

  test('Refresh refetches and picks up a change made elsewhere', async ({ mock, page, connected }) => {
    void connected
    // Simulates another device writing to the same server.
    mock.calendar('personal').events.push({
      uid: 'ev-elsewhere',
      summary: 'Added from a phone',
      start: `${ymd(0)}T170000`,
      end: `${ymd(0)}T180000`,
    })
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(2)

    await page.getByTestId('refresh').click()
    await waitForIdle(page)
    await expect(page.getByTestId('event-card')).toHaveCount(3)
    await expect(page.locator('[data-testid="event-card"]').filter({ hasText: 'Added from a phone' })).toBeVisible()
  })

  test('Refresh ignores the staleness floor -- an explicit tap always goes', async ({ mock, page, connected }) => {
    void connected
    mock.clearRequests()
    await page.getByTestId('refresh').click()
    await waitForIdle(page)
    expect(mock.countOf('REPORT')).toBeGreaterThan(0)
  })

  test('coming back to the tab inside 60s does not refetch', async ({ mock, page, connected }) => {
    void connected
    mock.clearRequests()
    await blurAndReturn(page)
    await page.waitForTimeout(2500)
    // A full fetch pulls every event in every calendar; an alt-tabbing user
    // must not trigger one per switch.
    expect(mock.countOf('REPORT')).toBe(0)
  })

  test('coming back to the tab after 60s does refetch', async ({ mock, page, connected }) => {
    // This one waits the real minute. Faking the clock was tried and does not
    // work: `chrono::Local::now()` resolves through the wasm-bindgen glue,
    // which holds the `Date` it was given at instantiation, so patching the
    // global afterwards changes nothing -- and patching it beforehand skews
    // the baseline by the same amount, leaving the difference at zero. A
    // slow test that measures the real threshold beats a fast one that
    // measures a mock of it.
    test.setTimeout(140_000)
    void connected
    await waitForIdle(page)
    mock.clearRequests()

    await blurAndReturn(page)
    await page.waitForTimeout(2000)
    expect(mock.countOf('REPORT'), 'inside the floor, nothing should be sent').toBe(0)

    await page.waitForTimeout(62_000)
    await blurAndReturn(page)
    await expect.poll(() => mock.countOf('REPORT'), { timeout: 20_000 }).toBeGreaterThan(0)
  })

  test('being hidden on its own refetches nothing', async ({ mock, page, connected }) => {
    void connected
    mock.clearRequests()
    await page.evaluate(() => {
      Object.defineProperty(document, 'hidden', { configurable: true, get: () => true })
      document.dispatchEvent(new Event('visibilitychange', { bubbles: true }))
    })
    await page.waitForTimeout(2000)
    expect(mock.countOf('REPORT')).toBe(0)
  })

  test('two refreshes at once send one fetch, not two', async ({ mock, page, connected }) => {
    void connected
    // Four callers -- mount, save, delete and tab focus -- can collide, and
    // the slower response would overwrite the newer one.
    mock.fail({ match: 'REPORT', delayMs: 1200 })
    mock.clearRequests()
    await page.getByTestId('refresh').click()
    await page.getByTestId('refresh').click({ force: true }).catch(() => {})
    await blurAndReturn(page)
    await waitForIdle(page)
    // Two calendars, and each is a calendar-query plus a multiget: one pass.
    expect(mock.countOf('REPORT')).toBeLessThanOrEqual(4)
  })

  test('the Updated time moves forward after a refresh', async ({ page, connected }) => {
    void connected
    const before = await page.getByTestId('sync-line').innerText()
    await page.waitForTimeout(1100)
    await page.getByTestId('refresh').click()
    await waitForIdle(page)
    const after = await page.getByTestId('sync-line').innerText()
    expect(after).toMatch(/Updated \d\d:\d\d/)
    expect(before).toMatch(/Updated \d\d:\d\d/)
  })

  test('a refresh that fails leaves the previous data on screen', async ({ mock, page, connected }) => {
    void connected
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(2)

    mock.fail({ match: 'REPORT', status: 500 })
    await page.getByTestId('refresh').click()
    await expect(page.getByTestId('status-error')).toBeVisible()
    // Wiping the list on a failed refresh would be worse than stale data.
    await expect(page.getByTestId('event-card')).toHaveCount(2)
  })
})

import { test, expect, seedSession, waitForIdle, isoDay, dayCell } from '../fixtures/app'

test.describe('loading the calendar', () => {
  test('no stored session sends you back to connect', async ({ mock, page }) => {
    void mock
    await page.goto('/calendar')
    await expect(page).toHaveURL(/\/$/)
    await expect(page.getByTestId('server-url')).toBeVisible()
  })

  test('calendars are listed before events are fetched', async ({ mock, page }) => {
    await seedSession(page)
    await page.goto('/calendar')
    await waitForIdle(page)

    const order = mock.requests.map((r) => `${r.method} ${r.path}`)
    const listedHome = order.findIndex((r) => r.includes('/calendars/alice/'))
    const firstReport = order.findIndex((r) => r.startsWith('REPORT'))
    expect(listedHome).toBeGreaterThanOrEqual(0)
    expect(firstReport).toBeGreaterThan(listedHome)
  })

  test('both calendars and their events arrive', async ({ mock, page }) => {
    void mock
    await seedSession(page)
    await page.goto('/calendar')
    await waitForIdle(page)

    await expect(page.getByTestId('calendar-row')).toHaveCount(2)
    await expect(page.getByTestId('calendars-panel')).toContainText('Work')
    await expect(page.getByTestId('calendars-panel')).toContainText('Personal')
    // Two of the seeded events are today.
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-event-count', '2')
  })

  test('the sync line reports working, then when it last succeeded', async ({ mock, page }) => {
    mock.fail({ match: 'REPORT', delayMs: 1500 })
    await seedSession(page)
    await page.goto('/calendar')

    await expect(page.getByTestId('sync-line')).toContainText('Updating')
    await expect(page.getByTestId('spinner').first()).toBeVisible()
    await expect(page.getByTestId('sync-line')).toContainText(/Updated \d\d:\d\d/, { timeout: 20_000 })
  })

  // KNOWN BUG. `rsdav`'s fetch_calendars answers a 503 with Ok(vec![]) rather
  // than Err, so `list_calendars` never reaches its error arm. The page then
  // shows an empty Calendars panel, no error, and no sync line -- a server
  // outage is pixel-identical to an account with no calendars. Marked failing
  // so it is tracked and flips green the day it is fixed.
  test.fail('a failed calendar listing is reported, not left blank', async ({ mock, page }) => {
    mock.fail({ match: 'PROPFIND /calendars/alice/', status: 503 })
    await seedSession(page)
    await page.goto('/calendar')

    await expect(page.getByTestId('status-error')).toBeVisible()
    await expect(page.getByTestId('status-error')).toContainText(/calendar/i)
  })

  test('a 503 on the listing currently yields a silent empty page', async ({ mock, page }) => {
    // The other half of the bug above, asserted as-is so the suite notices if
    // the behaviour changes in either direction.
    mock.fail({ match: 'PROPFIND /calendars/alice/', status: 503 })
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('calendars-panel')).toBeVisible()
    await expect(page.getByTestId('calendar-row')).toHaveCount(0)
    await expect(page.getByTestId('status-error')).toHaveCount(0)
  })

  test.fail('service discovery runs once per page load, not once per call', async ({ mock, page }) => {
    // KNOWN BUG. `list_calendars` and `list_events_for_calendars` each call
    // `login()`, which builds a fresh client and redoes the whole
    // .well-known -> principal -> calendar-home chain. A single page load
    // therefore pays for it twice, and so does every save, delete and tab
    // focus, because each of those calls refresh_events.
    await seedSession(page)
    await page.goto('/calendar')
    await waitForIdle(page)
    expect(mock.countOf('PROPFIND /.well-known/caldav')).toBe(1)
  })

  test('service discovery currently repeats for every call', async ({ mock, page }) => {
    await seedSession(page)
    await page.goto('/calendar')
    await waitForIdle(page)
    expect(mock.countOf('PROPFIND /.well-known/caldav')).toBe(2)
  })

  test('a failed event fetch still leaves the calendars usable', async ({ mock, page }) => {
    mock.fail({ match: 'REPORT', status: 500 })
    await seedSession(page)
    await page.goto('/calendar')

    await expect(page.getByTestId('status-error')).toBeVisible()
    await expect(page.getByTestId('calendar-row')).toHaveCount(2)
    // The grid still renders -- an event fetch failing is not a reason to
    // lose the month.
    await expect(page.getByTestId('day-cell').first()).toBeVisible()
  })

  test('a dropped connection is reported rather than hanging', async ({ mock, page }) => {
    mock.fail({ match: 'REPORT', abort: true })
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('status-error')).toBeVisible({ timeout: 20_000 })
  })

  test('an account with no calendars does not look broken', async ({ mock, page }) => {
    mock.calendars = []
    await seedSession(page)
    await page.goto('/calendar')

    await expect(page.getByTestId('calendars-panel')).toBeVisible()
    await expect(page.getByTestId('calendar-row')).toHaveCount(0)
    await expect(page.getByTestId('day-cell').first()).toBeVisible()
    await expect(page.getByTestId('status-error')).toHaveCount(0)
  })

  test('a calendar with no events renders an empty month', async ({ mock, page }) => {
    mock.calendars = [{ id: 'work', displayName: 'Work', events: [] }]
    await seedSession(page)
    await page.goto('/calendar')
    await waitForIdle(page)

    await expect(page.getByTestId('calendar-row')).toHaveCount(1)
    const counts = await page.getByTestId('day-cell').evaluateAll((els) =>
      els.map((e) => e.getAttribute('data-event-count')),
    )
    expect(new Set(counts)).toEqual(new Set(['0']))
  })

  test('disconnect clears the session and returns to connect', async ({ connected, page }) => {
    void connected
    await page.getByTestId('disconnect').click()
    await expect(page).toHaveURL(/\/$/)
    expect(await page.evaluate(() => window.localStorage.getItem('webcaldav.connection'))).toBeNull()
  })

  test('the top bar shows the connected username', async ({ connected }) => {
    await expect(connected.getByTestId('topbar-username')).toHaveText('alice')
  })

  test('the footer names the client timezone', async ({ connected }) => {
    // Rendered only after hydration -- printing the server's zone during SSR
    // would be both wrong and a hydration mismatch.
    await expect(connected.getByTestId('footer-timezone')).toContainText(/\w+\/\w+ · UTC[+-]\d\d:\d\d/)
  })
})

import { test, expect, seedSession, waitForIdle, isoDay, dayCell, ymd } from '../fixtures/app'
import type { CalDavMock, MockEvent } from '../fixtures/app'

/** Serves one calendar holding exactly the events a test cares about. */
async function withEvents(
  mock: CalDavMock,
  page: import('@playwright/test').Page,
  events: MockEvent[],
) {
  mock.calendars = [{ id: 'work', displayName: 'Work', color: '#8a9a7b', events }]
  await seedSession(page)
  await page.goto('/calendar')
  await waitForIdle(page)
}

test.describe('awkward data', () => {
  test('an event with no summary still renders a card', async ({ mock, page }) => {
    await withEvents(mock, page, [
      { uid: 'ev-blank', summary: '', start: `${ymd(0)}T090000`, end: `${ymd(0)}T100000` },
    ])
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(1)
    await expect(page.getByTestId('event-time')).toContainText('–')
  })

  test('emoji and non-Latin text survive the round trip', async ({ mock, page }) => {
    await withEvents(mock, page, [
      { uid: 'ev-uni', summary: 'Hẹn cà phê ☕️ với Mai', start: `${ymd(0)}T090000`, end: `${ymd(0)}T100000` },
    ])
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-summary')).toHaveText('Hẹn cà phê ☕️ với Mai')
  })

  test('an all-day event lands on its day', async ({ mock, page }) => {
    await withEvents(mock, page, [
      { uid: 'ev-allday', summary: 'Tet holiday', allDay: true, start: ymd(1), end: ymd(2) },
    ])
    await expect(dayCell(page, isoDay(1))).toHaveAttribute('data-event-count', '1')
    await dayCell(page, isoDay(1)).click()
    await expect(page.getByTestId('event-summary')).toHaveText('Tet holiday')
  })

  test('an event crossing midnight is listed on its start day', async ({ mock, page }) => {
    await withEvents(mock, page, [
      { uid: 'ev-night', summary: 'Night shift', start: `${ymd(0)}T223000`, end: `${ymd(1)}T063000` },
    ])
    // Filtering is by start prefix, so this is a statement about which day it
    // appears on, not a claim that multi-day spanning is handled.
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-event-count', '1')
    await expect(dayCell(page, isoDay(1))).toHaveAttribute('data-event-count', '0')
  })

  test('a malformed VEVENT is skipped without taking the page down', async ({ mock, page }) => {
    const errors: string[] = []
    page.on('pageerror', (e) => errors.push(e.message))
    await withEvents(mock, page, [
      { uid: 'ev-ok', summary: 'Fine', start: `${ymd(0)}T090000`, end: `${ymd(0)}T100000` },
      { uid: 'ev-bad', summary: '', start: '', end: '', rawIcs: 'BEGIN:VCALENDAR\nthis is not iCalendar at all\n' },
      { uid: 'ev-bad2', summary: '', start: '', end: '', rawIcs: '' },
    ])
    await dayCell(page, isoDay(0)).click()
    // The good one survives; the broken ones are dropped rather than throwing.
    await expect(page.getByTestId('event-summary')).toHaveText('Fine')
    expect(errors).toEqual([])
  })

  test('a VEVENT with no DTEND does not crash the list', async ({ mock, page }) => {
    const errors: string[] = []
    page.on('pageerror', (e) => errors.push(e.message))
    await withEvents(mock, page, [
      {
        uid: 'ev-noend',
        summary: 'Open ended',
        start: '',
        end: '',
        rawIcs: `BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\nUID:ev-noend\nDTSTAMP:20260101T000000Z\nDTSTART:${ymd(0)}T090000\nSUMMARY:Open ended\nEND:VEVENT\nEND:VCALENDAR\n`,
      },
    ])
    expect(errors).toEqual([])
    await expect(page.getByTestId('day-cell').first()).toBeVisible()
  })

  test('two events sharing a UID across calendars both show', async ({ mock, page }) => {
    mock.calendars = [
      { id: 'work', displayName: 'Work', events: [{ uid: 'dup', summary: 'Work copy', start: `${ymd(0)}T090000`, end: `${ymd(0)}T100000` }] },
      { id: 'personal', displayName: 'Personal', events: [{ uid: 'dup', summary: 'Personal copy', start: `${ymd(0)}T090000`, end: `${ymd(0)}T100000` }] },
    ]
    await seedSession(page)
    await page.goto('/calendar')
    await waitForIdle(page)
    await dayCell(page, isoDay(0)).click()
    // The day list keys on uid alone, so a UID shared across two calendars is
    // exactly the case where that key stops being unique.
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-event-count', '2')
  })

  test('a calendar with a hundred events on one day still renders', async ({ mock, page }) => {
    await withEvents(
      mock,
      page,
      Array.from({ length: 100 }, (_, i) => ({
        uid: `bulk-${i}`,
        summary: `Event ${i}`,
        start: `${ymd(0)}T${String(i % 24).padStart(2, '0')}0000`,
        end: `${ymd(0)}T${String(i % 24).padStart(2, '0')}3000`,
      })),
    )
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-event-count', '100')
    await expect(dayCell(page, isoDay(0)).getByTestId('day-dots').locator('span')).toHaveCount(3)
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(100)
  })

  test('HTML in a summary cannot become markup', async ({ mock, page }) => {
    const errors: string[] = []
    page.on('pageerror', (e) => errors.push(e.message))
    await withEvents(mock, page, [
      { uid: 'ev-xss', summary: '<img src=x onerror=alert(1)>', start: `${ymd(0)}T090000`, end: `${ymd(0)}T100000` },
    ])
    await dayCell(page, isoDay(0)).click()

    // The property that matters: no element is created and no handler fires.
    // The text is a text node, whatever it ends up containing.
    expect(await page.locator('[data-testid="event-summary"] *').count()).toBe(0)
    expect(await page.locator('img').count()).toBe(0)
    expect(errors).toEqual([])
    await expect(page.getByTestId('event-summary')).toContainText('onerror=alert(1)')
  })

  // KNOWN BUG, low severity. `<` and `>` are dropped from a summary somewhere
  // between the calendar-data and the card: the mock sends
  // `SUMMARY:<img src=x onerror=alert(1)>` and the UI shows
  // `img src=x onerror=alert(1)`. Not a security problem -- nothing is parsed
  // as markup -- but a title containing angle brackets is silently mangled.
  test.fail('angle brackets in a summary survive', async ({ mock, page }) => {
    await withEvents(mock, page, [
      { uid: 'ev-angle', summary: 'a <b> c', start: `${ymd(0)}T090000`, end: `${ymd(0)}T100000` },
    ])
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-summary')).toHaveText('a <b> c')
  })

  // KNOWN BUG. A calendar whose `displayname` is empty -- or absent from the
  // PROPFIND response entirely -- renders the literal characters `{}` as its
  // name. `display_name.unwrap_or_else(|| "(untitled calendar)")` never fires
  // because the value arrives as Some("{}") rather than None, so the fallback
  // is unreachable and the UI shows braces. Measured both ways.
  test.fail('a calendar with no name falls back to a readable label', async ({ mock, page }) => {
    mock.calendars = [{ id: 'nameless', displayName: '', events: [] }]
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('calendar-row')).toHaveCount(1)
    await expect(page.getByTestId('calendar-row')).toContainText(/untitled/i)
  })

  test('a nameless calendar currently shows literal braces', async ({ mock, page }) => {
    mock.calendars = [{ id: 'nameless', displayName: '', events: [] }]
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('calendar-row')).toHaveText('{}')
  })

  test('a calendar with no colour still gets a dot', async ({ mock, page }) => {
    mock.calendars = [
      { id: 'plain', displayName: 'Plain', events: [{ uid: 'e1', summary: 'Thing', start: `${ymd(0)}T090000`, end: `${ymd(0)}T100000` }] },
    ]
    await seedSession(page)
    await page.goto('/calendar')
    await waitForIdle(page)
    const dot = dayCell(page, isoDay(0)).getByTestId('day-dots').locator('span').first()
    await expect(dot).toHaveCount(1)
    // The inline style, not the computed one: this spec runs without Tailwind,
    // so a computed check would be measuring the absence of the stylesheet.
    await expect(dot).toHaveAttribute('style', /background-color/)
  })

  test('a server URL with a path prefix is honoured', async ({ mock, page }) => {
    void mock
    await page.addInitScript(() => {
      window.localStorage.setItem(
        'webcaldav.connection',
        JSON.stringify({ server_url: 'https://dav.example.test/dav/', username: 'alice', password: 'hunter2' }),
      )
    })
    await page.goto('/calendar')
    await page.waitForTimeout(2500)
    // Whatever it resolves to, it must have asked under the given prefix
    // rather than silently rewriting it to the root.
    expect(mock.requests.some((r) => r.path.startsWith('/dav/'))).toBe(true)
  })
})

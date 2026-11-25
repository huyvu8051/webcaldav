import { test, expect, seedSession, waitForIdle, dayCell } from '../fixtures/app'
import type { Page } from '@playwright/test'

/**
 * Create, edit and delete across the calendar rather than on today.
 *
 * The rest of the suite exercises CRUD almost entirely on `isoDay(0)`, which
 * means every date assertion in it would still pass if the app put events on
 * the wrong day in some *other* month. The cases that actually break date
 * handling are the ones at the edges: the 1st, the 31st, 29 February, the
 * turn of the year, and a day that is visible in the grid but belongs to the
 * neighbouring month.
 *
 * The randomised half picks fresh days on every run and prints its seed, so a
 * failure can be replayed with `SEED=<n> npm run e2e`.
 */

// Derived from the date, not Math.random: Playwright matches tests by title
// across processes, so a seed that differs between the collection pass and the
// worker makes the test "not found". Varies day to day, stable within a run,
// and overridable for a replay.
const SEED = Number(process.env.SEED ?? 0) || Number(new Date().toISOString().slice(0, 10).replace(/-/g, ''))

/** mulberry32 -- small, and the same seed gives the same days every time. */
function rng(seed: number) {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

const MONTHS = ['January','February','March','April','May','June','July','August','September','October','November','December']

function iso(y: number, m: number, d: number): string {
  return `${y}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`
}

function daysInMonth(y: number, m: number): number {
  return new Date(y, m, 0).getDate()
}

/** What the header currently shows, with the chevron glyph stripped off. */
async function currentMonth(page: Page): Promise<{ y: number; m: number }> {
  const title = (await page.getByTestId('month-title').innerText()).trim()
  // innerText is "November 2026expand_more" -- the glyph is a ligature, so
  // splitting on whitespace alone gives a year of "2026expand_more", which
  // parses as NaN and silently makes every comparison false.
  const found = title.match(/^([A-Za-z]+)\s+(\d{4})/)
  if (!found) throw new Error(`unreadable month header: ${JSON.stringify(title)}`)
  return { y: Number(found[2]), m: MONTHS.indexOf(found[1]) + 1 }
}

/** Jumps to a month, by picker for a long hop and chevrons for a short one. */
async function goToMonth(page: Page, y: number, m: number) {
  const at = await currentMonth(page)
  const distance = (y - at.y) * 12 + (m - at.m)
  if (distance === 0) return

  if (Math.abs(distance) > 2) {
    await page.getByTestId('month-title').click()
    await page.getByTestId('year-select').selectOption(String(y))
    await page.getByTestId('month-select').selectOption(String(m))
    await page.getByTestId('month-title').click()
  } else {
    const step = distance > 0 ? 'month-next' : 'month-prev'
    for (let i = 0; i < Math.abs(distance); i++) await page.getByTestId(step).click()
  }

  const now = await currentMonth(page)
  expect({ y: now.y, m: now.m }, `failed to reach ${MONTHS[m - 1]} ${y}`).toEqual({ y, m })
}

/** Types the date straight into the form, whatever month is on screen. */
async function createEvent(page: Page, isoDate: string, summary: string, hour = 9) {
  await page.getByTestId('add-event').click()
  await page.getByTestId('form-summary').fill(summary)
  await page.getByTestId('form-start').fill(`${isoDate}T${String(hour).padStart(2, '0')}:00`)
  await page.getByTestId('form-end').fill(`${isoDate}T${String(hour + 1).padStart(2, '0')}:00`)
  await page.getByTestId('form-submit').click()
  await expect(page.getByTestId('event-form')).toHaveCount(0)
  await waitForIdle(page)
}

/** The event is on this day and on no other day in the visible grid. */
async function expectOnlyOn(page: Page, isoDate: string, summary: string) {
  const [y, m] = isoDate.split('-').map(Number)
  await goToMonth(page, y, m)

  await expect(dayCell(page, isoDate)).toHaveAttribute('data-event-count', '1')
  await dayCell(page, isoDate).click()
  await expect(page.locator('[data-testid="event-card"]').filter({ hasText: summary })).toHaveCount(1)

  // Every other cell in the grid is empty. This is the assertion that catches
  // an event landing a day either side of where it was asked to go.
  const stray = await page.getByTestId('day-cell').evaluateAll((els, target) =>
    els
      .filter((e) => e.getAttribute('data-date') !== target)
      .filter((e) => e.getAttribute('data-event-count') !== '0')
      .map((e) => `${e.getAttribute('data-date')}=${e.getAttribute('data-event-count')}`),
    isoDate,
  )
  expect(stray, `event leaked onto other days`).toEqual([])
}

test.describe('events away from today', () => {
  test.beforeEach(async ({ mock, page }) => {
    // One empty calendar, so every count in this file is unambiguous.
    mock.calendars = [{ id: 'work', displayName: 'Work', color: '#8a9a7b', events: [] }]
    await seedSession(page)
    await page.goto('/calendar')
    await waitForIdle(page)
  })

  const now = new Date()
  const Y = now.getFullYear()
  const M = now.getMonth() + 1

  const EDGES = [
    { name: 'the 1st of this month', date: iso(Y, M, 1) },
    { name: 'the last day of this month', date: iso(Y, M, daysInMonth(Y, M)) },
    { name: 'a day in the previous month', date: iso(M === 1 ? Y - 1 : Y, M === 1 ? 12 : M - 1, 15) },
    { name: 'a day in the next month', date: iso(M === 12 ? Y + 1 : Y, M === 12 ? 1 : M + 1, 15) },
    { name: 'a 31-day month at the 31st', date: iso(Y, 1, 31) },
    { name: '28 February', date: iso(Y, 2, 28) },
    { name: '29 February in a leap year', date: iso(2028, 2, 29) },
    { name: "new year's eve", date: iso(Y, 12, 31) },
    { name: "new year's day", date: iso(Y + 1, 1, 1) },
  ]

  for (const edge of EDGES) {
    test(`a full lifecycle on ${edge.name} (${edge.date})`, async ({ mock, page }) => {
      const summary = `Edge ${edge.date}`
      await createEvent(page, edge.date, summary)

      // Created where it was asked to go, and nowhere else.
      await expectOnlyOn(page, edge.date, summary)
      expect(mock.calendar('work').events).toHaveLength(1)
      expect(mock.matching('PUT')).toHaveLength(1)

      // Still there after a reload -- the server holds it, not the page.
      await page.reload()
      await waitForIdle(page)
      await expectOnlyOn(page, edge.date, summary)

      // And it can be removed again.
      await dayCell(page, edge.date).click()
      await page.getByTestId('event-delete').click()
      await waitForIdle(page)
      await expect(dayCell(page, edge.date)).toHaveAttribute('data-event-count', '0')
      expect(mock.calendar('work').events).toHaveLength(0)
    })
  }

  test('an event moved to another month leaves the old day and arrives on the new', async ({ mock, page }) => {
    const from = iso(Y, M, Math.min(14, daysInMonth(Y, M)))
    const toMonth = M === 12 ? 1 : M + 1
    const toYear = M === 12 ? Y + 1 : Y
    const to = iso(toYear, toMonth, 3)

    await createEvent(page, from, 'Moving day')
    await expectOnlyOn(page, from, 'Moving day')

    await goToMonth(page, Y, M)
    await dayCell(page, from).click()
    await page.getByTestId('event-edit').click()
    await page.getByTestId('form-start').fill(`${to}T15:00`)
    await page.getByTestId('form-end').fill(`${to}T16:00`)
    await page.getByTestId('form-submit').click()
    await expect(page.getByTestId('event-form')).toHaveCount(0)
    await waitForIdle(page)

    // One object on the server, not two.
    expect(mock.calendar('work').events).toHaveLength(1)
    await goToMonth(page, Y, M)
    await expect(dayCell(page, from)).toHaveAttribute('data-event-count', '0')
    await expectOnlyOn(page, to, 'Moving day')
  })

  test('a day that belongs to the neighbouring month still works', async ({ page }) => {
    // The grid always shows 42 cells, so the first and last rows are someone
    // else's days -- and they are tappable.
    await goToMonth(page, Y, M)
    const outside = await page
      .locator('[data-testid="day-cell"][data-in-month="false"]')
      .first()
      .getAttribute('data-date')

    await createEvent(page, outside!, 'Spillover')
    await goToMonth(page, Y, M)
    // Visible from this month's grid even though it is not this month's day.
    await expect(dayCell(page, outside!)).toHaveAttribute('data-event-count', '1')
    await dayCell(page, outside!).click()
    await expect(page.getByTestId('event-summary')).toHaveText('Spillover')
  })

  test('two events on the same far-off day both survive a delete of one', async ({ mock, page }) => {
    const target = iso(M === 12 ? Y + 1 : Y, M === 12 ? 1 : M + 1, 20)
    await createEvent(page, target, 'First', 9)
    await createEvent(page, target, 'Second', 14)

    const [ty, tm] = target.split('-').map(Number)
    await goToMonth(page, ty, tm)
    await expect(dayCell(page, target)).toHaveAttribute('data-event-count', '2')

    await dayCell(page, target).click()
    await page.locator('[data-testid="event-card"]').filter({ hasText: 'First' }).getByTestId('event-delete').click()
    await waitForIdle(page)

    await expect(dayCell(page, target)).toHaveAttribute('data-event-count', '1')
    await expect(page.getByTestId('event-summary')).toHaveText('Second')
    expect(mock.calendar('work').events.map((e) => e.summary)).toEqual(['Second'])
  })

  test('randomised lifecycles across a year', async ({ mock, page }) => {
    test.setTimeout(180_000)
    // Printed, not in the title -- replay a failure with SEED=<n> npm run e2e.
    console.log(`seed ${SEED}`)
    const rand = rng(SEED)
    const picked: string[] = []

    for (let i = 0; i < 6; i++) {
      // Anywhere from six months back to six months forward.
      const offset = Math.floor(rand() * 13) - 6
      const d = new Date(Y, M - 1 + offset, 1)
      const y = d.getFullYear()
      const m = d.getMonth() + 1
      const day = 1 + Math.floor(rand() * daysInMonth(y, m))
      const date = iso(y, m, day)
      const hour = 1 + Math.floor(rand() * 20)
      picked.push(date)

      const summary = `Random ${i} ${date}`
      await createEvent(page, date, summary, hour)
      await expectOnlyOn(page, date, summary)
      expect(mock.calendar('work').events, `after creating ${date}`).toHaveLength(1)

      await dayCell(page, date).click()
      await page.getByTestId('event-delete').click()
      await waitForIdle(page)
      await expect(dayCell(page, date)).toHaveAttribute('data-event-count', '0')
      expect(mock.calendar('work').events, `after deleting ${date}`).toHaveLength(0)
    }

    console.log(`seed ${SEED} covered: ${picked.join(', ')}`)
  })
})

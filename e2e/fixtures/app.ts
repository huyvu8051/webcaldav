import { test as base, expect, type Page } from '@playwright/test'
import { CalDavMock, CREDS, seedSession, type MockCalendar } from './caldav-mock'

/**
 * Test fixtures.
 *
 * `mock` is installed before every test, so a test only has to describe the
 * data or the fault it cares about. `connected` goes one step further and
 * starts on a loaded /calendar, because most tests are about what happens
 * after that point and re-typing the connect form in each of them would test
 * the connect form fifty times and everything else once.
 */

export { expect, CREDS }
export * from './caldav-mock'

interface Fixtures {
  /** Load the real Tailwind build instead of the shim. Set with `test.use`. */
  realTailwind: boolean
  mock: CalDavMock
  /** A /calendar page with calendars and events already loaded. */
  connected: Page
}

export const test = base.extend<Fixtures>({
  realTailwind: [false, { option: true }],

  mock: async ({ page, realTailwind }, use) => {
    const mock = new CalDavMock()
    if (realTailwind) mock.withTailwind()
    await mock.install(page)
    await use(mock)
    // A request the mock did not recognise means the client changed shape and
    // the mock stopped speaking its language -- which would quietly turn every
    // later assertion into a test of an empty page.
    expect(mock.unhandled, `unhandled CalDAV requests:\n${mock.unhandled.join('\n')}`).toEqual([])
  },

  connected: async ({ page, mock }, use) => {
    await seedSession(page)
    await page.goto('/calendar')
    await expect(page.getByTestId('calendar-row')).toHaveCount(mock.calendars.length)
    await waitForIdle(page)
    await use(page)
  },
})

/**
 * Waits until the WASM has taken over the connect form.
 *
 * Until then the submit button is a plain button in a plain form, so clicking
 * it does a native GET navigation and the test watches the wrong page. The
 * password eye is the cheapest proof of hydration: nothing but the Leptos
 * handler can change that input's type.
 */
export async function waitForHydration(page: Page): Promise<void> {
  const field = page.getByTestId('password')
  await expect(field).toBeVisible()
  await expect(async () => {
    await page.getByTestId('password-toggle').click()
    await expect(field).toHaveAttribute('type', 'text', { timeout: 1000 })
  }).toPass({ timeout: 20_000 })
  await page.getByTestId('password-toggle').click()
  await expect(field).toHaveAttribute('type', 'password')
}

/** Waits for the sync line to stop saying it is working. */
export async function waitForIdle(page: Page): Promise<void> {
  await expect(page.getByTestId('sync-line')).toBeVisible()
  await expect(page.getByTestId('sync-line')).not.toContainText('Updating', { timeout: 15_000 })
}

/** Local `YYYY-MM-DD`, which is how day cells are addressed. */
export function isoDay(offsetDays = 0): string {
  const d = new Date()
  d.setDate(d.getDate() + offsetDays)
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

export function dayCell(page: Page, iso: string) {
  return page.locator(`[data-testid="day-cell"][data-date="${iso}"]`)
}

/** Opens the add-event form and fills the parts every event needs. */
export async function fillNewEvent(
  page: Page,
  opts: { summary: string; start?: string; end?: string; description?: string; location?: string },
): Promise<void> {
  await page.getByTestId('add-event').click()
  await expect(page.getByTestId('event-form')).toBeVisible()
  await page.getByTestId('form-summary').fill(opts.summary)
  if (opts.start) await page.getByTestId('form-start').fill(opts.start)
  if (opts.end) await page.getByTestId('form-end').fill(opts.end)
  if (opts.description) await page.getByTestId('form-description').fill(opts.description)
  if (opts.location) await page.getByTestId('form-location').fill(opts.location)
}

/** `YYYY-MM-DDTHH:MM`, the value a datetime-local input takes. */
export function localDateTime(offsetDays: number, hour: number, minute = 0): string {
  return `${isoDay(offsetDays)}T${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`
}

export type { MockCalendar }

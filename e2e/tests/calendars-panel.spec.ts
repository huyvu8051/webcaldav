import { test, expect, isoDay, dayCell, waitForIdle } from '../fixtures/app'

test.describe('the calendars panel', () => {
  test('lists every calendar with a colour and a checkbox', async ({ connected: page }) => {
    await expect(page.getByTestId('calendar-row')).toHaveCount(2)
    for (const row of await page.getByTestId('calendar-row').all()) {
      await expect(row.getByTestId('calendar-visible')).toBeChecked()
      await expect(row.getByTestId('calendar-color')).toBeVisible()
    }
  })

  test('hiding a calendar removes its events but keeps the others', async ({ connected: page }) => {
    await dayCell(page, isoDay(2)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(1)

    const personal = page.locator('[data-testid="calendar-row"]').filter({ hasText: 'Personal' })
    await personal.getByTestId('calendar-visible').uncheck()
    await expect(page.getByTestId('event-card')).toHaveCount(0)
    await expect(dayCell(page, isoDay(2))).toHaveAttribute('data-event-count', '0')

    // Work is untouched.
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-event-count', '2')
  })

  test('unhiding brings them back without a refetch', async ({ mock, page, connected }) => {
    void connected
    const personal = page.locator('[data-testid="calendar-row"]').filter({ hasText: 'Personal' })
    await personal.getByTestId('calendar-visible').uncheck()
    await expect(dayCell(page, isoDay(2))).toHaveAttribute('data-event-count', '0')

    mock.clearRequests()
    await personal.getByTestId('calendar-visible').check()
    await expect(dayCell(page, isoDay(2))).toHaveAttribute('data-event-count', '1')
    // Visibility is a local filter over data already held, not a new query.
    expect(mock.countOf('REPORT')).toBe(0)
  })

  test('hiding survives a reload', async ({ page, connected }) => {
    void connected
    await page.locator('[data-testid="calendar-row"]').filter({ hasText: 'Personal' }).getByTestId('calendar-visible').uncheck()
    await expect(dayCell(page, isoDay(2))).toHaveAttribute('data-event-count', '0')

    await page.reload()
    await waitForIdle(page)
    await expect(
      page.locator('[data-testid="calendar-row"]').filter({ hasText: 'Personal' }).getByTestId('calendar-visible'),
    ).not.toBeChecked()
    await expect(dayCell(page, isoDay(2))).toHaveAttribute('data-event-count', '0')
  })

  test('hiding every calendar leaves an empty but working month', async ({ connected: page }) => {
    for (const row of await page.getByTestId('calendar-row').all()) {
      await row.getByTestId('calendar-visible').uncheck()
    }
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-event-count', '0')
    await expect(page.getByTestId('day-cell')).toHaveCount(42)
    await expect(page.getByTestId('add-event')).toBeEnabled()
  })

  test('a colour change survives a reload', async ({ page, connected }) => {
    void connected
    const swatch = page.locator('[data-testid="calendar-row"]').filter({ hasText: 'Work' }).getByTestId('calendar-color')
    await swatch.evaluate((el: HTMLInputElement) => {
      el.value = '#123456'
      el.dispatchEvent(new Event('input', { bubbles: true }))
    })
    await expect(swatch).toHaveValue('#123456')

    await page.reload()
    await waitForIdle(page)
    await expect(
      page.locator('[data-testid="calendar-row"]').filter({ hasText: 'Work' }).getByTestId('calendar-color'),
    ).toHaveValue('#123456')
  })

  test('the dot on a day takes its colour from the calendar', async ({ page, connected }) => {
    void connected
    await expect(dayCell(page, isoDay(2)).getByTestId('day-dots').locator('span')).toHaveCount(1)
    const dot = dayCell(page, isoDay(2)).getByTestId('day-dots').locator('span').first()
    const before = await dot.evaluate((e) => getComputedStyle(e).backgroundColor)

    const swatch = page.locator('[data-testid="calendar-row"]').filter({ hasText: 'Personal' }).getByTestId('calendar-color')
    await swatch.evaluate((el: HTMLInputElement) => {
      el.value = '#ff0000'
      el.dispatchEvent(new Event('input', { bubbles: true }))
    })
    await expect.poll(() => dot.evaluate((e) => getComputedStyle(e).backgroundColor)).not.toBe(before)
  })

  // A calendar with no name is covered in data-edge-cases.spec.ts, where the
  // `{}` bug it exposes is tracked.

  test('a hidden calendar is not offered as a target for a new event', async ({ page, connected }) => {
    void connected
    await page.locator('[data-testid="calendar-row"]').filter({ hasText: 'Work' }).getByTestId('calendar-visible').uncheck()
    await page.getByTestId('add-event').click()
    // The form defaults to the first *visible* calendar rather than one the
    // user has just said they do not want to see.
    await expect(page.getByTestId('form-calendar-url')).not.toHaveValue(/work/)
  })
})

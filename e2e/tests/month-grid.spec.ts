import { test, expect, isoDay, dayCell, ymd } from '../fixtures/app'

test.describe('the month grid', () => {
  test('is six weeks of seven days, Monday first', async ({ connected: page }) => {
    await expect(page.getByTestId('day-cell')).toHaveCount(42)
    const labels = await page.locator('.grid-cols-7').first().allInnerTexts()
    expect(labels.join(' ')).toContain('Mon')

    // Column 0 must be a Monday for every row, which is the assertion that
    // actually catches an off-by-one in the leading-blank calculation.
    const dates = await page
      .getByTestId('day-cell')
      .evaluateAll((els) => els.map((e) => e.getAttribute('data-date')!))
    for (let i = 0; i < dates.length; i += 7) {
      expect(new Date(dates[i] + 'T12:00:00').getDay(), `${dates[i]} should be a Monday`).toBe(1)
    }
  })

  test('runs consecutively with no gaps or repeats', async ({ connected: page }) => {
    const dates = await page
      .getByTestId('day-cell')
      .evaluateAll((els) => els.map((e) => e.getAttribute('data-date')!))
    expect(new Set(dates).size).toBe(42)
    for (let i = 1; i < dates.length; i++) {
      const prev = new Date(dates[i - 1] + 'T12:00:00')
      const cur = new Date(dates[i] + 'T12:00:00')
      expect(Math.round((+cur - +prev) / 86_400_000)).toBe(1)
    }
  })

  test('days outside the month are marked and dimmed', async ({ connected: page }) => {
    const out = page.locator('[data-testid="day-cell"][data-in-month="false"]')
    expect(await out.count()).toBeGreaterThan(0)
    // Dimmed by colour, not opacity: `opacity-40` on #1d1d1f is 1.8:1, and
    // these cells are still tappable, so that was unreadable interactive text.
    await expect(out.first().locator('span').first()).toHaveClass(/text-ink-48/)
    await expect(out.first().getByTestId('day-dots')).toHaveClass(/opacity-40/)
    const inside = await page.locator('[data-testid="day-cell"][data-in-month="true"]').count()
    expect([28, 29, 30, 31]).toContain(inside)
  })

  test('today is marked exactly once', async ({ connected: page }) => {
    await expect(page.locator('[data-testid="day-cell"][data-today="true"]')).toHaveCount(1)
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-today', 'true')
  })

  test('nothing is selected until you tap, then exactly one thing is', async ({ connected: page }) => {
    await expect(page.locator('[data-testid="day-cell"][data-selected="true"]')).toHaveCount(0)
    await expect(page.getByTestId('day-detail-empty')).toBeVisible()

    await dayCell(page, isoDay(0)).click()
    await expect(page.locator('[data-testid="day-cell"][data-selected="true"]')).toHaveCount(1)
    await expect(page.getByTestId('day-detail')).toBeVisible()

    await dayCell(page, isoDay(1)).click()
    await expect(page.locator('[data-testid="day-cell"][data-selected="true"]')).toHaveCount(1)
    await expect(dayCell(page, isoDay(1))).toHaveAttribute('data-selected', 'true')
  })

  test('the selected day is filled, not just tinted', async ({ connected: page }) => {
    // The first pass used a faint tint and it was unreadable on a phone in
    // daylight; the disc is the fix, so it is worth asserting.
    const disc = dayCell(page, isoDay(0)).locator('span').first()
    await dayCell(page, isoDay(0)).click()
    await expect(disc).toHaveClass(/bg-action/)
    await expect(disc).toHaveClass(/text-canvas/)
  })

  test('dots count events, capped at three', async ({ mock, page, connected }) => {
    void connected
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-event-count', '2')
    await expect(dayCell(page, isoDay(0)).getByTestId('day-dots').locator('span')).toHaveCount(2)
    await expect(dayCell(page, isoDay(3))).toHaveAttribute('data-event-count', '0')
    void mock
  })

  test('a day with five events still shows three dots', async ({ mock, page }) => {
    mock.calendar('work').events = [1, 2, 3, 4, 5].map((n) => ({
      uid: `ev-${n}`,
      summary: `Thing ${n}`,
      start: `${ymd(0)}T0${n}0000`,
      end: `${ymd(0)}T0${n}3000`,
    }))
    mock.calendars = [mock.calendar('work')]
    await page.addInitScript(() => {
      window.localStorage.setItem(
        'webcaldav.connection',
        JSON.stringify({ server_url: 'https://dav.example.test/', username: 'alice', password: 'hunter2' }),
      )
    })
    await page.goto('/calendar')
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-event-count', '5')
    await expect(dayCell(page, isoDay(0)).getByTestId('day-dots').locator('span')).toHaveCount(3)
  })

  test('the day detail lists that day’s events and counts them', async ({ connected: page }) => {
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(2)
    await expect(page.getByTestId('day-detail-count')).toContainText('2')
    await expect(page.getByTestId('event-summary').first()).toHaveText(/Standup|Design review/)

    await dayCell(page, isoDay(2)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(1)
    await expect(page.getByTestId('event-summary')).toHaveText('Dentist')
  })

  test('a day with nothing on it says so rather than showing an empty list', async ({ connected: page }) => {
    await dayCell(page, isoDay(5)).click()
    await expect(page.getByTestId('day-detail')).toBeVisible()
    await expect(page.getByTestId('event-card')).toHaveCount(0)
  })

  test('event cards carry time, title and description', async ({ connected: page }) => {
    await dayCell(page, isoDay(0)).click()
    const card = page.locator('[data-testid="event-card"]').filter({ hasText: 'Design review' })
    await expect(card.getByTestId('event-time')).toContainText('–')
    await expect(card.getByTestId('event-description')).toHaveText('Bring the mocks')
  })

  test('an event with no description renders no description line', async ({ connected: page }) => {
    await dayCell(page, isoDay(0)).click()
    const card = page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' })
    await expect(card.getByTestId('event-description')).toHaveCount(0)
  })
})

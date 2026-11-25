import { test, expect, isoDay, dayCell } from '../fixtures/app'

const MONTHS = ['January','February','March','April','May','June','July','August','September','October','November','December']

function monthName(offset: number): string {
  const d = new Date()
  d.setDate(1)
  d.setMonth(d.getMonth() + offset)
  return `${MONTHS[d.getMonth()]} ${d.getFullYear()}`
}

test.describe('moving between months', () => {
  test('opens on the current month', async ({ connected: page }) => {
    await expect(page.getByTestId('month-title')).toContainText(monthName(0))
  })

  test('the chevrons step one month at a time, in both directions', async ({ connected: page }) => {
    await page.getByTestId('month-next').click()
    await expect(page.getByTestId('month-title')).toContainText(monthName(1))
    await page.getByTestId('month-next').click()
    await expect(page.getByTestId('month-title')).toContainText(monthName(2))
    await page.getByTestId('month-prev').click()
    await page.getByTestId('month-prev').click()
    await page.getByTestId('month-prev').click()
    await expect(page.getByTestId('month-title')).toContainText(monthName(-1))
  })

  test('stepping across a year boundary keeps the year right', async ({ connected: page }) => {
    // Twelve forward must land on the same month one year on. An off-by-one
    // in the month arithmetic shows up here and nowhere else.
    for (let i = 0; i < 12; i++) await page.getByTestId('month-next').click()
    await expect(page.getByTestId('month-title')).toContainText(monthName(12))
  })

  test('Today returns to the current month and the grid follows', async ({ connected: page }) => {
    await page.getByTestId('month-next').click()
    await page.getByTestId('month-next').click()
    await expect(dayCell(page, isoDay(0))).toHaveCount(0)

    await page.getByTestId('today-button').click()
    await expect(page.getByTestId('month-title')).toContainText(monthName(0))
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-today', 'true')
  })

  test('the picker is closed until the title is tapped', async ({ connected: page }) => {
    await expect(page.getByTestId('month-title')).toHaveAttribute('aria-expanded', 'false')
    await expect(page.getByTestId('month-select')).toHaveCount(0)

    await page.getByTestId('month-title').click()
    await expect(page.getByTestId('month-title')).toHaveAttribute('aria-expanded', 'true')
    await expect(page.getByTestId('month-select')).toBeVisible()
    await expect(page.getByTestId('year-select')).toBeVisible()

    await page.getByTestId('month-title').click()
    await expect(page.getByTestId('month-select')).toHaveCount(0)
  })

  test('the chevron glyph reflects which way the picker will go', async ({ connected: page }) => {
    await expect(page.getByTestId('month-title')).toContainText('expand_more')
    await page.getByTestId('month-title').click()
    await expect(page.getByTestId('month-title')).toContainText('expand_less')
  })

  test('the picker opens showing the month on screen', async ({ connected: page }) => {
    const now = new Date()
    await page.getByTestId('month-title').click()
    await expect(page.getByTestId('month-select')).toHaveValue(String(now.getMonth() + 1))
    await expect(page.getByTestId('year-select')).toHaveValue(String(now.getFullYear()))
  })

  test('choosing a month jumps there and keeps the year', async ({ connected: page }) => {
    const year = new Date().getFullYear()
    await page.getByTestId('month-title').click()
    await page.getByTestId('month-select').selectOption('3')
    await expect(page.getByTestId('month-title')).toContainText(`March ${year}`)
    await expect(page.getByTestId('year-select')).toHaveValue(String(year))
  })

  test('choosing a year jumps there and keeps the month', async ({ connected: page }) => {
    const now = new Date()
    const target = now.getFullYear() + 3
    await page.getByTestId('month-title').click()
    await page.getByTestId('year-select').selectOption(String(target))
    await expect(page.getByTestId('month-title')).toContainText(`${MONTHS[now.getMonth()]} ${target}`)
    await expect(page.getByTestId('month-select')).toHaveValue(String(now.getMonth() + 1))
  })

  test('the picker offers twelve months and twenty-one years', async ({ connected: page }) => {
    await page.getByTestId('month-title').click()
    await expect(page.getByTestId('month-select').locator('option')).toHaveCount(12)
    await expect(page.getByTestId('year-select').locator('option')).toHaveCount(21)

    const years = await page.getByTestId('year-select').locator('option').evaluateAll((o) => o.map((e) => Number((e as HTMLOptionElement).value)))
    const now = new Date().getFullYear()
    expect(Math.min(...years)).toBe(now - 10)
    expect(Math.max(...years)).toBe(now + 10)
  })

  test('months are named, not numbered', async ({ connected: page }) => {
    await page.getByTestId('month-title').click()
    const labels = await page.getByTestId('month-select').locator('option').allInnerTexts()
    expect(labels.map((l) => l.trim())).toEqual(MONTHS)
  })

  test('the chevrons still work while the picker is open', async ({ connected: page }) => {
    await page.getByTestId('month-title').click()
    await page.getByTestId('month-next').click()
    await expect(page.getByTestId('month-title')).toContainText(monthName(1))
    // And the picker follows the grid rather than holding a stale value.
    const d = new Date()
    d.setDate(1)
    d.setMonth(d.getMonth() + 1)
    await expect(page.getByTestId('month-select')).toHaveValue(String(d.getMonth() + 1))
  })

  test('changing month does not refetch -- everything is already loaded', async ({ mock, page, connected }) => {
    void connected
    mock.clearRequests()
    await page.getByTestId('month-next').click()
    await page.getByTestId('month-prev').click()
    await page.waitForTimeout(800)
    expect(mock.requests).toHaveLength(0)
  })

  test('a day selected in one month does not leak into another', async ({ connected: page }) => {
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(2)
    await page.getByTestId('month-next').click()
    // The selection is outside the rendered grid now, so no cell is marked.
    await expect(page.locator('[data-testid="day-cell"][data-selected="true"]')).toHaveCount(0)
  })
})

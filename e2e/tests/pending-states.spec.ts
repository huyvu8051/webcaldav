import { test, expect, CREDS, isoDay, dayCell, fillNewEvent, localDateTime, waitForIdle , waitForHydration } from '../fixtures/app'

/**
 * Before this pass, saving and deleting showed nothing at all between the tap
 * and the list refreshing -- the complaint that started the work was "bấm xong
 * chả thấy gì hết". Every assertion here is about that gap.
 */
test.describe('in-flight feedback', () => {
  test('Connect shows a spinner and locks while it waits', async ({ mock, page }) => {
    // Held on the very first request of the chain, so the button is pending
    // from the moment it is tapped.
    const release = mock.hold('PROPFIND /.well-known')
    await page.goto('/')
    await waitForHydration(page)
    await page.getByTestId('server-url').fill(CREDS.server)
    await page.getByTestId('username').fill(CREDS.username)
    await page.getByTestId('password').fill(CREDS.password)
    await page.getByTestId('connect-submit').click()

    const btn = page.getByTestId('connect-submit')
    await expect(btn).toContainText('Connecting')
    await expect(btn.getByTestId('spinner')).toBeVisible()
    await expect(btn).toBeDisabled()
    await expect(btn).toHaveAttribute('aria-busy', 'true')
    release()
    await expect(page).toHaveURL(/\/calendar/, { timeout: 20_000 })
  })

  test('Save shows a spinner and locks while it waits', async ({ mock, page, connected }) => {
    void connected
    const release = mock.hold('PUT')
    await fillNewEvent(page, { summary: 'Slow one', start: localDateTime(0, 9), end: localDateTime(0, 10) })
    await page.getByTestId('form-submit').click()

    const btn = page.getByTestId('form-submit')
    await expect(btn).toContainText('Saving')
    await expect(btn.getByTestId('spinner')).toBeVisible()
    await expect(btn).toBeDisabled()
    await expect(btn).toHaveAttribute('aria-busy', 'true')
    release()
    await expect(page.getByTestId('event-form')).toHaveCount(0, { timeout: 20_000 })
  })

  test('a second tap on Save cannot send a second PUT', async ({ mock, page, connected }) => {
    void connected
    // Two PUTs of the same new event would create two objects on the server,
    // which is why the button disables rather than just changing its label.
    const release = mock.hold('PUT')
    await fillNewEvent(page, { summary: 'Only once', start: localDateTime(0, 9), end: localDateTime(0, 10) })

    const btn = page.getByTestId('form-submit')
    await btn.click()
    await expect(btn).toBeDisabled()
    await btn.click({ force: true }).catch(() => {})
    await btn.click({ force: true }).catch(() => {})
    release()
    await expect(page.getByTestId('event-form')).toHaveCount(0, { timeout: 20_000 })

    expect(mock.countOf('PUT')).toBe(1)
    expect(mock.calendar('work').events.filter((e) => e.summary === 'Only once')).toHaveLength(1)
  })

  test('deleting spins the row that was tapped and no other', async ({ mock, page, connected }) => {
    void connected
    const release = mock.hold('DELETE')
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(2)

    const standup = page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' })
    const review = page.locator('[data-testid="event-card"]').filter({ hasText: 'Design review' })
    await standup.getByTestId('event-delete').click()

    // One action serves the whole list, so this is the assertion that the
    // per-row narrowing by `action.input()` actually works.
    await expect(standup.getByTestId('spinner')).toBeVisible()
    await expect(standup.getByTestId('event-delete')).toBeDisabled()
    await expect(review.getByTestId('spinner')).toHaveCount(0)
    await expect(review.getByTestId('event-delete')).toBeEnabled()
    release()
  })

  test('a second tap on the same delete cannot send a second DELETE', async ({ mock, page, connected }) => {
    void connected
    const release = mock.hold('DELETE')
    await dayCell(page, isoDay(0)).click()
    const btn = page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).getByTestId('event-delete')
    await btn.click()
    await btn.click({ force: true }).catch(() => {})
    release()
    await waitForIdle(page)
    expect(mock.countOf('DELETE')).toBe(1)
  })

  test('Refresh locks while it runs and the sync line says so', async ({ mock, page, connected }) => {
    void connected
    const release = mock.hold('REPORT')
    await page.getByTestId('refresh').click()

    await expect(page.getByTestId('refresh')).toBeDisabled()
    await expect(page.getByTestId('sync-line')).toContainText('Updating')
    await expect(page.getByTestId('sync-line').getByTestId('spinner')).toBeVisible()
    release()
    await expect(page.getByTestId('refresh')).toBeEnabled({ timeout: 20_000 })
  })

  test('every control comes back after the work finishes', async ({ page, connected }) => {
    void connected
    await fillNewEvent(page, { summary: 'Then enabled', start: localDateTime(0, 9), end: localDateTime(0, 10) })
    await page.getByTestId('form-submit').click()
    await waitForIdle(page)

    await expect(page.getByTestId('refresh')).toBeEnabled()
    await expect(page.getByTestId('add-event')).toBeEnabled()
    await expect(page.getByTestId('spinner')).toHaveCount(0)
  })

  test('the spinner holds still when the platform asks for less motion', async ({ page, connected }) => {
    void connected
    await page.emulateMedia({ reducedMotion: 'reduce' })
    await page.getByTestId('add-event').click()
    await page.getByTestId('form-summary').fill('x')
    await page.getByTestId('form-start').fill(localDateTime(0, 9))
    await page.getByTestId('form-end').fill(localDateTime(0, 10))

    // The blanket reduced-motion rule sets every animation to 0.01ms, which
    // would strobe a spinner rather than calm it; `.spinner` overrides that.
    const animation = await page.evaluate(() => {
      const el = document.createElement('span')
      el.className = 'spinner'
      document.body.appendChild(el)
      const v = getComputedStyle(el).animationName
      el.remove()
      return v
    })
    expect(animation).toBe('none')
  })
})

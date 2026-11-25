import { test, expect, CREDS, seedSession , waitForHydration } from '../fixtures/app'

test.describe('connect', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('shows the three fields and the preset chips', async ({ page }) => {
    await expect(page.getByTestId('server-url')).toBeVisible()
    await expect(page.getByTestId('username')).toBeVisible()
    await expect(page.getByTestId('password')).toBeVisible()
    await expect(page.getByTestId('preset')).toHaveCount(4)
  })

  test('a preset chip fills the server address and nothing else', async ({ page }) => {
    await page.locator('[data-testid="preset"][data-preset="Radicale"]').click()
    await expect(page.getByTestId('server-url')).toHaveValue(/caldav\.example\.com/)
    await expect(page.getByTestId('username')).toHaveValue('')
    await expect(page.getByTestId('password')).toHaveValue('')
  })

  test('the password is masked until the eye is tapped, and masks again', async ({ page }) => {
    const field = page.getByTestId('password')
    await field.fill('hunter2')
    await expect(field).toHaveAttribute('type', 'password')
    await page.getByTestId('password-toggle').click()
    await expect(field).toHaveAttribute('type', 'text')
    await page.getByTestId('password-toggle').click()
    await expect(field).toHaveAttribute('type', 'password')
  })

  test('submitting empty sends no request -- the fields are required', async ({ mock, page }) => {
    await page.getByTestId('connect-submit').click()
    await page.waitForTimeout(600)
    expect(mock.requests).toHaveLength(0)
    await expect(page).toHaveURL(/\/$/)
  })

  test('a good server lands on the calendar and stores the session', async ({ mock, page }) => {
    await fillConnect(page)
    await page.getByTestId('connect-submit').click()
    await expect(page).toHaveURL(/\/calendar/)
    await expect(page.getByTestId('calendar-row')).toHaveCount(2)

    const stored = await page.evaluate(() => window.localStorage.getItem('webcaldav.connection'))
    expect(JSON.parse(stored!)).toMatchObject({ username: 'alice' })
    expect(mock.countOf('PROPFIND /')).toBeGreaterThan(0)
  })

  test('401 keeps you on the page and says so', async ({ mock, page }) => {
    mock.fail({ match: 'PROPFIND /', status: 401 })
    await fillConnect(page)
    await page.getByTestId('connect-submit').click()

    await expect(page.getByTestId('connect-error')).toBeVisible()
    await expect(page).toHaveURL(/\/$/)
    // Nothing is stored on a failed attempt -- otherwise the next load would
    // redirect straight to a calendar that cannot be fetched.
    expect(await page.evaluate(() => window.localStorage.getItem('webcaldav.connection'))).toBeNull()
  })

  test('the button recovers after a failure, and a retry works', async ({ mock, page }) => {
    // Regression: the old hand-managed `connecting` flag was set before the
    // request and only cleared on the error branch of one match arm, so some
    // failures left the button disabled forever. `.pending()` cannot do that.
    mock.fail({ match: 'PROPFIND /', status: 500, times: 2 })
    await fillConnect(page)
    await page.getByTestId('connect-submit').click()
    await expect(page.getByTestId('connect-error')).toBeVisible()
    await expect(page.getByTestId('connect-submit')).toBeEnabled()
    await expect(page.getByTestId('connect-submit')).toContainText('Connect')

    await page.getByTestId('connect-submit').click()
    await expect(page).toHaveURL(/\/calendar/)
  })

  test('a stored session skips the form entirely', async ({ page }) => {
    await seedSession(page)
    await page.goto('/')
    await expect(page).toHaveURL(/\/calendar/)
  })

  test('the password never goes to this app’s own origin', async ({ mock, page }) => {
    void mock
    // The whole premise is that credentials stay in the browser. If any
    // same-origin request ever carried them, that premise is broken.
    const leaked: string[] = []
    page.on('request', (r) => {
      const url = new URL(r.url())
      if (!url.host.endsWith('8090')) return
      const body = r.postData() ?? ''
      if (body.includes(CREDS.password) || r.url().includes(CREDS.password)) {
        leaked.push(`${r.method()} ${r.url()}`)
      }
    })
    await fillConnect(page)
    await page.getByTestId('connect-submit').click()
    await expect(page).toHaveURL(/\/calendar/)
    expect(leaked).toEqual([])
  })
})

async function fillConnect(page: import('@playwright/test').Page) {
  await waitForHydration(page)
    await page.getByTestId('server-url').fill(CREDS.server)
  await page.getByTestId('username').fill(CREDS.username)
  await page.getByTestId('password').fill(CREDS.password)
}

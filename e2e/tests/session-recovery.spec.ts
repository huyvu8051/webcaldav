import { test, expect, seedSession, CREDS } from '../fixtures/app'

/**
 * What the page does when it cannot finish loading.
 *
 * `username`, the CalDAV config and the Disconnect handler are all set inside
 * the `#[cfg(feature = "hydrate")]` effect, so if the WASM never runs the
 * server-rendered page stays on screen: a name skeleton that never fills in,
 * a permanent "Updating…", an inert Disconnect, no calendars, and no error
 * saying why. It reads as stuck rather than as an empty account -- which is
 * an improvement on where it started -- but it is still a dead end.
 *
 * This is not hypothetical. Each pod bakes in exactly one content-hashed
 * bundle and serves 404 for any other hash; the Service has
 * `sessionAffinity: None` and the deployment runs two replicas with
 * `maxSurge: 1`. During a rollout the HTML can come from the new pod while
 * the follow-up request for its bundle round-robins to the old one.
 */

/** Everything the page can tell someone about whether they are signed in. */
async function sessionState(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    let stored: boolean | 'unavailable' = 'unavailable'
    try {
      stored = !!window.localStorage.getItem('webcaldav.connection')
    } catch {
      /* private mode */
    }
    return {
      path: location.pathname,
      username: document.querySelector('[data-testid="topbar-username"]')?.textContent?.trim() ?? null,
      skeleton: !!document.querySelector('[data-testid="topbar-username-skeleton"]'),
      hasDisconnect: !!document.querySelector('[data-testid="disconnect"]'),
      calendars: document.querySelectorAll('[data-testid="calendar-row"]').length,
      error: !!document.querySelector('[data-testid="status-error"]'),
      stored,
    }
  })
}

const BREAKAGE = [
  { name: 'the wasm bundle 404s', pattern: '**/pkg/*.wasm', body: '' },
  { name: 'the hydrate script 404s', pattern: '**/pkg/*.js', body: '' },
] as const

test.describe('a page that never hydrates', () => {
  for (const { name, pattern } of BREAKAGE) {
    test(`${name}: the page still claims to be signed in`, async ({ mock, page }) => {
      void mock
      await page.route(pattern, (r) => r.fulfill({ status: 404, body: '' }))
      await seedSession(page)
      await page.goto('/calendar')
      await page.waitForTimeout(2500)

      const s = await sessionState(page)
      // Current behaviour, pinned. The page at least no longer looks
      // *finished*: since the pre-hydration loading state was added it sits
      // on "Updating…" with a name skeleton, which reads as stuck rather than
      // as an empty account. It is still a trap, though -- see below.
      expect(s.path).toBe('/calendar')
      expect(s.username, 'the name never arrives').toBeNull()
      expect(s.skeleton, 'stuck on the loading skeleton forever').toBe(true)
      expect(s.hasDisconnect).toBe(true)
      expect(s.calendars).toBe(0)
      expect(s.error, 'nothing ever says what went wrong').toBe(false)
    })

    test(`${name}: Disconnect does nothing, so there is no way out`, async ({ mock, page }) => {
      void mock
      await page.route(pattern, (r) => r.fulfill({ status: 404, body: '' }))
      await seedSession(page)
      await page.goto('/calendar')
      await page.getByTestId('disconnect').click()
      await page.waitForTimeout(1000)

      const s = await sessionState(page)
      expect(s.path, 'still on the signed-in route').toBe('/calendar')
      expect(s.stored, 'credentials still stored').toBe(true)
    })
  }

  test('a script that throws leaves the same state', async ({ mock, page }) => {
    void mock
    await page.route('**/pkg/*.js', (r) =>
      r.fulfill({ status: 200, headers: { 'content-type': 'application/javascript' }, body: 'throw new Error("boom")' }),
    )
    await seedSession(page)
    await page.goto('/calendar')
    await page.waitForTimeout(2500)
    const s = await sessionState(page)
    expect(s.path).toBe('/calendar')
    expect(s.username).toBeNull()
    expect(s.skeleton).toBe(true)
    expect(s.error).toBe(false)
  })

  // KNOWN BUG. Nothing tells the user, and nothing lets them out. The only
  // escape is clearing site data by hand. Two fixes, in BACKLOG.md as S1:
  // keep the HTML and its bundle on one pod so the trigger goes away, and add
  // an inline bootstrap guard -- plain HTML, no module -- that surfaces a
  // "could not load" banner with a button that clears storage and returns to
  // the connect page.
  test.fail('a page that cannot hydrate says so', async ({ mock, page }) => {
    void mock
    await page.route('**/pkg/*.wasm', (r) => r.fulfill({ status: 404, body: '' }))
    await seedSession(page)
    await page.goto('/calendar')
    // Either an explanation, or a way back to the connect page. Currently
    // neither arrives.
    await expect(page.getByTestId('status-error').or(page.getByTestId('server-url'))).toBeVisible({ timeout: 12_000 })
  })
})

test.describe('states that already recover correctly', () => {
  test('a corrupt stored config sends you back to connect', async ({ mock, page }) => {
    void mock
    await page.addInitScript(() => window.localStorage.setItem('webcaldav.connection', '{not json'))
    await page.goto('/calendar')
    await expect(page).toHaveURL(/\/$/)
    await expect(page.getByTestId('server-url')).toBeVisible()
  })

  test('an unreachable server still shows who you are and lets you leave', async ({ mock, page }) => {
    mock.fail({ match: 'PROPFIND', abort: true })
    await seedSession(page)
    await page.goto('/calendar')

    await expect(page.getByTestId('status-error')).toBeVisible({ timeout: 15_000 })
    await expect(page.getByTestId('topbar-username')).toHaveText(CREDS.username)
    await page.getByTestId('disconnect').click()
    await expect(page).toHaveURL(/\/$/)
    expect(await page.evaluate(() => window.localStorage.getItem('webcaldav.connection'))).toBeNull()
  })

  test('credentials rejected later are reported, not swallowed', async ({ mock, page }) => {
    // The password changed on the server since you connected.
    mock.fail({ match: 'PROPFIND', status: 401 })
    await seedSession(page)
    await page.goto('/calendar')

    await expect(page.getByTestId('status-error')).toContainText(/401|credential/i)
    await expect(page.getByTestId('topbar-username')).toHaveText(CREDS.username)
    await page.getByTestId('disconnect').click()
    await expect(page).toHaveURL(/\/$/)
  })

  test('storage blocked entirely sends you back to connect', async ({ mock, page }) => {
    void mock
    // Safari in private mode, or a profile with site data disabled.
    await page.addInitScript(() => {
      Object.defineProperty(window, 'localStorage', {
        configurable: true,
        get() {
          throw new DOMException('denied', 'SecurityError')
        },
      })
    })
    await page.goto('/calendar')
    await expect(page).toHaveURL(/\/$/)
  })
})

// KNOWN BUG, minor. `load()` returns None for unparseable JSON and the page
// redirects, but the unparseable value is left in localStorage, so every
// subsequent visit repeats the round trip.
test.fail('a corrupt stored config is cleared, not left behind', async ({ mock, page }) => {
  void mock
  await page.addInitScript(() => window.localStorage.setItem('webcaldav.connection', '{not json'))
  await page.goto('/calendar')
  await expect(page.getByTestId('server-url')).toBeVisible()
  expect(await page.evaluate(() => window.localStorage.getItem('webcaldav.connection'))).toBeNull()
})

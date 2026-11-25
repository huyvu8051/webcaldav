import { defineConfig, devices } from '@playwright/test'

// The app is phone-only by design, so the phone is the default project and
// desktop exists only to assert that the same column is centred in more
// background. See e2e/tests/responsive.spec.ts.
export default defineConfig({
  testDir: './e2e/tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: process.env.CI ? [['github'], ['html', { open: 'never' }]] : [['list']],
  timeout: 30_000,
  expect: { timeout: 7_000 },
  use: {
    baseURL: process.env.BASE_URL ?? 'http://127.0.0.1:8090',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    // Behaviour runs once, on WebKit at iPhone size: that is the engine and
    // the viewport this app is built for, and asserting the same interactions
    // twice mostly buys engine-specific noise (Chrome renders `accent-color`
    // as #1269c0, for one, which is not a second accent in the design).
    { name: 'phone', use: { ...devices['iPhone 14 Pro Max'] } },
    // Desktop exists to prove the one thing that is specifically about
    // desktop: the same column, centred in more background.
    {
      name: 'desktop',
      use: { ...devices['Desktop Chrome'] },
      testMatch: /(responsive|design-system)\.spec\.ts/,
    },
  ],
  webServer: {
    command: 'cargo leptos serve',
    url: 'http://127.0.0.1:8090/',
    reuseExistingServer: true,
    timeout: 600_000,
  },
})

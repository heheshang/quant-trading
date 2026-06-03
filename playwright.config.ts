/**
 * Playwright config for quant-trading E2E suite.
 *
 * Usage:
 *   npx playwright test                      # everything
 *   npx playwright test e2e-tests/auth_full  # single file
 *   npx playwright test --ui                 # debug UI
 *
 * All spec files live in e2e-tests/ and use the helpers exported from
 * runner.mjs.  The base URL and AI service URL are picked up from env so
 * CI can point at any deployment.
 */
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e-tests',
  testMatch: /.*\.spec\.ts$/,
  // Auth.spec.ts is the original example kept for reference; spec files
  // we author for the P0-2 expansion all live alongside it.
  fullyParallel: false,        // tests share a registered user / state
  workers: 1,                  // keep server state deterministic
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: [
    ['list'],
    ['html', { outputFolder: 'e2e-report', open: 'never' }],
    ['json', { outputFile: 'e2e-report/results.json' }],
  ],
  timeout: 60_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL: process.env.E2E_BASE_URL || 'http://localhost:8080',
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    actionTimeout: 15_000,
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
});

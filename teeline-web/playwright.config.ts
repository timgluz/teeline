import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './tests',
  // tests/auth runs under playwright.auth.config.ts (wrangler pages dev +
  // WebAuthn virtual authenticator) — not the astro dev server here.
  testIgnore: '**/auth/**',
  // The explainer specs animate a whole solver run and legitimately take ~25s (bhk's DP run is the
  // longest), so a 30s cap was one slow machine away from failing them.
  timeout: 60_000,
  // Animation timing under a loaded CI box is the one thing here that cannot be made
  // deterministic from the test: a retry that passes is reported as flaky, so a real regression
  // still fails twice.
  retries: process.env.CI ? 1 : 0,
  use: {
    baseURL: 'http://localhost:4321',
    headless: true,
    // Only on a retry, so a passing run pays nothing; the artifact upload in web-e2e.yml is what
    // makes a CI-only failure diagnosable after the runner is gone.
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chrome',
      use: {
        ...devices['Desktop Chrome'],
        channel: 'chrome',
      },
    },
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:4321',
    reuseExistingServer: !process.env.CI,
    // A cold Vite dependency optimisation on a fresh CI checkout regularly takes longer than the
    // 30s this used to allow, and Playwright gives up before the first test runs.
    timeout: 120_000,
    // Astro auto-detects it's running inside an AI coding agent environment
    // and silently daemonizes `astro dev` (returns immediately, server keeps
    // running detached) unless this is set — which breaks Playwright's
    // webServer process lifecycle management (it expects a blocking
    // foreground process it can spawn and kill directly).
    env: { ASTRO_DEV_BACKGROUND: 'false' },
  },
})

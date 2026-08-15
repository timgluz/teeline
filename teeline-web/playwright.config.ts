import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './tests',
  // tests/auth runs under playwright.auth.config.ts (wrangler pages dev +
  // WebAuthn virtual authenticator) — not the astro dev server here.
  testIgnore: '**/auth/**',
  timeout: 30_000,
  use: {
    baseURL: 'http://localhost:4321',
    headless: true,
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
    timeout: 30_000,
    // Astro auto-detects it's running inside an AI coding agent environment
    // and silently daemonizes `astro dev` (returns immediately, server keeps
    // running detached) unless this is set — which breaks Playwright's
    // webServer process lifecycle management (it expects a blocking
    // foreground process it can spawn and kill directly).
    env: { ASTRO_DEV_BACKGROUND: 'false' },
  },
})

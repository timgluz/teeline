import { defineConfig, devices } from '@playwright/test'

// Auth-service e2e: runs against the FULL local stack via `wrangler pages dev`
// (static site + Pages Functions + local D1), unlike the main config which
// uses `astro dev`. WebAuthn requires Chromium + a CDP virtual authenticator.
export default defineConfig({
  testDir: './tests/auth',
  timeout: 60_000,
  workers: 1,
  use: {
    baseURL: 'http://localhost:8788',
    headless: true,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command:
      'bash -c "mkdir -p .e2e-state && printf \'SESSION_SECRET=e2e-session-secret\\nAUTH_SERVICE_SECRET=e2e-shared-secret\\nALLOWED_ORIGINS=http://localhost:8788\\n\' > .dev.vars && NODE_OPTIONS=--max-old-space-size=8192 npm run build:check && npx wrangler d1 migrations apply teeline-auth --local && npx wrangler pages dev --port 8788"',
    url: 'http://localhost:8788/api-key/',
    reuseExistingServer: !process.env.CI,
    timeout: 180_000,
    // Keep wrangler's log/registry state inside the project (works on
    // read-only homes; harmless on CI).
    env: {
      WRANGLER_LOG_PATH: '.e2e-state/wrangler.log',
      XDG_CONFIG_HOME: '.e2e-state/config',
      XDG_CACHE_HOME: '.e2e-state/cache',
      HOME: '.e2e-state/home',
    },
  },
})

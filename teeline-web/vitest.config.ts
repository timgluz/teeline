import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    // node env (default) — tests import only DOM-free modules
    // Playwright tests live in tests/ — excluded by not matching this pattern
    include: ['src/**/*.test.ts'],
  },
})

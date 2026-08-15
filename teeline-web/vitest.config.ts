import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    // node env (default) — tests import only DOM-free modules
    // Playwright tests live in tests/ — excluded by not matching this pattern
    // functions/** tests use miniflare's local D1 emulation (node env)
    include: ['src/**/*.test.ts', 'functions/**/*.test.ts'],
  },
})

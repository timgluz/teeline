/// <reference types="vitest/config" />
import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    outDir: 'dist',
  },
  test: {
    // node env (default) — only import DOM-free modules in tests (e.g. app.ts, not main.ts)
    // If tests need DOM later, add: environment: 'jsdom', and devDep: jsdom
  },
})

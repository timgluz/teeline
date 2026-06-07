import { sentryVitePlugin } from "@sentry/vite-plugin";
/// <reference types="vitest/config" />
import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    outDir: 'dist',
    sourcemap: true
  },

  resolve: {
    preserveSymlinks: true,  // follow npm file: symlinks to teeline-wasm
  },

  optimizeDeps: {
    exclude: ['teeline-wasm'],  // don't pre-bundle; WASM needs to stay as-is
  },

  worker: {
    format: 'es',  // ES module workers — required for top-level await in teeline_wasm.js
  },

  test: {
    // node env (default) — tests import only DOM-free modules (solver-options.ts)
  },

  plugins: [sentryVitePlugin({
    org: "timo-sulg",
    project: "javascript"
  })]
})

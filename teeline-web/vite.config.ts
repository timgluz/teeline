import { sentryVitePlugin } from "@sentry/vite-plugin";
/// <reference types="vitest/config" />
import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      input: {
        main: 'index.html',
        fourier: 'algorithms/fourier/index.html',
        fourierExplainer: 'algorithms/fourier/explainer/index.html',
      },
    },
  },

  resolve: {
    preserveSymlinks: true,
    alias: {
      'react': 'preact/compat',
      'react-dom': 'preact/compat',
      'react-dom/client': 'preact/compat',
    },
  },

  esbuild: {
    jsxImportSource: 'preact',
  },

  optimizeDeps: {
    exclude: ['teeline-wasm'],
  },

  worker: {
    format: 'es',
  },

  test: {
    // node env (default) — tests import only DOM-free modules
  },

  plugins: [sentryVitePlugin({
    org: "timo-sulg",
    project: "javascript"
  })],
})

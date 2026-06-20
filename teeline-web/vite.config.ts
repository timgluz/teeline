import { sentryVitePlugin } from "@sentry/vite-plugin";
/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import { existsSync } from 'fs'
import { resolve as resolvePath } from 'node:path'
import { fileURLToPath } from 'node:url'
import { globSync } from 'tinyglobby'

const configDir = fileURLToPath(new URL('.', import.meta.url))

// Dynamically discover all generated solver doc pages and explainer sub-pages.
// build-docs.mjs must run before vite build to populate algorithms/*/index.html.
const algoPages = Object.fromEntries(
  globSync('algorithms/*/index.html').map(f => [f.split('/')[1], f])
)
const explainerPages = Object.fromEntries(
  globSync('algorithms/*/explainer/index.html')
    .filter(f => existsSync(f))
    .map(f => [`${f.split('/')[1]}Explainer`, f])
)

export default defineConfig({
  envPrefix: ['VITE_', 'WEBMCP_'],

  build: {
    outDir: 'dist',
    sourcemap: true,
    rollupOptions: {
      input: {
        main: 'index.html',
        ...algoPages,
        ...explainerPages,
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
    // Playwright tests live in tests/ — exclude them from Vitest
    include: ['src/**/*.test.ts'],
  },

  plugins: [
    // preview2-shim ships separate node/ and browser/ variants.
    // With preserveSymlinks:true, Vite finds the copy nested inside
    // teeline-wasm/js-bindings/node_modules/ and picks the 'node' condition.
    // Intercept here to force the browser variant from our own node_modules.
    {
      name: 'force-preview2-shim-browser',
      resolveId(id: string) {
        if (!id.startsWith('@bytecodealliance/preview2-shim')) return undefined
        const sub = id.slice('@bytecodealliance/preview2-shim'.length)
        const name = sub.replace(/^\//, '') || 'index'
        return resolvePath(configDir, `node_modules/@bytecodealliance/preview2-shim/lib/browser/${name}.js`)
      },
    },
    sentryVitePlugin({
      org: "timo-sulg",
      project: "javascript"
    }),
    // Make CSS non-blocking on the main SPA page.
    // Algorithm docs pages keep blocking CSS (static HTML needs immediate styling).
    {
      name: 'async-css-main',
      transformIndexHtml: {
        order: 'post' as const,
        handler(html: string, ctx: { filename: string }) {
          if (ctx.filename.includes('/algorithms/')) return html
          return html.replace(
            /<link rel="stylesheet" crossorigin href="([^"]+)">/g,
            `<link rel="preload" as="style" crossorigin href="$1" onload="this.onload=null;this.rel='stylesheet'">` +
            `<noscript><link rel="stylesheet" crossorigin href="$1"></noscript>`
          )
        },
      },
    },
  ],
})

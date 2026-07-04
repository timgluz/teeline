import { sentryVitePlugin } from "@sentry/vite-plugin";
/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import { existsSync } from 'fs'
import { resolve as resolvePath } from 'node:path'
import { fileURLToPath } from 'node:url'
import { globSync } from 'tinyglobby'
import { renderTopbarHtml, renderSidebarHtml } from './src/nav-html.mjs'
import { renderAlgorithmCardsHtml } from './src/algorithm-cards.mjs'
import { renderFeatureCardsHtml } from './src/feature-cards.mjs'

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
        webmcp: 'webmcp/index.html',
        tsp: 'tsp/index.html',
        apiKey: 'api-key/index.html',
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
    // Server-render the topbar/sidebar nav into every HTML entry at build time.
    // Previously these were injected client-side only (el.innerHTML = ...), so
    // Googlebot's initial (non-JS) crawl saw zero internal links between algorithm
    // pages — they were reachable only via sitemap.xml, which reads as orphan pages
    // and hurts indexing priority. Client-side init still runs (see docs-init.ts) for
    // dropdown interactivity; it just re-renders identical markup on top of this.
    {
      name: 'ssr-nav-html',
      transformIndexHtml: {
        order: 'pre' as const,
        handler(html: string, ctx: { filename: string }) {
          let out = html.replace(
            '<div id="topbar"></div>',
            `<div id="topbar">${renderTopbarHtml()}</div>`
          )
          const algoMatch = ctx.filename.match(/\/algorithms\/([^/]+)\//)
          if (algoMatch) {
            out = out.replace(
              '<nav id="algo-sidebar" aria-label="Algorithms"></nav>',
              `<nav id="algo-sidebar" aria-label="Algorithms">${renderSidebarHtml(algoMatch[1])}</nav>`
            )
          }
          out = out.replace(
            '<div id="algorithms-index"></div>',
            `<div id="algorithms-index">${renderAlgorithmCardsHtml()}</div>`
          )
          out = out.replace(
            '<div id="features-index"></div>',
            `<div id="features-index">${renderFeatureCardsHtml()}</div>`
          )
          return out
        },
      },
    },
  ],
})

import { defineConfig } from 'astro/config'
import preact from '@astrojs/preact'
import sitemap from '@astrojs/sitemap'
import { resolve as resolvePath } from 'node:path'
import { fileURLToPath } from 'node:url'
import { sentryVitePlugin } from '@sentry/vite-plugin'

const configDir = fileURLToPath(new URL('.', import.meta.url))

export default defineConfig({
  site: 'https://tspsolver.com',
  output: 'static',
  integrations: [preact(), sitemap()],
  // Astro's default output asset directory is dist/_astro/, not Vite's
  // dist/assets/. Pin it to 'assets' so scripts/copy-wasm.mjs and
  // public/_headers' Content-Type rule for *.wasm (both hardcode "assets/")
  // don't need to change.
  build: {
    assets: 'assets',
  },
  // Site is light-themed (data-theme="light", PicoCSS light) — match Shiki's
  // theme to what the old @speed-highlight/core + github-light.css pipeline
  // used. Astro's own default is github-dark, which would mismatch.
  markdown: {
    shikiConfig: {
      theme: 'github-light',
    },
  },
  vite: {
    envPrefix: ['VITE_', 'WEBMCP_'],
    resolve: {
      preserveSymlinks: true,
      alias: {
        react: 'preact/compat',
        'react-dom': 'preact/compat',
        'react-dom/client': 'preact/compat',
      },
    },
    optimizeDeps: { exclude: ['teeline-wasm'] },
    worker: { format: 'es' },
    plugins: [
      // preview2-shim ships separate node/ and browser/ variants.
      // With preserveSymlinks:true, Vite finds the copy nested inside
      // teeline-wasm/js-bindings/node_modules/ and picks the 'node' condition.
      // Intercept here to force the browser variant from our own node_modules.
      {
        name: 'force-preview2-shim-browser',
        resolveId(id) {
          if (!id.startsWith('@bytecodealliance/preview2-shim')) return undefined
          const sub = id.slice('@bytecodealliance/preview2-shim'.length)
          const name = sub.replace(/^\//, '') || 'index'
          return resolvePath(configDir, `node_modules/@bytecodealliance/preview2-shim/lib/browser/${name}.js`)
        },
      },
      sentryVitePlugin({ org: 'timo-sulg', project: 'javascript' }),
    ],
    server: {
      proxy: {
        '/tsplib': {
          target: 'https://static.tspsolver.com',
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/tsplib/, '/tsplib'),
        },
      },
    },
  },
})

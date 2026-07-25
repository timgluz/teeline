import { defineConfig } from 'astro/config'
import preact from '@astrojs/preact'
import { resolve as resolvePath } from 'node:path'
import { fileURLToPath } from 'node:url'
import { sentryVitePlugin } from '@sentry/vite-plugin'

const configDir = fileURLToPath(new URL('.', import.meta.url))

export default defineConfig({
  site: 'https://tspsolver.com',
  output: 'static',
  integrations: [preact()],
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
  },
})

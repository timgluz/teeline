import { existsSync, readdirSync, copyFileSync } from 'fs'

// Vite does not transform new URL() patterns inside node_modules-linked
// packages, so teeline_wasm.core.wasm from the jco transpile output may not
// end up in dist/assets automatically.
//
// Strategy:
//   1. If Vite hashed the file, de-hash it to the stable name the worker expects.
//   2. If Vite did not include it at all, copy directly from node_modules.
//   3. If neither source is available, fail the build loudly (don't deploy a broken app).

const distAssets = 'dist/assets'
const nodeModulesSrc = 'node_modules/teeline-wasm/teeline_wasm.core.wasm'
const stableName = 'teeline_wasm.core.wasm'
const stableDest = `${distAssets}/${stableName}`

const files = readdirSync(distAssets)
const hashed = files.find((f) => f.startsWith('teeline_wasm.core') && f.endsWith('.wasm'))

if (hashed && hashed !== stableName) {
  copyFileSync(`${distAssets}/${hashed}`, stableDest)
  console.log(`[copy-wasm] ${hashed} → ${stableName}`)
} else if (hashed === stableName) {
  console.log('[copy-wasm] WASM already has stable name, nothing to do')
} else {
  // Vite did not bundle the WASM — copy it directly from the linked package
  if (!existsSync(nodeModulesSrc)) {
    console.error('[copy-wasm] ERROR: teeline_wasm.core.wasm not found in dist/assets or node_modules/teeline-wasm')
    console.error('[copy-wasm] Run: npx @bytecodealliance/jco transpile ... -o ../teeline-wasm/js-bindings  then  npm ci')
    process.exit(1)
  }
  copyFileSync(nodeModulesSrc, stableDest)
  console.log('[copy-wasm] Copied teeline_wasm.core.wasm from node_modules/teeline-wasm (Vite did not bundle it)')
}

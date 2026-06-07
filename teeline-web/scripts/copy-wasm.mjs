import { readdirSync, copyFileSync } from 'fs'

// After vite build, the WASM gets a content hash in its filename but the URL
// baked into the worker bundle by cargo-component's generated JS keeps the
// original unhashed name.  Copy to stable name so the worker finds it.
const files = readdirSync('dist/assets')
const hashed = files.find((f) => f.startsWith('teeline_wasm.core') && f.endsWith('.wasm'))
if (hashed && hashed !== 'teeline_wasm.core.wasm') {
  copyFileSync(`dist/assets/${hashed}`, 'dist/assets/teeline_wasm.core.wasm')
  console.log(`[copy-wasm] dist/assets/${hashed} → dist/assets/teeline_wasm.core.wasm`)
} else if (hashed === 'teeline_wasm.core.wasm') {
  console.log('[copy-wasm] WASM already has stable name, nothing to do')
} else {
  console.error('[copy-wasm] WARNING: no teeline_wasm.core*.wasm found in dist/assets')
}

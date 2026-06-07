import { existsSync, readdirSync, copyFileSync } from 'fs'

// Vite does not transform new URL() patterns inside node_modules-linked
// packages (file: deps), so jco's core WASM files never reach dist/assets.
// Copy every *.wasm from node_modules/teeline-wasm to dist/assets so the
// worker can fetch them at runtime.
//
// jco transpile must be called with --name teeline_wasm so the output files
// match the names expected by the package (teeline_wasm.core.wasm, etc.).

const distAssets = 'dist/assets'
const pkgDir = 'node_modules/teeline-wasm'

if (!existsSync(pkgDir)) {
  console.error('[copy-wasm] ERROR: node_modules/teeline-wasm not found — run npm ci after jco transpile')
  process.exit(1)
}

const wasmFiles = readdirSync(pkgDir).filter((f) => f.endsWith('.wasm'))

if (wasmFiles.length === 0) {
  console.error('[copy-wasm] ERROR: no .wasm files in node_modules/teeline-wasm')
  console.error('[copy-wasm] Ensure deploy-web.yml runs: jco transpile ... --name teeline_wasm -o ../teeline-wasm/js-bindings')
  process.exit(1)
}

for (const file of wasmFiles) {
  copyFileSync(`${pkgDir}/${file}`, `${distAssets}/${file}`)
  console.log(`[copy-wasm] ${pkgDir}/${file} → ${distAssets}/${file}`)
}

# teeline-wasm

WebAssembly Component Model build of the Teeline TSP solver. Exposes a single typed `solve` function callable from Go, Python, JavaScript, Rust, and any WIT-compatible runtime.

## Build

Requires [cargo-component](https://github.com/bytecodealliance/cargo-component):

```bash
cargo component build --manifest-path teeline-wasm/Cargo.toml --release
```

The component is written to `target/wasm32-wasip2/release/teeline_wasm.wasm`.

## Usage (JavaScript via jco)

```bash
# transpile to JS bindings
npx jco transpile target/wasm32-wasip2/release/teeline_wasm.wasm -o js-bindings

# call from Node.js
import { solve } from './js-bindings/teeline_wasm.js';
const result = solve('sa', cities, options);
```

See [docs/wasm.md](../docs/wasm.md) for the full interface reference and working examples in JavaScript, Python, Go, and Rust.

---
id: 019e31aa-108a-7153-ab9e-e723a72c7045
slug: decisions/adr-wasm-component-model
title: "ADR: WASM Component Model for multi-language solver exposure"
type: spec
status: active
priority: medium
---

# ADR: WASM Component Model for multi-language solver exposure

**Status:** Accepted
**Date:** 2026-05-16
**Implements:** GH #68

## Context

Teeline's solvers are pure compute — no I/O, no OS calls. Goal: expose them from Go, Python, JS, Rust, and Spin without a separate HTTP service and without requiring callers to understand Rust internals.

## Decision

Use the WebAssembly Component Model (`wasm32-wasip2`) with a WIT interface definition, built via `cargo-component 0.21.1`. The WIT file is the single source of truth for the API; consumers generate typed bindings automatically from it.

## Alternatives considered

| Option | Rejected because |
|--------|-----------------|
| `wasm-bindgen` (wasm32-unknown-unknown) | Browser/JS only; no Go/Python path |
| WASI stdin/stdout | No typed API; callers must parse text; no generated bindings |
| HTTP microservice (Spin HTTP handler) | Extra network hop; requires running server |

## Interface

Package `teeline:solver@0.1.0` — WIT at `teeline-wasm/wit/world.wit`.

```
solve(solver: string, cities: list<city>, options: solve-options) -> result<solution, string>
```

Supported solver names: `sa`, `simulated_annealing`, `2opt`, `two_opt`, `nn`, `nearest_neighbor`, `ga`, `genetic_algorithm`, `pso`, `particle_swarm`, `cs`, `cuckoo_search`, `fpa`, `flower_pollination`, `tabu_search`, `stochastic_hill`.

## Build

```bash
# build WASM component (from repo root)
cargo component build --manifest-path teeline-wasm/Cargo.toml --release

# inspect exported interface
wasm-tools component wit target/wasm32-wasip1/release/teeline_wasm.wasm

# transpile to JS for browser/Node.js
jco transpile target/wasm32-wasip1/release/teeline_wasm.wasm -o teeline-wasm/js-bindings/
cd teeline-wasm/js-bindings && npm install   # installs @bytecodealliance/preview2-shim
```

## Implementation notes

- `cargo-component 0.21.1` defaults to `wasm32-wasip1` target; `teeline-wasm/.cargo/config.toml` forces `wasm32-wasip2` (needed for wasip2-compliant runtimes and Spin)
- Bindings dep: `wit-bindgen-rt = { version = "0.44.0", features = ["bitflags"] }` (not `wit-bindgen` directly)
- Generated `Guest` trait lives at `bindings::Guest`; types at `bindings::teeline::solver::types::*`
- Artifact path: `target/wasm32-wasip1/release/teeline_wasm.wasm` (cargo-component names dir after underlying target)
- JS consumers: jco output requires `@bytecodealliance/preview2-shim` and `"type": "module"` in package.json

## Consequences

- `eframe` is now optional behind the `gui` feature in the main crate
- `ProgressMessage` moved to `src/tsp/messages.rs`; `progress.rs` re-exports it for compat
- `bhk` and `branch_bound` excluded — exponential complexity makes them unsafe for embedders
- WASI linker (`wasmtime_wasi::add_to_linker_sync`) required in any host that instantiates this component, because `rand` imports `wasi:random/random`
- Spin compatible (wasm32-wasip2 + wasmtime runtime)

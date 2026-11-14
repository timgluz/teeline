# Proposal

## Why

Teeline has fast constructive solvers (nearest-neighbor, greedy-edge, savings, Fourier) but no O(n log n) geometric constructor that exploits spatial locality without any pairwise distance work. A space-filling curve (Hilbert/Morton) tour is the classic Platzman–Bartholdi construction: it orders cities by position along a locality-preserving 1D curve, typically lands ~25% above optimal, and is an excellent warm start for 2-opt, Or-opt, and Lin–Kernighan — often a better and much cheaper seed than nearest-neighbor.

## What Changes

- Add a deterministic Space-Filling Curve constructive solver to the core crate, exposed as `sfc` with aliases `hilbert` and `space_filling_curve`.
- Support Hilbert and Morton curve indexing with a configurable grid order, normalization mode, rotation/mirror transforms, an optional best-of-orientation ensemble, and bucket tie-breakers.
- Wire the solver through every surface that enumerates solvers: metadata registries, TOML config stages, CLI flags, the `teeline-api` request models, the WASM bridge, and the web option plumbing.
- Emit the non-fatal pipeline composition warning when `sfc` appears after the first stage (it ignores a warm-start seed, like other constructive solvers).
- Add unit, integration, documentation, and benchmark coverage.

No breaking changes.

## Capabilities

### New Capabilities

- `tour-construction`: Constructive tour-building strategies that turn city geometry into a complete tour, including the new space-filling-curve solver and its parameters.

### Modified Capabilities

<!-- None: `openspec/specs/` is empty, so no existing capability changes. -->

## Impact

- Core: `src/tsp/space_filling_curve.rs` (new), `src/tsp/mod.rs`, `src/tsp/pipeline.rs`, `src/config.rs`
- CLI: `teeline-cli/src/main.rs`
- API: `teeline-api/src/models/request.rs`, `teeline-api/src/services/tsp_service.rs`, `teeline-api/src/openapi.rs`
- WASM: `teeline-wasm/src/lib.rs`, `teeline-wasm/tests/wasm_component.rs`
- Web: `teeline-web/src/teeline-wasm.d.ts`, `teeline-web/src/solver-options.ts`
- Tests: inline unit tests and `tests/space_filling_curve_test.rs`
- Docs: `README.md`, `CLAUDE.md`/`AGENTS.md`, `docs/algorithms/space-filling-curve.md`
- Bench: `bench/solvers.json` and the baseline/post TSVs if the harness registers solvers there
- Verification: TSPLIB instances only, fewer than 2,000 cities and with published optima (e.g. berlin52, att532); larger-instance work is tracked in #647

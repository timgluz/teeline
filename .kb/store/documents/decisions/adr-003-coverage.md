---
id: 019e2745-c75c-7343-ba85-11a67407fb08
slug: decisions/adr-003-coverage
title: "ADR-003: Test coverage via cargo-llvm-cov"
type: brief
status: active
tags: [testing, coverage, ci]
---

## Context

The Teeline project had uneven test coverage: 4 solvers (`nearest_neighbor`, `branch_bound`, `tabu_search`, `stochastic_hill`) had zero unit tests, and only a single integration test existed. CI had no coverage gate, making it impossible to track or enforce coverage over time.

## Decision

Use `cargo-llvm-cov` for test coverage measurement.

- Installed in CI via `taiki-e/install-action@cargo-llvm-cov` (downloads pre-built binaries, avoids slow `cargo install` compile)
- Generates LCOV output for Codecov upload
- Coverage report uploaded via `codecov/codecov-action@v4` using `CODECOV_TOKEN` secret

## Alternatives Considered

- **`cargo-tarpaulin`**: Linux-only, uses ptrace for instrumentation. Slower and requires special CI permissions. Cross-platform projects can't use it.
- **`grcov`**: Requires separate LLVM source-based coverage flags and post-processing; more complex setup for marginal benefit over llvm-cov.

## Consequences

- `cargo llvm-cov` requires the `llvm-tools-preview` rustup component; `taiki-e/install-action` handles this automatically.
- Baseline coverage achieved: **79% line coverage** across `src/tsp/` (excluding `progress.rs` which is UI-thread code).
- Target gate: ≥ 70% line coverage on `src/tsp/`.

## Tests Added

### Unit tests (new)
- `nearest_neighbor.rs` — 3 tests: all cities visited, finite tour, collinear cities
- `branch_bound.rs` — 3 tests: all cities visited, optimal tour on tsp5, matches BHK
- `tabu_search.rs` — 6 tests: TabuList add/contains/eviction/capacity, solver tour validity, positive length
- `stochastic_hill.rs` — 3 tests: all cities visited, finite tour, respects epoch limit

### Integration tests (`tests/solvers_integration.rs`)
- 10 tests covering all 8 solvers on real TSPLIB data
- Heuristics tested on berlin52 (52 cities)
- Exact solvers tested on tsp5 hand-crafted instance (optimal = 4.0)

---
id: 019ec00f-ab67-7dc0-8c4b-4d222e99743b
slug: tasks/gh-185-fourier-solver
title: "GH #185: Fourier-basis constructive TSP solver"
type: task
status: completed
priority: high
tags: [solver, fourier, constructive]
---

## Overview

Implement a Fourier-basis constructive TSP solver (GH #185). Encodes a tour as a closed
curve in the complex plane and optimises the Fourier coefficients with gradient descent.
`argsort` decode always produces a valid permutation — no penalty terms or validity repair.

## Goals

- Add `src/tsp/fourier.rs` with coarse-to-fine gradient descent and argsort decode
- `FourierOptions` struct with `k_max`, `m`, `lambda`, `lambda_decay`, `lr`, `epochs`
- Register as `"fourier"` in enum, FromStr, dispatch, SOLVER_LIST, all_meta()
- WASM integration in this PR
- TDD: failing tests committed before implementation

## Acceptance Criteria

- [x] `num-complex = "0.4"` added to Cargo.toml
- [x] `FourierOptions` in mod.rs with Default, validate(), from_toml(), from_cli()
- [x] `src/tsp/fourier.rs` implements coarse-to-fine gradient descent + argsort decode
- [x] decode_tour maps argsort positions to city IDs (not array positions)
- [x] Unit tests: decode of 5-city circle valid; gradient step reduces energy; non-contiguous IDs
- [x] Integration tests in tests/fourier_test.rs (fast + ignored quality)
- [x] Registered in SOLVER_LIST (17→18) and all_meta() — both registries
- [x] WASM: build_opts arm, recommendation, params_for_solver, kind_for, wasm test count 17→18
- [x] docs/algorithms/fourier.md created
- [x] docs/benchmarks.md updated with fourier + fourier+2opt rows
- [x] CLAUDE.md solver table updated
- [x] cargo test passes; cargo clippy clean

### 2026-10-11 — completed, verified against master file by file

The criteria were never ticked, but every one of them is present: `num-complex = "0.4"` in
`Cargo.toml`; `FourierOptions` in `src/tsp/mod.rs` (15 references, including the registries);
`src/tsp/fourier.rs` with `decode_tour` and 4 unit tests; `tests/fourier_test.rs`;
`docs/algorithms/fourier.md`; 7 Fourier references on the WASM side; 5 `fourier` rows in
`docs/benchmarks.md`; the solver table in `AGENTS.md`. `cargo test`/`clippy` are covered by master's
CI, which is green. The explainer and the write-up also shipped (`/algorithms/fourier/`, and
`teeline-web/src/content/blog/fourier-algorithm-how-it-came-to-be.md`).

Reopen if a criterion in the body turns out to have been delivered only in part — the evidence above
is file-level, not a run of the ignored quality test.

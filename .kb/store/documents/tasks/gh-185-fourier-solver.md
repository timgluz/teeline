---
id: 019ec00f-ab67-7dc0-8c4b-4d222e99743b
slug: tasks/gh-185-fourier-solver
title: "GH #185: Fourier-basis constructive TSP solver"
type: task
status: active
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

- [ ] `num-complex = "0.4"` added to Cargo.toml
- [ ] `FourierOptions` in mod.rs with Default, validate(), from_toml(), from_cli()
- [ ] `src/tsp/fourier.rs` implements coarse-to-fine gradient descent + argsort decode
- [ ] decode_tour maps argsort positions to city IDs (not array positions)
- [ ] Unit tests: decode of 5-city circle valid; gradient step reduces energy; non-contiguous IDs
- [ ] Integration tests in tests/fourier_test.rs (fast + ignored quality)
- [ ] Registered in SOLVER_LIST (17→18) and all_meta() — both registries
- [ ] WASM: build_opts arm, recommendation, params_for_solver, kind_for, wasm test count 17→18
- [ ] docs/algorithms/fourier.md created
- [ ] docs/benchmarks.md updated with fourier + fourier+2opt rows
- [ ] CLAUDE.md solver table updated
- [ ] cargo test passes; cargo clippy clean

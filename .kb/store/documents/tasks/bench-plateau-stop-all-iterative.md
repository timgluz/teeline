---
id: 01a12651-55d5-7f71-84c7-360c3a0086fa
slug: tasks/bench-plateau-stop-all-iterative
title: "feat(tsp): shared plateau stop for all iterative solvers"
type: task
status: active
priority: high
tags: [solver, benchmarks, convergence]
blocked_by: [tasks/bench-sa-honours-epoch-budget]
---

## Overview

Only two solvers implement a convergence criterion: `lin_kernighan` and `stochastic_hill`
(`platoo_epochs`, default 500 — stop after N consecutive non-improving iterations). The other
iterative solvers have no stopping rule at all, and several (`aco`, `cs`, `fpa`, `gsa`) default
to `epochs: 0`, which currently means "forever".

A convergence result is therefore not a measurable event today, which blocks the agreed
benchmark basis of running each algorithm both at a budget and to convergence
(`decisions/benchmarking-design`, decisions 1 and 2).

## Scope

Add a shared plateau stop, reusing the existing `platoo_epochs` semantics, to the 10 iterative
solvers that lack one:

`sa`, `ga`, `pso`, `cs`, `fpa`, `gsa`, `aco`, `fourier`, `som`, `tabu_search`

**Not** required for:

- one-shot constructors — `nn`, `christofides`, `greedy_edge`, `savings` (nothing to converge)
- already-exact local searches — `2opt`, `3opt`, `or_opt` (they terminate at a local optimum)
- exact solvers — `bhk`, `branch_bound`
- `shuffle` — a utility baseline, not an algorithm

## Goals

- One shared implementation rather than ten copies of the same counter
- Expose the stopping epoch as reportable data, so "converged at epoch N" is observable
- Define what `epochs: 0` means everywhere (it currently means infinite for four solvers)

## Acceptance Criteria

- [ ] Shared plateau helper exists and is used by all iterative solvers
- [ ] Each of the 10 listed solvers stops on sustained non-improvement
- [ ] The stopping epoch is observable (log line at minimum) rather than inferred from runtime
- [ ] `epochs: 0` semantics defined and consistent across solvers
- [ ] Per-solver test that a converged run terminates before its safety cap
- [ ] No solver regresses in solution quality at equivalent budgets
- [ ] `cargo test`, `cargo clippy --workspace -- -D warnings` clean
- [ ] `docs/algorithms/*.md` termination wording updated where it asserts otherwise

## Notes

Prerequisite for the benchmarking campaign. Coordinate with
`tasks/bench-sa-honours-epoch-budget`: SA is in both scopes.

## Progress Log

### 2026-10-10

- **Foundation merged** as [#566](https://github.com/timgluz/teeline/pull/566) (merge commit
  `a1cdb103`). Three review passes, 23 findings, all addressed. Outcomes worth carrying forward:
  `stagnation_epochs` defaults to **0 (disabled)** so existing API callers' runs are unchanged;
  the test-only epoch counter was replaced by a testable `Budget` type; `teeline-wasm` and
  `teeline-qt` both needed the new field — both sit outside the Cargo workspace, so no `--workspace`
  command compiles them.
- **Solvers wired so far (5 of 10):** `ga` (#566), plus `pso`, `cs`, `fpa`, `gsa`.
- **Remaining (5):** `sa`, `aco`, `fourier`, `som`, `tabu_search`.

The four mechanics share one treatment: `for epoch in 0..epochs` becomes
`while budget.record(improved)`, with improvement decided by one comparison of the best across the
epoch. My first attempt marked `improved` at each improvement site instead, which is wrong for a
solver with several such sites (`cuckoo_search` has two) — `clippy`'s `unused_assignments` caught it.

**Still to do beyond wiring:** per-solver convergence tests, migrating `lin_kernighan` off
`platoo_epochs`, removing `simulated_annealing`'s test-only `ITERATIONS` counter, and updating the
solver docs.

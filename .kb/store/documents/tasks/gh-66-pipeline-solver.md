---
id: 019e4612-51df-7853-a829-5c3805f56f1d
slug: tasks/gh-66-pipeline-solver
title: "Pipeline Solver: Chain Multiple Solvers with Warm-Start Seeding (GH #66)"
type: task
status: draft
priority: high
tags: [feature, cli, solver, pipeline, rust]
---

## Overview

Every TSP solver in Teeline currently seeds from scratch — sequential city order for
most single-tour solvers, random for population-based ones, or an embedded internal
NN call for 3-opt, PSO, cuckoo search, and FPA. This hidden per-solver seeding logic
is inconsistent, untestable in isolation, and produces suboptimal starting points
for local-search algorithms (SA starts sequential, 2-opt starts sequential, GA is
fully random).

This task implements a `pipeline` mechanism that makes warm-start seeding explicit
and composable. The key insight: **each solver becomes a pure algorithm**; pre-seeding
is the pipeline's responsibility. The CLI auto-expansion makes this transparent to
users — `teeline solve sa` silently runs `pipeline(nn, sa)` without the user needing
to know.

Closes GitHub issue #66. Related follow-up: [[tasks/gh-79-ga-seeding]] (stronger GA
warm-start strategies, tracked separately).

## Goals

- Add `pipeline::solve(steps, cities, distances, options)` library function that
  chains any list of solvers, threading each stage's best tour as `initial_tour` to
  the next stage
- Add `initial_tour: Option<Vec<usize>>` to `SolverOptions`; no other pipeline state
  in options
- Remove all embedded NN seeding from solver internals (3-opt, PSO, CS, FPA)
- Auto-expand `teeline solve <local-search-solver>` to `pipeline(nn, solver)` for
  all 9 local-search solvers; add `--no-seed` opt-out flag
- Add explicit `teeline pipeline --steps=nn,2opt,sa` subcommand for custom chains
- Add named presets: `classic` (nn+2opt+sa), `fast` (nn+2opt), `thorough` (nn+3opt+sa)

## Implementation

Plan file: `.claude/plans/linked-wiggling-book.md` — full task-by-task breakdown with
exact code snippets.

### Architecture summary

**No `Solvers::Pipeline` variant** — `pipeline::solve()` is called directly from
`main.rs`, preventing the recursion problem and `FromStr` footgun identified in
Opus review.

**`SolverOptions` gains one field:**
```rust
pub initial_tour: Option<Vec<usize>>,
```
Plus two helpers:
- `validate_tour(tour, cities) -> Result<(), String>` — called once at pipeline entry
- `for_internal_seed(&self) -> SolverOptions` — clears `initial_tour` + `progress_tx`
  for sub-solver calls (used by 3-opt's fallback NN path)

**`pipeline::solve()` loop:**
- Validates seed once at entry (not in every solver)
- Uses `seed.take()` to avoid N-1 `Vec<usize>` clones across stages
- Returns last stage's `Solution` directly (no recompute)
- Stage-indexed error messages: `"pipeline stage {i} ({solver:?}): {e}"`

**CLI auto-expansion (`main.rs`):**
- `Solvers::auto_expand_with_nn() -> bool` — true for all 9 local-search solvers
- `resolve_preset(name) -> Option<&'static [Solvers]>` — classic/fast/thorough
- `run_as_pipeline(steps, args)` — shared helper used by both `run_solve` and
  `run_pipeline` (eliminates ~70% code duplication)
- Preset names added to `Solvers::variants()` so clap accepts them

**Solver changes (9 files):**
- single-tour (2opt, sa, tabu): `initial_tour.clone().unwrap_or_else(sequential)`
- stochastic_hill: skip shuffle when seeded
- GA: `TspPopulation::from_cities_seeded()` — seeds `n/10` individuals (1 exact + mutants)
- PSO/CS/FPA: remove `nn_seed` special case for index 0; use `initial_tour` or random
- 3-opt: remove internal NN call; use `initial_tour` or sequential; use `for_internal_seed()`

## Files to Create / Modify

| File | Change |
|------|--------|
| `src/tsp/mod.rs` | `initial_tour` field; `validate_tour`; `for_internal_seed()`; `auto_expand_with_nn()`; preset names in `variants()`; `pub mod pipeline` |
| `src/tsp/pipeline.rs` | **CREATE** |
| `src/main.rs` | `pipeline` subcommand; `--no-seed`; `run_as_pipeline`; `resolve_preset`; auto-expansion in `run_solve` |
| `src/tsp/two_opt.rs` | Respect `initial_tour` |
| `src/tsp/simulated_annealing.rs` | Respect `initial_tour` |
| `src/tsp/stochastic_hill.rs` | Respect `initial_tour`, skip shuffle when seeded |
| `src/tsp/tabu_search.rs` | Respect `initial_tour` |
| `src/tsp/genetic_algorithm.rs` | `TspPopulation::from_cities_seeded()` |
| `src/tsp/particle_swarm.rs` | Remove `nn_seed`, use `initial_tour` or random |
| `src/tsp/cuckoo_search.rs` | Remove `nn_seed`, use `initial_tour` or random |
| `src/tsp/flower_pollination.rs` | Remove `nn_seed`, use `initial_tour` or random |
| `src/tsp/three_opt.rs` | Remove internal NN call; `for_internal_seed()` |
| `tests/solvers_integration.rs` | Pipeline integration tests |

## Acceptance Criteria

- [ ] `pipeline::solve(&[NN, TwoOpt], ...)` on berlin52 produces a valid tour with
  `total <= nn_alone.total` (2-opt cannot worsen a tour)
- [ ] `teeline solve sa -i berlin52.tsp` runs `pipeline(nn, sa)` transparently
- [ ] `teeline solve sa --no-seed -i berlin52.tsp` runs plain SA from sequential order
- [ ] `teeline solve classic -i berlin52.tsp` runs `pipeline(nn, 2opt, sa)`
- [ ] `teeline pipeline --steps=nn,2opt,sa -i berlin52.tsp` produces valid tour
- [ ] `teeline solve nn -i berlin52.tsp` does NOT auto-expand (NN is a constructor)
- [ ] No `nn_seed` or `nearest_neighbor::solve` call remains in 3-opt, PSO, CS, FPA
  except in the `for_internal_seed()` fallback path of 3-opt
- [ ] Each modified solver has a `test_<solver>_respects_initial_tour` unit test
- [ ] `cargo test` all pass; `cargo clippy -- -D warnings` clean
- [ ] Benchmark rows added for `nn+2opt` and `nn+3opt` in `data/benchmarks.md`

## Key Decisions (Opus-reviewed)

| Decision | Choice |
|----------|--------|
| `Solvers::Pipeline` variant | No — `pipeline::solve()` called directly |
| `pipeline_steps_typed` in SolverOptions | No — pipeline takes `steps: &[Solvers]` |
| `validate_tour` placement | Once at pipeline entry, `debug_assert!` in solvers |
| GA seeding depth | `n/10` individuals as baseline; GH #79 tracks improvement |
| Solver-internal NN seeding | Removed — pipeline provides it via auto-expansion |
| `run_as_pipeline` deduplication | Same PR, not deferred |

## Out of Scope (Follow-up Issues)

- Stronger GA warm-start seeding — tracked in GH #79 / [[tasks/gh-79-ga-seeding]]
- Per-stage option overrides (`--steps=nn,sa:epochs=5000`)
- `--initial-tour <FILE>` to read `.opt.tour` seed
- Restart semantics for stochastic_hill (K restarts, best-of-K)
- GUI stage-boundary events (`ProgressMessage` variants)

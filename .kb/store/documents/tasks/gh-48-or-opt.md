---
id: 019ebc73-3281-7360-803c-06dbaded69e8
slug: tasks/gh-48-or-opt
title: "feat: implement Or-opt local search solver (GH #48)"
type: task
status: completed
priority: high
---

## Overview

Implement Or-opt as a new `Solvers::OrOpt` variant (alias `"or_opt"`). Or-opt relocates sequences of 1, 2, or 3 consecutive cities to a better position in the tour — a relocation move vs 2-opt's reversal. It catches improvements 2-opt misses and works as a post-processing pass on top of other solvers.

Branch: `feat/gh-48-or-opt`

## Goals

- Implement all three move sizes: Or-1, Or-2, Or-3
- Best-improvement per pass (scan all relocations, apply best, repeat)
- All three sizes together in each pass
- Reversed insertions for Or-2 and Or-3 (symmetric matrix assumption)
- Correctness and idiomatic Rust over raw perf

## Acceptance Criteria

- [x] `Solvers::OrOpt` variant, aliased as `"or_opt"` and `"or-opt"`
- [x] `src/tsp/or_opt.rs` implements all three move sizes
- [x] Best-improvement strategy (scan all, apply best)
- [x] Produces tours competitive with 2-opt on berlin52 and att48
- [x] Uses existing `Solution` type; no `unsafe` code
- [x] Unit tests for each move size on small hand-constructed tours
- [x] Integration test on berlin52
- [x] WASM solver count updated to 15
- [x] `cargo clippy -- -D warnings` clean

## Completion Evidence

- Commit: 8628930 — feat: implement Or-opt local search solver (GH #48)
- All 251 unit tests + all integration tests pass
- Clippy clean (`-D warnings`)
- berlin52: Or-opt 8097 vs 2-opt 8384 (Or-opt wins), optimal 7542
- att48: Or-opt 34862 vs 2-opt 34577 (~0.8% behind, different local optima)
- WASM solver count: 14 → 15

## Key Design Decisions

- **Signature**: `pub fn solve(problem: &TspProblem, opts: &HeuristicOptions, progress_tx: Option<&mpsc::Sender<ProgressMessage>>, init_tour: Option<&[usize]>) -> Solution`
- **Distance API**: `problem.distances.distance_between(city_id_a, city_id_b)`
- **Threshold**: `-1e-3` (not `-1e-6` — f32 ULP at large coordinate scale)
- **apply_relocation**: `Vec::splice` with index adjustment after drain
- **Exclusion range**: forbid j in `{i-1, i, ..., i+seg_len-1}` mod n
- **Debug assertion**: recompute tour_length to verify delta on each applied move

## Files

| File | Change |
|------|--------|
| `src/tsp/or_opt.rs` | New |
| `src/tsp/mod.rs` | Enum, variants, from_str, dispatch, auto_expand_with_nn |
| `tests/or_opt_test.rs` | New |

## References

- [[tasks/gh-45-lin-kernighan]] — prior algorithm work (pattern to follow)
- GH issue #48

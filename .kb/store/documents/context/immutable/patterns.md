---
id: 019e2683-384c-7543-9573-cfc2edc3b01b
slug: context/immutable/patterns
title: "System Patterns"
type: patterns
status: active
priority: medium
---

## Solver Dispatch Pattern

Every algorithm is a variant of the `Solvers` enum in `src/tsp/mod.rs`. Adding a new solver requires:
1. Add variant to `Solvers` enum
2. Add string alias(es) to `Solvers::variants()` and `FromStr` impl
3. Create `src/tsp/<solver_name>.rs`
4. Add dispatch arm in `src/main.rs`
5. Add integration test in `tests/`

## Data Flow

```
stdin / file → tsplib parser → Vec<KDPoint> → solver → Solution → stdout
                                                   ↓
                                         progress channel → Piston window (separate thread)
```

## Spatial Query Pattern

All nearest-neighbour lookups go through `KDTree` (preferred) or `DistanceMatrix` (brute-force fallback). Both expose `nearest(&pt, n)` returning `NearestResult`. New solvers that need candidate edge lists should use `KDTree`.

## Solution Type

`Solution` holds `route: Vec<usize>` (city IDs), `cities: Vec<KDPoint>`, and `total: f32`. Call `update_total()` after any route mutation. City IDs are 1-based (TSPLIB convention).

## Testing Pattern

- Unit tests: inline `#[cfg(test)]` in each solver file
- Integration tests: in `tests/`, import `teeline` crate, run on TSPLIB files
- Quality check: compare output against known optima (berlin52=7542, att48=10628, bayg29=1610)
- Approximate float comparison: use `test::helpers::assert_approx`

## Progress Reporting

Solvers send `ProgressMessage` structs via an `mpsc` channel to the Piston window thread. Use `try_send` (non-blocking) to avoid stalling the solver.

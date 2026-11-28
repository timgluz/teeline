# Design

## Context

See `proposal.md` — Why, and the two spec deltas for the behaviour contracts. The current
state that shapes this design:

- `src/tsp/mod.rs` is the single registration point: `Solvers` enum, `variants()`,
  `FromStr`, `SolverMeta`, the fixed-size `SOLVER_LIST`, and the `solve_with_context`
  dispatch. Solver registration is a constitution-level guardrail; missing one is a defect.
- `src/tsp/bellman_karp.rs` already implements a Held–Karp DP, but as a memoized recursive
  `solve_bhk` plus a separate `read_optimal_route` reconstruction that compares a
  decremented running total with a relative tolerance. It is exact, but the recurrence is
  entangled with the solver, so a second solver would duplicate it.
- `DistanceSource` exposes `distance_by_pos`, `tour_length` and `tour_length_by_pos` (the
  latter closes the cycle). There is no open-path cost helper.
- `nearest_neighbor::solve` is the deterministic greedy constructor; reusing it is the
  intended way to obtain a seed.
- `Position` is a dense index newtype; `Solution::from_parts` builds the reported tour.
- The benchmark harness runs the CLI, so a solver with an automatic `nn` prefix is measured
  as `pipeline(nn, popmusic)`.
- The website's `nav-data.test.ts` cross-checks `ALGORITHM_COUNT` against
  `bench/solvers.json` entries with a `doc`, and the `SOLVER_META` complexity string
  against the docs page `| **Complexity** |` row. The solver page, the nav entry and the
  benchmark index must land together.

## Goals / Non-Goals

**Goals:**

- One shared, brute-force-verified exact path DP used by BHK and POPMUSIC.
- A deterministic POPMUSIC implementation with fixed constants, no RNG and no new options
  surface.
- Keep the DP cheap enough for on-the-fly backends (the Tier A `tier-distance-large`
  corpus includes `pla7397`).
- Preserve BHK's exact optimal cost while removing its duplicate recurrence.

**Non-Goals:**

- Exposing `R` or `MAX_PASSES` through CLI/TOML/API/WASM (issue #770 defers options until
  the benchmark shows a need).
- Non-contiguous windows, or using `3opt`/`lk` as the window sub-solver.
- A new benchmark family: POPMUSIC registers as `meta` for v1.
- Changing BHK's `EXACT_MAX_CITIES` bound or its output format.

## Decisions

### 1. One shared fixed-endpoint Held–Karp path DP

Add `src/tsp/held_karp.rs`:

```rust
pub(crate) fn held_karp_path(matrix: &[f32], m: usize) -> Option<(f32, Vec<usize>)>
```

`matrix` is a packed row-major `(m + 2)²` table over local indices: `0..m` are the
interior nodes, `m` is the start and `m + 1` is the end. The return value is the open-path
cost `start → … → end` and the interior visit order as local indices.

- **Why a matrix, not `&dyn DistanceSource`.** At `R = 14` a window's DP performs roughly
  `m² · 2^m ≈ 600k` distance reads. A `(m + 2)² = 196`-entry local table turns those into
  array indexing, so an on-the-fly backend (e.g. `pla7397`) pays 196 dynamic queries per
  window instead of 600k. It also makes the brute-force oracle a pure function of a raw
  matrix.
- **Why local indices + endpoints.** Both callers need a different node set; keeping the
  primitive free of city IDs and `Position` makes it trivially testable and keeps the
  position/city-ID boundary in the callers, as the codebase requires.
- **DP shape.** Iterative over masks in increasing order; `dp[mask][last]` is the cheapest
  path from `start` through exactly `mask`, ending at `interior[last]`. Base
  `dp[1 << i][i] = d(start, i)`; answer `min_last dp[full][last] + d(last, end)`. Unreached
  states are `INFINITY` (not `f32::MAX`, which would round back to itself when a finite
  distance is added).
- **Reconstruction.** Walk backward from `(full, best_last)`, recomputing the argmin over
  `prev` with the same first-strict-`<` tie-break the forward pass used. This always finds
  a predecessor, needs no parent table (keeping BHK's memory at `m · 2^m`), and avoids the
  running-subtraction tolerance that made the old `read_optimal_route` fragile.
- **Bound.** Assert `m <= 26` before allocating `m · 2^m` entries. BHK is already capped at
  24 cities, so its `m = 23` fits unchanged (~772 MB at the extreme, the same as today).

### 2. Refactor BHK onto the primitive

`bellman_karp::solve` builds the `(n + 1)²` local matrix (local `i < n - 1` maps to
`Position::new(i)`; local `n - 1` and `n` both map to the anchor `Position::new(n - 1)`),
calls `held_karp_path(matrix, n - 1)`, and builds `route = [anchor] ++ interior`. The
`CityChange`/`PathUpdate`/`Done` progress messages remain.

- **Observable change.** The route is now emitted in forward DP order (the old reconstruction
  emitted it reversed), and equal-cost tie-breaking may choose a different but equally
  optimal cycle. Cost is unchanged. This is a behaviour change to a documented solver, so
  `docs/algorithms/bellman-held-karp.md` gets a version bump and a changelog row, including
  the removal of the `--verbose` DP-table dump (`show_table`).
- **De-risking.** Before the refactor, capture the old cost and route on `burma14`, `gr17`,
  `ulysses16` and a few random 3–12-city instances; after it, assert identical cost and a
  valid permutation. The DP itself is independently verified against brute force.

### 3. POPMUSIC window size and small-input guard

`R = min(14, n)` and `m = R - 2`. If `m < 2` the solver returns the seed unchanged. The
issue text said `min(14, n - 2)`, but that wastes the window on small inputs: at `n = 5` it
gives `R = 3` and a single interior city, so nothing can be reordered. `R = n` is valid for
small instances because `u` and `v` are distinct and stay joined by the fixed closing edge.

### 4. Two-offset scan, pass semantics, wrap-around

- Offsets `[0, R / 2]`, stride `R` within each offset; the two offsets together are one
  pass; at most `MAX_PASSES = 4` passes; stop as soon as a pass makes no splice.
- Windows use modular indices `tour[(start + k) % n]` and are written back into the same
  slots. **No tour rotation**: a splice only permutes cities within fixed slots, so later
  window indices stay valid; rotating would stale every later offset in the pass.
- The final window wraps and overlaps the first.

### 5. Improvement test: same-order sum, epsilon, skip identity

The DP accumulates edge costs in path order, so the current window's open-path cost is
computed as a left-to-right `f32` sum in the same order. An unchanged interior order then
compares bit-identically. A splice is applied only when the candidate order differs and
`candidate < current - eps`, with `eps = 1e-6 · max(current, 1)`.

- **Why.** Two different interior orders with the same true cost can differ by one `f32`
  ulp; a strict `<` would accept a "gain" that is not real, keep the early stop from firing,
  and let overlapping windows trade ulps back and forth.

### 6. Seeding without duplication

The solver consumes `init_tour` when present and otherwise calls
`nearest_neighbor::solve(problem, &HeuristicOptions::default(), None, None)` and takes its
route. It does **not** re-implement greedy NN, and it passes `None` for NN's progress
channel so POPMUSIC's own `PathUpdate`/`Done` stream is the only one on the channel.

### 7. No new options surface

`R` and `MAX_PASSES` stay module constants; `solve` accepts `&HeuristicOptions` for the
uniform solver signature but ignores it, like `nearest_neighbor`. There is no
`PopmusicOptions`, no `AppOptions` field, no CLI flag and no TOML table. The API's
`make_app_options` fallback and the WASM `build_opts` fallback already cover it.

### 8. Registration and cross-surface coupling

Register in all six registries plus `auto_expand_with_nn`. The web side lands as one unit:
`bench/solvers.json` (`family: "meta"`, `doc: "popmusic"`), `nav-data.ts`
(`SOLVER_META.popmusic` with the same complexity string as the page, plus the
Metaheuristic group) and `docs/algorithms/popmusic.md`. `hasExplainer: false`, so
`EXPLAINER_SOLVERS` and the explainer registry are untouched. `deterministic` is set to
`true` only after measurement records `determinism_evidence`, per the benchmark schema.

## Risks / Trade-offs

- **BHK regression during the refactor** → brute-force verification of the DP, before/after
  cost capture, and the existing BHK unit/integration/conformance tests.
- **False improvements from `f32` accumulation** → same-order sum plus the `eps` guard and
  the idempotence test.
- **Wrap-around splice bug** → explicit modular-index unit tests with a bad segment placed
  across index 0.
- **On-the-fly runtime** → the per-window local matrix; the DP never calls the distance
  source inside its inner loop.
- **Benchmark claim without evidence** → `deterministic` is measured, and the PR records the
  chosen `R` and window/pass timings.
- **BHK output change surprises downstream** → documented in the BHK page changelog; the
  output contract (cost + ordered IDs) and the optimal cost are unchanged.

## Migration Plan

1. Land the shared DP + BHK refactor behind the existing tests; no data or API migration.
2. Add POPMUSIC, then registration, then docs/web/bench in the same branch.
3. Run the Tier A `meta` sweep and publish through the existing benchmark script/skill.
4. Rollback is a branch revert; the solver is additive and the BHK change is
   behaviour-preserving in cost.

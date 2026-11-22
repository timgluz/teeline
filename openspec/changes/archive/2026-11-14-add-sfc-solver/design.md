# Design

## Context

See `proposal.md` — Why. The relevant current state:

- `src/tsp/mod.rs` holds **six** solver registries that must stay in sync: the `Solvers` enum, `variants()`, the `SolverMeta` list, the UI-facing `SOLVER_LIST` (a fixed-size `[SolverInfo; 22]`), `FromStr`, and the `solve_with_context` dispatch.
- Constructive solvers are `solve(problem, &opts, tx, init_tour)` and ignore `init_tour`. Parameter-free ones take `HeuristicOptions`; option-bearing ones get a dedicated struct (`FourierOptions`, `SOMOptions`, `AcoOptions`, …) carried on `AppOptions`.
- `src/tsp/pipeline.rs::stage_warnings` already flags `GreedyEdge` and `Savings` when they appear after stage 0 because they discard a seed.
- Progress is streamed as `ProgressMessage::PathUpdate`/`Done`; constructive solvers without intermediate state (e.g. greedy-edge, christofides) emit a placeholder update.
- `DistanceMatrix` requires at least two points, so a 1-city instance cannot normally reach a solver; guards are still defensive.
- `budget.rs` is an **epoch/convergence** budget for iterative solvers — it is not a wall-clock limiter.

## Goals / Non-Goals

**Goals:**

- A deterministic, O(n log n), parameterized SFC constructor that is correct on the degenerate cases and fast on the TSPLIB instances this repo verifies (fewer than 2,000 cities with published optima).
- One coherent options path (CLI, TOML, API, WASM, web) so the solver behaves identically everywhere.
- Tests that would fail if the curve geometry is wrong, not just tests that the tour is a permutation.

**Non-Goals:**

- The interactive `teeline-web` explainer (curve overlay, order slider, seam highlight) — separate follow-up issue.
- Continuous pre-rotation angle θ.
- Any change to existing solvers' behavior.

## Decisions

### 1. Dedicated `SfcOptions`, modeled on `FourierOptions`

Add `SfcOptions` (Clone, Debug, PartialEq, Default, `validate`, `from_toml`, `from_cli`) and `AppOptions.sfc: Option<SfcOptions>`. `HeuristicOptions` is deliberately not extended: it is documented as the home for parameter-free constructors, and adding SFC fields would leak them into every solver. Alternatives considered: reuse `HeuristicOptions` (rejected — muddies the per-solver options contract), or hard-code parameters (rejected — the issue requires CLI/TOML configurability).

### 2. Hilbert indexing: Lam–Shapiro `xy2d`, verified against a reference

Implement the standard iterative `xy2d` with the full-grid reflection (`n - 1 - x`), using `u64` internally so `p = 16` (`2^32` cells) cannot overflow. The pseudocode called out in the issue (`s - 1 - x`) is not used; correctness is guarded by:

- bijection test over all `4^p` cells for `p = 1..=6`,
- Manhattan-adjacency test for consecutive indices for `p = 1..=6`,
- a small-`p` cross-check against a known reference ordering.

Alternatives considered: recursive Butz decomposition (equivalent, more error-prone for the same tests). Morton is implemented independently by bit interleaving; it intentionally has long diagonal jumps and is offered as a contrast.

### 3. Normalization in `f64`, clamped to the grid

Compute the bounding box from `f32` coordinates but do the arithmetic in `f64`. `uniform` divides both axes by the larger span (preserves aspect); `stretch_to_square` divides each axis by its own span (fills elongated maps). Both scale to `2^p - 1`, round to the nearest cell, and clamp to `[0, 2^p - 1]` to avoid the max-edge off-by-one. A zero span uses an `EPSILON` guard so identical or collinear cities collapse deterministically.

### 4. Symmetry transforms and bounded orientation ensemble

Apply rotation/mirror on integer grid coordinates before indexing (`rotation ∈ {0,90,180,270}`, `mirror: bool`). Multi-orientation enumerates at most 8 transforms (4 rotations × 2 mirror), builds one tour per orientation, and keeps the shortest. This is at most 8 sorts, so it needs no `budget.rs` integration — that type governs iterative epoch/convergence, not wall-clock, and adding it here would be misuse.

### 5. Deterministic bucket tie-breakers

`input_order` uses the original index; `lexicographic` uses the city's original
coordinates (grid coordinates are equal within a cell, so they cannot distinguish
ties); `nearest_prev` greedily picks the bucket city nearest the last emitted city,
falling back to input order on ties. Buckets larger than 64 cities keep input order,
bounding the greedy scan so the solver stays O(n log n) even on heavily clustered
input. No RNG is used anywhere.

### 6. Public `solve` sanitizes invalid options

`solve` is public and `SfcOptions` fields are public, so a direct caller can bypass
`solve_with_context`'s strict validation. Rather than panicking in `transform_grid`
or the grid shift, `solve` clamps `order` to `1..=16`, `rotation` to
`{0, 90, 180, 270}`, and `multi_orientation` to `0..=8`, logging a warning. The
CLI/TOML/API paths still reject invalid values with an error.

### 7. Register on every surface, and test the registries

Because `teeline-wasm` is outside the workspace, exhaustive-match drift is a known hazard (see the `DistanceType::as_str` comment). Add the solver to: the `Solvers` enum, `variants()`, `SolverMeta`, `SOLVER_LIST` (bumping `22 → 23`), `FromStr`, dispatch, `AppOptions`, `config.rs` stage fields, CLI flags, `teeline-api` request/mapping/OpenAPI, WASM description/category/params/options, and web option plumbing. Registry round-trip tests plus the existing `variants_all_round_trip_through_fromstr` test cover the core.

### 8. Pipeline semantics

SFC ignores `init_tour` (constructive), so it is added to the `i > 0` arm of `stage_warnings`, matching `GreedyEdge`/`Savings`. It is **not** added to `auto_expand_with_nn` or `auto_expand_with_shuffle`: it does not consume a seed and is deterministic. It remains valid as a first stage and as a warm-start producer for `2opt`/`or_opt`/`lk`.

### 9. Verification is limited to TSPLIB instances under 2,000 cities with published optima

`TspProblem` bundles a `DistanceMatrix` storing `n·(n−1)/2` `f32`s, so a 100,000-city instance needs roughly 20 GB and cannot be constructed. Until #647 replaces the full matrix with an O(n) distance backend, verification for this change is limited to TSPLIB instances with fewer than 2,000 cities and a published optimal cost (e.g. berlin52 and att532). Quality is asserted loosely (1.5× the published optimum), which absorbs the known EUC_2D rounding difference. Curve-geometry properties (bijection, adjacency) remain synthetic `p ≤ 6` tests because they exercise the curve itself, not an instance. No 100k measurement is part of this change; it is tracked in #647.

## Risks / Trade-offs

- **Wrong Hilbert geometry** → reference, bijection, and adjacency tests for `p ≤ 6`; no merge without them.
- **`p = 16` overflow** → `u64` indices and a boundary test at `p = 16`.
- **Verification capped below 2,000 cities** → larger-instance support and measurement are tracked in #647; the 1.5× quality bound stays loose enough to absorb metric rounding.
- **Quality only ~25% above optimal** → quality is asserted loosely (berlin52 `< 1.5 × 7542`); the value is as a warm start, not a final answer.
- **Elongated maps waste cells with `uniform`** → `stretch_to_square` mode and a test with a high-aspect-ratio point set.
- **Surface drift (API/WASM/web)** → compile-time exhaustiveness where possible, plus explicit list/round-trip tests; the wasm test suite is run manually since it is outside the workspace.
- **Multi-orientation cost** still O(n log n) per orientation → capped at 8 orientations; no budget plumbing needed.
- **n = 1 unreachable via `DistanceMatrix`** → guard defensively like `greedy_edge`, even though the normal path cannot construct it.

## Migration Plan

Additive only — no data migration, no breaking changes. Rollback is deleting the new solver and its registry entries.

## Open Questions

None. The web explainer is explicitly a separate follow-up and does not affect this design.

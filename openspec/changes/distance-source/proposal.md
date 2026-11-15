# Proposal

## Why

`TspProblem` unconditionally bundles a full `DistanceMatrix` (the lower triangle of a symmetric matrix, `n·(n-1)/2` `f32` entries). That is O(n²) memory — ~18.6 GiB of distances alone at 100,000 cities — so a 100k-city instance cannot even be constructed, let alone solved. It is latent today because real TSPLIB instances top out around 1k–10k cities, but it blocks the 100k-city claim the Space-Filling Curve solver (#643) makes.

## What Changes

- Introduce a `DistanceSource` **trait** as the single distance-query boundary, with a swappable backend held as `Arc<dyn DistanceSource>`. Two backends ship initially: `Matrix` (the current precomputed lower triangle) and `OnTheFly` (compute the TSPLIB distance per lookup, zero matrix memory). `TspProblem.distances` changes from a concrete `DistanceMatrix` to `Arc<dyn DistanceSource>`. **BREAKING** for every consumer that reads `problem.distances` as a concrete `DistanceMatrix`.
- Route the distance methods (`distance_between`, `distance_by_pos`, `tour_length`, `tour_length_by_pos`, `distances_from`, `nearest`, `nearest_unvisited`, `pos2city_id`, `city_id2pos`, `num_cities`) through the trait, with default implementations so every backend shares one summation order and one set of edge-case behaviours.
- Fix `lin_kernighan`, which today builds its own EUC_2D matrix from `problem.cities` (`lin_kernighan.rs:41`) and therefore ignores the declared metric and the chosen backend. This is a behaviour fix on GEO/ATT/CEIL_2D/EXPLICIT instances and requires re-baselining LK's published numbers.
- Add a backend-comparison benchmark that runs the **same** (solver, instance) cells under `--distance-backend matrix` and `--distance-backend on-the-fly`, over multiple runs per cell, and reports peak RSS, wall-clock, and tour cost as medians/distributions rather than single runs, reusing `scripts/bench-matrix.sh`.
- Auto-select the on-the-fly backend above a configurable city-count threshold for coordinate-based input, and expose `--distance-backend matrix|on-the-fly|auto`. Selection keys off whether the file carries explicit weights, not off the (overridable) declared distance type. EXPLICIT input always keeps the materialized matrix.
- Add k-nearest candidate-list move evaluation for the local-search solvers that only inspect a neighbourhood (`two_opt`, `or_opt`, `lin_kernighan`).
- Store EXPLICIT distances as `u32`/`u16` when the TSPLIB weight range allows, halving/quartering the stored EXPLICIT matrix.
- Let the SFC solver complete 100k-city EUC_2D instances through the normal `solve` path under the documented memory ceiling; no separate SFC entry point.

## Capabilities

### New Capabilities

- `distance-source`: the system's distance-query boundary — a swappable-backend trait, matrix vs on-the-fly result equivalence, backend selection policy, the large-instance memory ceiling, and the measured memory/CPU trade-off between backends.

### Modified Capabilities

- `tour-construction`: the SFC solver must construct and score 100k-city EUC_2D tours through the normal `solve` path under 1 GiB with automatic selection. This retires the existing "verification limited to instances below 2,000 cities until larger-instance support is addressed separately" limitation.

## Impact

- **Library (`src/tsp/`)**: `distance_matrix.rs` (trait impl + the four TSPLIB primitives), `tsplib.rs` (backend-aware constructor keyed off `has_explicit_weights()`), `mod.rs` (`TspProblem`, `Solution`, `DistanceType`), `comparison.rs`, `graph.rs::sorted_edges`, `lin_kernighan.rs` (private-matrix fix), and the remaining solver modules.
- **`teeline-cli`**: `main.rs` builds the matrix and `TspProblem`, and recomputes the optimal-tour comparison via `distances.tour_length(&opt_tour.route)` (lines ~540–586, ~725–736). The `--distance-type` override at lines ~524–534 must not bypass explicit-weight detection.
- **`src/config.rs`**: currently parses only `[[stage]]`; it must gain a top-level `distance_backend` key with CLI-over-TOML precedence, covered by `tests/pipeline_config_test.rs`.
- **`teeline-api`**: `services/tsp_service.rs` constructs `TspProblem` in two places (lines ~44, ~52); the API runs solvers in `tokio::task::spawn_blocking`, so the trait must be `Send + Sync`. Adding `seed` to `LKOptions` also requires updating the `LKOptions` literal at `tsp_service.rs:177–184`.
- **`teeline-wasm`** (out-of-workspace): `lib.rs:~116`, `~317`, `~550`, `~571` build matrices. The two scoring-only paths must pick `Matrix` for explicit weights and `OnTheFly` for coordinates, or EXPLICIT inputs regress. The ambient `teeline-wasm.d.ts` must be updated for any WIT export change.
- **`teeline-qt`** (out-of-workspace): `solver_engine.rs` constructs `TspProblem` and moves it into `std::thread::spawn` workers.
- **`constitution.md`** §4 names `DistanceMatrix` explicitly in the `Solution.total` and spatial-query guardrails. Per §8, a separate prerequisite PR amends it to name the distance source before this change proceeds.
- **Tests**: inline `#[cfg(test)]` blocks plus integration tests in `tests/`.
- **Bench infrastructure**: reuse `scripts/bench-matrix.sh`, a new `bench/matrix/tier-distance.toml`, and a dedicated scaling run; reports under `docs/benchmarks/`.

No change to the TSPLIB input/output contract, the `Solvers` dispatch path, or any distance metric's semantics. Correctness is gated on byte-identical results between the `Matrix` and `OnTheFly` backends for every distance type.

Delivery is a stacked PR series, one reviewable concern per PR; the constitution amendment is PR 1 and must merge before the trait refactor. The stack is mapped in `design.md` and tracked in `tasks.md`.

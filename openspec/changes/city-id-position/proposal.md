# Proposal

## Why

City IDs and distance-source positions are both raw `usize`, and the codebase mixes them. The original plan proposed newtyping both spaces and migrating all 23 solvers. A code-level review showed most of that churn is in code where the two spaces never meet, while the genuinely dangerous defects are two reachable crashes and a set of duplicated indexes:

- **LK can abort the process.** The API accepts any `id: Option<usize>` with no range check (`teeline-api/src/models/request.rs:244`), and `lin_kernighan` sizes its scratch arrays by the largest ID (`vec![0; max_id+1]`, `lin_kernighan.rs:101`), so a few huge IDs force an enormous allocation. WASM has the same path through `parse_json_cities`.
- **Duplicate IDs panic.** Coordinate input reaches `assert!(n == city_idx.len())` in `DistanceMatrix::from_parts` (`distance_matrix.rs:269`); only EXPLICIT input is checked (`tsplib.rs:247`). The API returns a 500 and WASM traps instead of reporting a validation error.
- **The id→position mapping exists four times**: `DistanceMatrix.city_idx`, `OnTheFly`'s `IdIndex`, `CandidateList`'s `IdIndex`, and a per-`Solution` `cities_idx`. `Solution.cities`, `Solution.cities_idx`, and `get_by_city_id` have no callers and are dead.
- The Fourier `usize::MAX` query sentinel is a working workaround, not a bug; `branch_bound`'s `UNVISITED_NODE = 0` only corrupts progress messages; `bellman_karp`'s `unwrap_or(0)` is unreachable.

The full `CityId` migration would touch 102 `KDPoint` sites, 35 `Solution` sites, and all 23 solvers for little safety gain in code that never crosses the boundary. This change instead fixes the reachable defects, consolidates the index, and types the `Position` boundary where the two spaces actually meet. The `CityId` newtype is deferred (see `design.md` D1).

## What Changes

- Fix the two reachable defects: remap `lin_kernighan` to dense positions, and make duplicate-ID rejection a fallible step at source construction (API 400, WASM error).
- Replace `DistanceMatrix`'s `city_idx` HashMap and position-keyed `CityTable` HashMap with the existing `IdIndex` (dense/sparse) plus `Vec<KDPoint>`; delete the dead `Solution` fields and `city_table_from_vec`.
- Introduce `Position(usize)` and use it on `DistanceSource`'s position methods, `CandidateList`, and the position-internal solvers, so a position cannot be passed where an ID is expected and vice versa. Keep `graph.rs`'s compact `u32` edge storage unchanged.
- Fix `branch_bound`'s progress filter and `bellman_karp`'s `unwrap_or(0)`.
- Defer the `CityId` newtype, typing `KDPoint.id`/`Solution.route`/`ProgressMessage`/`init_tour`/`validate_tour`, migrating the 15 ID-based solvers, and removing the Fourier query sentinel.

## Capabilities

### New Capabilities

None. This is an internal fix and refactor: TSPLIB IDs are preserved, the output contract is identical, and no distance metric changes.

### Modified Capabilities

None. `skip_specs: true` is set in `.openspec.yaml` because no spec-level behavior changes.

## Impact

- **Library**: `id_index.rs`, `distance_matrix.rs`, `on_the_fly.rs`, `candidate_list.rs`, `distance_source.rs`, `mod.rs` (`Solution`), `graph.rs`, `lin_kernighan.rs`, `branch_bound.rs`, `bellman_karp.rs`, `ant_colony.rs`, `christofides.rs`, `savings.rs`, `greedy_edge.rs`.
- **Surfaces**: `teeline-api` (duplicate-ID 400), `teeline-wasm` (error instead of trap). No JSON/WIT/Qt output-shape changes.
- **Performance**: the matrix backend should get faster (dense `IdIndex` replaces SipHash `HashMap` lookups); LK memory drops from O(max_id) to O(n). `Position` is memory-neutral where the storage is already `usize`; `graph.rs`'s `u32` edge list stays `u32`.
- **Dependency**: none; `distance-source` is already archived (`openspec/changes/archive/2026-11-15-distance-source`).

## Delivery

Four stacked PRs: (1) LK remap + candidate rows; (2) fallible index + duplicate-ID errors + small fixes; (3) index consolidation + dead-code removal; (4) `Position` newtype. Deterministic solvers gate on sha256; stochastic solvers gate on permutation validity and quality across runs.

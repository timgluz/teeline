# Proposal

## Why

City IDs and distance-source positions are both raw `usize`, and the codebase mixes them. Tests build points with IDs `0..n`, while TSPLIB uses 1-based IDs and the API accepts arbitrary IDs, so "position == ID" holds in tests but not in production. Concrete symptoms:

- `KDPoint::new` sets `id = 0`, so points built that way collide in `NearestResult::add`, which excludes by ID. Fourier works around it with a `usize::MAX` sentinel.
- Solvers split into two conventions: position-based (`christofides`, `greedy_edge`, `savings`, `fourier`) and ID-based (`two_opt`, `or_opt`, `nearest_neighbor`, `three_opt`, `lin_kernighan`), with `ant_colony`/`bellman_karp` converting.
- `Solution` copies `cities` and rebuilds `cities_idx`, duplicating the source's mapping across 34 construction sites.
- The `nearest` regression on 1-based IDs is the kind of bug this ambiguity invites.

`distance-source` defines the contract (positions are dense indices, IDs are external) but does not type-enforce it. This change closes that gap.

## What Changes

- Introduce `CityId(usize)` and `Position(usize)` newtypes, plus a `CityIndex` that owns `ids: Vec<CityId>` and `by_id: HashMap<CityId, Position>`.
- `KDPoint` holds a `CityId`; `KDPoint::new` is removed or renamed so an ID is always explicit.
- `TspProblem` owns the canonical position↔ID mapping; `Solution` reuses it instead of copying the city list and rebuilding an index.
- `DistanceSource` takes `Position` for `distance_by_pos`/`point_at` and `CityId` for `distance_between`/`tour_length`; the reported route stays `CityId`.
- One solver convention: positions internally, IDs at the tour/output boundary. Update all solver modules.
- Replace the Fourier `usize::MAX` sentinel by making `nearest` take an explicit exclusion.
- Update the CLI, API, WASM, Qt, and tests.

## Capabilities

### New Capabilities

None. This is a pure internal refactor: TSPLIB IDs are preserved, the output contract is identical, and no distance metric changes.

### Modified Capabilities

None. `skip_specs: true` is set in `.openspec.yaml` because no spec-level behavior changes.

## Impact

- **Library**: `kdtree.rs` (`KDPoint`), `distance_matrix.rs`, `mod.rs` (`TspProblem`, `Solution`, `NearestResult`), and every solver module.
- **Surfaces**: `teeline-cli`, `teeline-api`, `teeline-wasm` (out-of-workspace), `teeline-qt` (out-of-workspace).
- **Tests**: `build_points`' 0-based IDs, the 1-based TSPLIB fixtures, and the 34 `Solution::new`/`from_parts` sites.
- **Dependency**: lands after the full `distance-source` stack (PR 10), so the trait signatures and solver conventions are stable before the newtypes change them.

## Delivery

A stacked PR series, each behaviour-preserving: (1) ADR 011 + newtypes + `CityIndex`; (2) `DistanceSource` signature migration; (3) solver migration to one convention; (4) `KDPoint::new`/sentinel removal; (5) docs and graphify update. Existing tests plus new non-contiguous-ID tests gate every step.

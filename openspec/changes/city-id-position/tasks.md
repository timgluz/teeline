# Tasks

## 1. Newtypes and the canonical index

- [ ] 1.1 Add `CityId` and `Position` as `#[repr(transparent)]` newtypes with `into_inner`, the standard derives, and round-trip tests; verify a `CityId` cannot be passed where a `Position` is expected.
- [ ] 1.2 Add `CityIndex` (`ids: Vec<CityId>`, `by_id: HashMap<CityId, Position>`) with `id_at`, `position_of`, `len`, duplicate-ID rejection, and tests; verify round-trips and duplicate rejection.
- [ ] 1.3 Add `Arc<CityIndex>` to `TspProblem` and build it once in `new`/`from_source`/`from_points`; verify the position invariant (`index.id_at(i) == cities[i].id`, `index.len() == cities.len()`).
- [ ] 1.4 Write ADR 011 recording the `CityId`/`Position` model and add it to the `docs/adr/` index; verify the index links it.

## 2. KDPoint and Solution

- [ ] 2.1 Change `KDPoint.id` to `CityId`, remove `KDPoint::new` (the id-0 constructor), and update `build_points`, TSPLIB, the API, and tests; verify every construction site compiles.
- [ ] 2.2 Change `Solution.route` to `Vec<CityId>`, reuse the shared `CityIndex` instead of rebuilding `cities_idx`, and add `route_ids()`; verify all 34 `Solution` construction sites.
- [ ] 2.3 Update the CLI, API, WASM, and Qt response builders to convert `CityId` to `usize`/`u32`; verify output is byte-identical on the tracked fixture corpus.
- [ ] 2.4 Add 1-based and non-contiguous-ID tests (for example 10, 20, 30) for construction, `validate_tour`, and the printed tour; verify the original IDs are preserved.

## 3. DistanceSource signatures

- [ ] 3.1 Change `distance_by_pos`, `point_at`, `pos2city_id`, and `city_id2pos` to `Position`/`CityId`; verify the matrix and on-the-fly implementations and the conformance suite.
- [ ] 3.2 Change `distance_between`, `tour_length`, and `distances_from` to `CityId`; verify the backend-equivalence tests.
- [ ] 3.3 Change `nearest` and `nearest_unvisited` to take `Position` and an explicit exclusion; update Fourier and the six test call sites; verify the `usize::MAX` sentinel is gone.
- [ ] 3.4 Update the conformance suite's non-contiguous-ID case to the newtypes; verify it fails if a position is used as an ID.

## 4. One solver convention

- [ ] 4.1 Keep `christofides`, `greedy_edge`, `savings`, and `fourier` on `Vec<Position>` internally and map to `CityId` via `CityIndex::id_at` at the end; verify each tour.
- [ ] 4.2 Keep `two_opt`, `or_opt`, `three_opt`, `lin_kernighan`, and `nearest_neighbor` on `Vec<CityId>` routes and call `city_id2pos` only where a position is needed; verify each tour.
- [ ] 4.3 Delete `ant_colony`'s and `bellman_karp`'s local `to_pos`/`to_id` converters and use `CityIndex`; verify their tours.
- [ ] 4.4 Remove remaining `cities[pos].id` and `id - 1` assumptions; verify with a non-contiguous-ID fixture.

## 5. Legacy footguns and documentation

- [ ] 5.1 Remove the remaining id-0 constructor and the Fourier sentinel references; verify no `usize::MAX` sentinel remains.
- [ ] 5.2 Update AGENTS.md "Core types", the README, and any doc that describes city IDs versus positions; verify the docs match the new types.
- [ ] 5.3 Run `graphify update .`; verify the graph is current.

## 6. Integration verification

- [ ] 6.1 Run `cargo test --workspace`; verify it is green.
- [ ] 6.2 Run `cargo clippy --workspace -- -D warnings`; verify no warnings.
- [ ] 6.3 Run `cargo fmt --check`; verify no diffs.
- [ ] 6.4 Build the WASM target and verify the ambient `teeline-wasm.d.ts` matches any changed export.
- [ ] 6.5 Build and smoke-test `teeline-qt`; verify a solve still renders progress.
- [ ] 6.6 Compare the CLI output on the tracked fixture corpus before/after; verify every tour cost and ID sequence is unchanged.

## 7. Stacked PR delivery

- [ ] 7.1 Open PR 1 (`refactor/city-id-newtypes`) against `master` for group 1, including ADR 011; verify no consumer behaviour changes.
- [ ] 7.2 Open PR 2 (`refactor/kdpoint-city-id`) against PR 1 for group 2; verify output is byte-identical.
- [ ] 7.3 Open PR 3 (`refactor/distance-source-ids`) against PR 2 for group 3; verify the conformance suite.
- [ ] 7.4 Open PR 4 (`refactor/solver-city-id-convention`) against PR 3 for group 4; verify every solver's tour.
- [ ] 7.5 Open PR 5 (`refactor/remove-id-footguns`) against PR 4 for group 5; verify no sentinel or id-0 constructor remains.
- [ ] 7.6 Open PR 6 (`chore/city-id-integration`) against PR 5 for group 6; verify the full integration checks.
- [ ] 7.7 After each merge, retarget the next PR to the new base and verify its diff contains only its own scope.

## Workflow follow-up

- Land this stack after the full `distance-source` stack (PR 10) so the trait signatures and solver conventions are stable.
- Archive after the stack merges and the fixture outputs are verified byte-identical.

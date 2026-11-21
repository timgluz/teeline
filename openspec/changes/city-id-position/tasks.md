# Tasks

## 1. Foundation

- [x] 1.1 Write ADR 013 recording the `Position`-only boundary, distance-source index ownership, and fallible source construction; add it to the `docs/adr/` index and verify the index links it.
- [ ] 1.2 Capture the pre-change baseline: build a `master` worktree in release, run `cargo bench --bench kdtree` and `cargo bench --bench distance_matrix` with `--save-baseline pre`, run `scripts/bench-distance-scaling.sh` for memory, and store the deterministic-solver fixture outputs.

## 2. PR 1 — Lin-Kernighan remap and position-storing candidate rows

- [x] 2.1 Change `CandidateList` rows to store neighbour **positions** (`Vec<Vec<Position>>` after PR 4; `Vec<Vec<usize>>` here), keeping the row index a position; preserve the city-ID tie-break in `build_from_distances` by sorting on `(distance, city_id)` before mapping to positions.
- [x] 2.2 Make `candidate_positions_into` consume positions directly so it no longer performs a `city_position` lookup per neighbour; verify 2-opt and or-opt tours are unchanged.
- [x] 2.3 Remap `lin_kernighan`'s tour to positions on entry and size `next`/`prev`/`used`/`adj`/`pos` by `n` (positions); switch `d()` to `distance_by_pos`.
- [x] 2.4 Map the final tour back to IDs with `pos2city_id`; handle an unknown `init_tour` ID with an error or the existing small-n fallback instead of indexing by the raw ID.
- [x] 2.5 Add tests with huge IDs (for example `10^12`) and unordered IDs (for example `[30, 10, 20]`) to `lin_kernighan` and the candidate list; verify the process does not allocate by the largest ID and the tour preserves the original IDs.
- [x] 2.6 Run the seeded LK quality checks and verify the tour length matches the pre-change baseline.

## 3. PR 2 — Fallible index and small fixes

- [x] 3.1 Change `IdIndex::build` to return `Result` and reject duplicate IDs; verify dense, sparse, non-contiguous, and duplicate inputs.
- [x] 3.2 Propagate the fallible construction through `OnTheFly::new`, `DistanceMatrix::new`/`from_explicit_slice`, and the `TspProblem` constructors; update the test helpers that call them.
- [x] 3.3 Map duplicate/invalid IDs to HTTP 400 in `teeline-api` and to an error (not a trap) in `teeline-wasm`; verify both surfaces.
- [x] 3.4 Change `branch_bound`'s progress update to send the visited prefix `path[..k]` so city 0 is not filtered out; keep `UNVISITED_NODE`.
- [x] 3.5 Replace `bellman_karp`'s `pos2city_id(..).unwrap_or(0)` with `expect`, since the position comes from `0..n`.
- [x] 3.6 Add duplicate-ID tests for coordinate input, EXPLICIT input, the API, and WASM; verify the error path instead of the panic/trap.

## 4. PR 3 — Index consolidation and dead-code removal

- [x] 4.1 Replace `DistanceMatrix`'s `city_idx: HashMap` and position-keyed `CityTable` with `IdIndex` + `Vec<KDPoint>`; verify `point_at`, `pos2city_id`, `city_id2pos`, `distance_between`, and `tour_length`.
- [x] 4.2 Reduce `Solution` to `{ total, route }`; delete `cities`, `cities_idx`, `cities()`, and `get_by_city_id()` (no callers), and stop copying the city list or building the map in `Solution::new`/`from_parts`.
- [x] 4.3 Delete the unused, ID-keyed `city_table_from_vec`; remove `CityTable` from `DistanceMatrix` and keep the alias only if another module still needs it.
- [x] 4.4 Verify the matrix backend's tours and `Solution` totals are identical to the pre-change baseline.
- [x] 4.5 Benchmark `distance_between`/`tour_length` on the matrix backend (dense IDs and sparse IDs) and record whether the dense `IdIndex` improves on the SipHash `HashMap`.

## 5. PR 4 — `Position` newtype

- [x] 5.1 Add `Position` as a `#[repr(transparent)]` newtype with `into_inner`, the standard derives, and tests; assert `size_of::<Position>() == size_of::<usize>()`.
- [x] 5.2 Change `DistanceSource`'s position methods (`distance_by_pos`, `pos2city_id`, `city_id2pos`, `point_at`, `tour_length_by_pos`) to `Position`; verify the matrix and on-the-fly implementations and the conformance suite.
- [x] 5.3 Type `CandidateList`'s rows and API with `Position`; type only the `usize` boundary of `graph.rs` (`select_edges`/`hamiltonian_cycle_to_path`) and keep its packed `(f32, u32, u32)` edge list unchanged.
- [x] 5.4 Adopt `Position` in `bellman_karp`, `ant_colony`, `christofides`, `savings`, and `greedy_edge`; verify each tour is unchanged.
- [x] 5.5 Update the remaining `src` and `tests` call sites; verify no CLI/API/WASM/Qt output type changes.
- [x] 5.6 Add the allocation test and verify no new allocation on `pos2city_id`/`city_id2pos`, `distance_by_pos`, and `tour_length` hot loops.

## 6. Verification

- [ ] 6.1 Run `cargo test --workspace`; verify it is green.
- [ ] 6.2 Run `cargo clippy --workspace -- -D warnings`; verify no warnings.
- [ ] 6.3 Run `cargo fmt --check`; verify no diffs.
- [ ] 6.4 Compare deterministic solvers (2opt, or_opt, 3opt, nn, sfc, bhk, seeded LK) by sha256 on the fixture corpus; verify byte-identical output.
- [ ] 6.5 Verify stochastic solvers (ACO, CS, FPA, GA, GSA, PSO, random shuffle) return a valid permutation of the original IDs and stay within the quality budget across runs.
- [ ] 6.6 Run the micro-bench gate: no regression > +3% (p < 0.05, two reruns); `nearest_k5` blocks at +2%.
- [ ] 6.7 Run the wall-clock gate: median of 10 runs, no regression > +5% on deterministic cells with a median ≥ 200 ms.
- [ ] 6.8 Run the memory gate: on-the-fly peak RSS no more than +2% or +1 MiB; verify `graph.rs`'s `u32` edge storage is unchanged and LK scales with `n`, not `max_id`.
- [ ] 6.9 Run the allocation gate and verify zero allocations in the hot-loop test.
- [ ] 6.10 Build the WASM target and verify the ambient `teeline-wasm.d.ts` matches any changed export; verify duplicate IDs return an error.
- [ ] 6.11 Build and smoke-test `teeline-qt`; verify a solve still renders progress.
- [ ] 6.12 Run `graphify update .`; verify the graph is current.

## 7. Stacked PR delivery

- [ ] 7.1 Open PR 1 (`fix/lk-position-remap`) against `master` for group 2, including ADR 013; verify the huge/unordered-ID tests.
- [ ] 7.2 Open PR 2 (`fix/duplicate-city-ids`) against PR 1 for group 3; verify the API 400 and WASM error paths.
- [ ] 7.3 Open PR 3 (`refactor/index-consolidation`) against PR 2 for group 4; verify the dead-code removal and matrix outputs.
- [ ] 7.4 Open PR 4 (`refactor/position-newtype`) against PR 3 for group 5; verify the full conformance suite and the performance gates.
- [ ] 7.5 After each merge, retarget the next PR to the new base and verify its diff contains only its own scope.
- [ ] 7.6 Archive after the stack merges and the deterministic fixture outputs are verified byte-identical.

## Out of scope

- The `CityId` newtype, typing `KDPoint.id`/`Solution.route`/`Route`/`init_tour`/`ProgressMessage`/`validate_tour`, and migrating the 15 ID-internal solvers.
- Removing the Fourier `usize::MAX` query sentinel; revisit only if it is free on `nearest_k5`.
- Seeding the stochastic solvers so their output can be sha256-gated; separate change.

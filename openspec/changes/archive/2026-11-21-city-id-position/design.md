# Design

## Context

See `proposal.md` for motivation. The current tree has four index spaces and four duplicated id→position structures:

| Space | Meaning | Today | Target |
| --- | --- | --- | --- |
| **City ID** | external identifier; TSPLIB 1-based, API-arbitrary, possibly non-contiguous | `usize` in `KDPoint.id`, routes, `Solution.route` | stays `usize` (deferred) |
| **Position** | dense `0..n` index into `problem.cities` (matrix row / point index) | `usize` in trait signatures and index maps | `Position` |
| **Route rank** | slot index in a tour (`path[rank]`) | `usize` loop vars; `CandidateList::route_positions` values | stays `usize` (transient) |
| **Auxiliary index** | Fourier curve sample, SOM neuron, SFC bucket | sometimes reuses `KDPoint.id` | stays local |

The 23-solver census found 8 position-internal solvers (`christofides`, `greedy_edge`, `savings`, `som`, `space_filling_curve`, `fourier`, `ant_colony`, `bellman_karp`) and 15 ID-internal ones. Only the position-internal camp and the trait boundary need typing; 10 of the 15 ID-internal solvers never touch a position.

Two reachable defects drive this plan:

- `lin_kernighan` sizes `next`/`prev`/`used`/`adj`/`pos` by the largest city ID (`lin_kernighan.rs:101-135`, `344`, `393`), and the API accepts arbitrary IDs (`teeline-api/src/models/request.rs:244`), so a few huge IDs abort the process.
- Coordinate input with duplicate IDs hits an `assert!` in `DistanceMatrix::from_parts` (`distance_matrix.rs:269`); only EXPLICIT input is validated (`tsplib.rs:247`).

## Goals / Non-Goals

**Goals:**

- Fix the LK allocation crash and the duplicate-ID panic.
- Type the position boundary with `Position` where IDs and positions actually meet.
- Consolidate the canonical id→position index in the distance source and delete the dead `Solution` state.
- Preserve output, solver quality, runtime, and memory.

**Non-Goals:**

- A `CityId` newtype; typing `KDPoint.id`, `Solution.route`, `Route`, `init_tour`, `ProgressMessage`, or `validate_tour`.
- Migrating the 15 ID-internal solvers.
- Changing TSPLIB/JSON/WIT shapes or the Qt/WASM output types.
- Removing the Fourier `usize::MAX` query sentinel (deferred; revisit only if it is free on `nearest_k5`).
- Sharing `TspProblem.cities` via `Arc<[KDPoint]>`.

## Decisions

### D1: `Position(usize)`, and defer `CityId`

```rust
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Position(usize);
```

`into_inner()` exposes the raw index at the few `usize` boundaries. Newtypes are zero-cost, prevent a position from being passed where an ID is expected (and vice versa), and make the trait self-documenting.

`CityId` is deferred because IDs already flow as `usize` through `Route`, `init_tour`, `ProgressMessage`, `validate_tour`, and 15 solvers; typing them spreads into the Qt/WASM boundary for little gain where the spaces never meet. **Revisit if** an ID-based solver moves to positions for speed, or another ID/position bug lands in ID-space solver code after this change.

### D2: The distance source owns a fallible `IdIndex`

- `IdIndex::build(cities) -> Result<IdIndex, &'static str>` rejects duplicate IDs (today `OnTheFly` and the parser each compare `len()` separately).
- `DistanceMatrix` holds `IdIndex` plus `Vec<KDPoint>`; `OnTheFly` keeps its `IdIndex`.
- `TspProblem` stays `{ cities, distances }` and does **not** build a second index; `pos2city_id`/`city_id2pos` expose the source's index through the trait. This avoids doubling memory at the 84 `TspProblem::new` sites, where the source was built separately.
- `DistanceMatrix::new`/`from_explicit_slice` and `OnTheFly::new` return `Result`; the API maps duplicate/invalid IDs to 400, and WASM returns an error instead of trapping.

`CandidateList` keeps its own one-time `IdIndex` (O(n), already dense/sparse) because it needs id→row lookup at query time; the consolidation targets `DistanceMatrix`'s two HashMaps and `Solution`'s dead map, not a shared `Arc`.

### D3: `DistanceSource` position methods take `Position`

```rust
fn distance_by_pos(&self, a: Position, b: Position) -> Result<f32, &'static str>;
fn pos2city_id(&self, pos: Position) -> Option<usize>;
fn city_id2pos(&self, id: usize) -> Option<Position>;
fn point_at(&self, pos: Position) -> Option<KDPoint>;
fn tour_length_by_pos(&self, path: &[Position]) -> f32;
```

The ID-based methods (`distance_between`, `tour_length`, `distances_from`) and `nearest`/`nearest_unvisited` keep their current `usize`/`KDPoint` signatures. This is the minimal two-way check: a position cannot reach an ID slot and an ID cannot reach a position slot.

### D4: `CandidateList` rows store positions

Rows are indexed by position and currently store neighbour **IDs**; store neighbour **positions** instead (`rows: Vec<Vec<Position>>`). This is memory-neutral on 64-bit (`Position` is `#[repr(transparent)]` over `usize`), and it removes the per-neighbour `city_position` lookup from `candidate_positions_into`.

- `build`/`build_from_distances` fill rows with positions.
- `build_from_distances` must keep sorting ties by **city ID** (today it sorts `(distance, city_id)`); resolve the position after the sort so output order is unchanged.
- `route_positions(&[usize]) -> Vec<usize>` keeps the documented `usize::MAX` "not in path" **rank** sentinel. It is not the Fourier query sentinel and is not removed.

### D5: `graph.rs` keeps its compact `u32` edge storage

`sorted_edges` packs O(n²) edge endpoints as `(f32, u32, u32)` to roughly halve the working set (~600MB vs ~1.2GB at n=10k, `graph.rs:54-58`). Do **not** store `Position` there. `Position` may appear at the `usize` boundary of `select_edges`/`hamiltonian_cycle_to_path` (memory-neutral), but the packed list stays `u32`.

### D6: `lin_kernighan` remaps to positions

- On entry, map the tour IDs to positions via `city_id2pos`; an unknown ID returns an error (or the existing n<4 fallback) instead of indexing or allocating by the ID.
- Keep `next`/`prev`/`used`/`adj`/`pos` as `Vec<usize>` sized `n` and indexed by position; the scratch arrays stay `usize` internally.
- Use `distance_by_pos` inside `d()`.
- On exit, map positions back to IDs via `pos2city_id`.
- Consume the position-storing candidate rows so the innermost neighbour loop needs no lookup.

### D7: Small fixes

- `branch_bound`: send the visited prefix `path[..k]` in progress updates instead of filtering `x != UNVISITED_NODE`, which drops city 0. Keep the constant.
- `bellman_karp`: replace `pos2city_id(pos).unwrap_or(0)` with `expect`, since positions come from `0..n`.

### D8: Delete dead `Solution` state and consolidate `DistanceMatrix`

- `Solution { total: f32, route: Vec<usize> }`; delete `cities`, `cities_idx`, `cities()`, and `get_by_city_id()` (no callers in the repo). `Solution::new`/`from_parts` stop copying the city list and building the map.
- `DistanceMatrix { n, items, cities: Vec<KDPoint>, city_idx: IdIndex, is_coordinate_metric }`; `point_at` indexes the `Vec`, `city_id2pos` delegates to `IdIndex`.
- Delete the unused, ID-keyed `city_table_from_vec` (`mod.rs:2100`) and remove `CityTable` from `DistanceMatrix`; keep the alias only if another module still needs it.
- Delete the now-dead `DistanceMatrix::city_index()` accessor, whose field type changed to `IdIndex` (no callers).

## Performance & Memory Budget

Correctness wins over performance, but these are merge gates:

- **Harnesses**: `cargo bench --bench kdtree`, `cargo bench --bench distance_matrix` (both criterion, `harness = false`), `scripts/bench-matrix.sh` for wall clock, `scripts/bench-distance-scaling.sh` for memory. (The earlier `bench/distance-backend` reference was a branch name, not a harness.)
- **Baseline**: a `git worktree` of `master` built in release, used as `--binary`; record commit, `rustc -V`, and CPU for every run; same machine, pinned core, AC power.
- **Capture before PR 1**: `distance_between` over 10k seeded random pairs (matrix and on-the-fly, dense and sparse IDs), `tour_length` on att532 and a 20k on-the-fly instance, `candidate_positions_into` on a280, and seeded LK on att532.
- **Micro-bench gate**: no regression > +3% with p < 0.05 on two reruns; `nearest_k5` blocks at +2%.
- **Wall-clock gate**: median of 10 runs > +5%, only for deterministic cells with a median ≥ 200 ms (2opt, or_opt, 3opt, nn, sfc, bhk, seeded LK). Report shorter cells; do not gate on them.
- **Memory gate**: on-the-fly peak RSS > +2% or +1 MiB. `Position` must be memory-neutral where the underlying storage is `usize`; `graph.rs`'s `u32` list must not grow.
- **Allocation gate**: a test-only counting allocator asserts zero allocations in `distance_between`, `city_id2pos`/`pos2city_id`, and `tour_length` loops.
- **Expected wins**: a dense `IdIndex` replaces SipHash `HashMap` lookups in `DistanceMatrix` (likely faster for 2-opt/or-opt/3-opt/LK/SA/tabu); LK scratch memory drops from O(max_id) to O(n). Measure and report both.
- **Where memory could rise**: a second index (avoided by D2), a redundant `ids` Vec (not in this plan), or `Option<CityId>` (not in this plan). None apply.

### Measured (PR 3, att532)

- Matrix `distance_between` with dense IDs: 33.6 ns (SipHash `HashMap`) → 5.33 ns (`IdIndex::Dense`), −84% (p < 0.001, criterion against the `pre` baseline).
- Matrix `distance_between` with sparse IDs: 33.8 ns (`IdIndex::Sparse`); `tour_length`: 2.81 µs dense / 10.86 µs sparse.
- Deterministic output is byte-identical to the pre-change baseline on 98 solver/fixture pairs; `3opt` on att532 drops from 546.9 s to 96.7 s.

### Final verification (2026-11-21, master `fbb845ee`)

- **Deterministic parity**: 188 solver/fixture runs (both backends, seeded LK, exact solvers) are byte-identical by sha256 to the pre-change `476b3724` binary.
- **Stochastic**: 140 runs across ACO/CS/FPA/GA/GSA/PSO/shuffle on berlin52 and a280 were all valid permutations; medians stayed within ±5.4% of the pre-change binary.
- **Micro-bench**: dense matrix lookups improved 84% and the matrix build benches improved 2–21%. The kdtree deltas (`nearest_k5` +2.45%) are address layout only: the benchmark functions' instruction streams are identical between the two binaries.
- **Wall clock** (median of 10, matrix): `or_opt` att532 −66.9%, `3opt` a280 −81.6%, `lk` att532 −1.8%, `lk` a280 +1.4%; `3opt` att532 546.9 s → 100.1 s.
- **Memory**: on-the-fly peak RSS at 10k/25k/50k = −2.5%/−7.3%/−15.1% vs the baseline; `graph.rs` still stores `(f32, u32, u32)`; the huge-ID LK test passes.
- **Allocation**: the hot-loop test reports zero allocations on both backends and both `IdIndex` branches.
- **WASM**: the component builds; WIT and `teeline-wasm.d.ts` are unchanged; the duplicate-ID tests pass.
- **Qt**: builds and starts headless; the QML event loop runs.

## Risks / Trade-offs

- **LK remap correctness** — the tour, tie-breaks, and chain reconstruction must be preserved. Gate with the existing LK tests, seeded quality runs, and a sparse/huge-ID test.
- **Candidate row tie-breaks** — `build_from_distances` currently breaks ties by city ID; resolve positions after the sort to keep output order identical.
- **Duplicate-ID propagation** — `DistanceMatrix::new` becomes fallible; API returns 400, WASM returns an error. Update the test helpers that call it.
- **`Position` call-site churn** — ~79 call sites in `src` and ~23 in `tests`, none in the CLI/API/WASM/Qt crates, because output types do not change.
- **Byte-identical verification** — only deterministic solvers can be compared by sha256; stochastic solvers must be checked as a valid permutation plus quality across runs. Seeding them is a separate change.
- **Benchmark noise** — a 2% gate on a laptop sits at criterion's own noise floor; use the +3% micro threshold, median-of-10 wall clock, and the pinned-core protocol above.

## Migration Plan

1. Write ADR 013 (`Position` boundary, index ownership, fallible construction) and capture the baseline.
2. PR 1: remap LK to positions and switch candidate rows to positions; add a huge/unordered-ID test.
3. PR 2: make `IdIndex::build` fallible, propagate duplicate-ID errors to the API/WASM, fix `branch_bound` progress and `bellman_karp` `expect`.
4. PR 3: consolidate `DistanceMatrix` onto `IdIndex` + `Vec<KDPoint>`, delete the dead `Solution` state and `city_table_from_vec`.
5. PR 4: introduce `Position` on `DistanceSource`, `CandidateList`, `graph.rs`'s `usize` boundary, and the position-internal solvers (BHK, ACO, Christofides, Savings, Greedy Edge).
6. Run the integration and performance gates; archive after the stack merges.

Rollback is a stack revert; no data migration.

## Open Questions

- `CandidateList::neighbours` should take a `Position` or a city ID after PR 4? Prefer `Position` once callers resolve their endpoints once.
- Keep `route_positions`' `usize::MAX` sentinel (recommended, memory-neutral) or switch to `Option<usize>` (doubles the array)?
- Should stochastic solvers get deterministic seeding in a separate PR so output can be sha256-gated?
- Is the Fourier nearest exclusion worth doing if it is free on `nearest_k5`? Deferred.

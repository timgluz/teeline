# Tasks

## 1. Prerequisite and governance

- [x] 1.1 Amend `constitution.md` in a separate PR (per §8): §4 so the `Solution.total` and spatial-query guardrails name a distance source rather than `DistanceMatrix`, and §2 with the multi-run improvement-verification principle; verify the amendment states what changed and why, no guardrail still names the concrete type, and the new principle is present.
- [x] 1.2 Update AGENTS.md "Core types" and any README text that names `DistanceMatrix` as the mandatory distance type; verify the docs describe the swappable source.

## 2. Phase 0 — Benchmark development (reuse the existing harness)

- [x] 2.1 Add `bench/matrix/tier-distance.toml` with a `matrix` config (the pre-selection default), the tracked fixtures `burma14`, `ulysses16`, `berlin52`, `a280`, `att532`, an explicit `solvers` list per config, and `utility = []` plus its budget entry; verify `scripts/bench-matrix.sh ... --dry-run` lists every cell.
- [x] 2.2 Add fixed-work configs (`--epochs N`) for the solvers that honour them, and bound the instance size for `2opt`/`or_opt`/`3opt`, which ignore `--epochs`; verify every compared cell completes with `status: ok`.
- [x] 2.3 Add a separate `bench/matrix/tier-distance-large.toml` for downloaded instances (`dsj1000`, `pla7397`, `pla33810`) with `download_data.sh` documented as a prerequisite; verify a fresh checkout's default comparison tier does not reference an untracked instance and therefore does not abort.
- [x] 2.4 Add the synthetic coordinate generator for n = 1k, 5k, 10k, 25k, 50k, 100k; verify it emits exactly the requested city count and a stable seed output.
- [x] 2.5 Add `scripts/bench-distance-scaling.sh` that writes one TSPLIB file per size, runs the CLI once per size in a fresh process under GNU time, and records peak RSS; verify it emits one row per size and records `OOM`/not-measured when the matrix exceeds RAM.
- [x] 2.6 Add the criterion construction-time benchmark in `benches/distance_matrix.rs` with a `[[bench]]` entry; verify `cargo bench --bench distance_matrix` runs the small sizes by default.
- [x] 2.7 Add a guard test asserting the coordinate matrix stores at most four bytes per pair (`<= 4 · n(n-1)/2`); verify it fails if a wider representation is introduced and does not trip on PR 9's `u16` EXPLICIT compaction.
- [x] 2.8 Run the matrix-only scaling baseline and record the pre-refactor numbers plus the Phase 1 refactor runtime bound in the baseline section of `docs/benchmarks/distance-source.md`; verify the matrix rows are reproducible from the documented command.
- [x] 2.9 Add a `task bench:distance` entry; verify it runs the comparison tier and the scaling script.
- [x] 2.10 Gate the 25k+ sizes behind the scaling script or `--ignored`; verify `cargo test` does not run them.
- [x] 2.11 Run the `matrix` config on the base commit and commit the JSONL to a tracked path outside `bench/runs/` (which is git-ignored); verify the file is in git and is the Phase 1 no-regression reference.

## 3. Phase 1 — Trait boundary (behaviour-preserving)

- [x] 3.1 Define `DistanceSource` with five required primitives, a `has_coordinates` default, and six shared method defaults; verify a `&dyn DistanceSource` test compiles and the conformance-suite skeleton runs.
- [x] 3.2 Implement the five primitives for `DistanceMatrix`, override `has_coordinates` to false in `new` (explicit weights) and true in `build`, and keep both the matrix-only accessors and the existing inherent query methods until PR 4; verify the existing `distance_matrix` unit tests pass unchanged and the workspace compiles with `TspProblem.distances` still concrete.
- [x] 3.3 Change `TspProblem.distances` to `Arc<dyn DistanceSource>`, add `new` and `from_source` (`from_points` lands with `DistanceBackend` in task 6.1), and remove the redundant inherent query methods in this same PR; verify all 80 existing `TspProblem::new` call sites compile.
- [x] 3.4 Add the position-invariant check to `from_source` (`num_cities == cities.len()`, `pos2city_id(i) == cities[i].id`); verify a mismatched source fails the debug assertion.
- [x] 3.5 Update `Solution::from_parts` to accept `&dyn DistanceSource` (`Solution::new` is unchanged); verify reported totals match the pre-change tests.
- [x] 3.6 Change the `&DistanceMatrix` helper parameters to `&dyn DistanceSource` across `bellman_karp`, `branch_bound`, `christofides`, `comparison`, `genetic_algorithm`, `graph`, `lin_kernighan`, `or_opt`, `savings`, `tabu_search`, and `three_opt`; verify `cargo test` is green.
- [x] 3.7 Change `build_evaluator` in `branch_bound` and `genetic_algorithm` to clone the `Arc` instead of deep-copying the matrix; verify their unit tests pass and no matrix clone remains.
- [x] 3.8 Change `nearest_unvisited` to take `&dyn Fn(usize) -> bool` and update `nearest_neighbor` plus the six test call sites; verify the NN determinism tests pass.
- [x] 3.9 Add `use DistanceSource` imports to every concrete caller (`tests/*`, `main.rs:736`, WASM `lib.rs:573`); verify each crate builds.
- [x] 3.10 Update the CLI, API, WASM, and Qt constructors to the new field type; verify each crate builds (WASM and Qt built separately).
- [x] 3.11 Run the `matrix` config against the tracked baseline from task 2.11; verify every tour cost is unchanged for deterministic solvers and the runtime is within the documented refactor bound.
- [x] 3.12 Verify every published-optimum test (att48, ulysses16, gr17, burma14, …) reports the same cost as before the refactor for deterministic solvers.
- [x] 3.13 Write ADR 010 recording the distance-source boundary (trait, backends, selection signal) and add it to the `docs/adr/` index; verify the index links it.
- [x] 3.14 Apply the PR-665 review fixes: make the default `tour_length` allocation-free, document the `distances_from` panic contract, and strengthen the trait skeleton test; verify the lib tests.

## 4. Lin-Kernighan metric fix (explicit behaviour change)

- [x] 4.1 Add an optional seed to `LKOptions` and thread it through the double-bridge RNG, updating `teeline-api/src/services/tsp_service.rs:177–184` and any TOML/CLI plumbing in the same PR; verify two runs with the same seed return the same tour.
- [x] 4.2 Replace `lin_kernighan`'s private `distance_matrix::from_cities` and its local `tour_distance` summation with `problem.distances`; verify seeded LK returns metric-correct tours on GEO and ATT fixtures and agrees across backends.
- [x] 4.3 Re-baseline LK's published numbers for GEO/ATT/CEIL_2D/EXPLICIT and record the change as a bug fix; verify the benchmark notes explain the difference.

## 5. On-the-fly backend and conformance

- [ ] 5.1 Implement `OnTheFly` primitives with position-indexed coordinates, an id→position index, and the four distance function pointers; verify it accepts all four coordinate types, rejects `Explicit`, and mirrors `DistanceMatrix::build`'s `n < 2` rejection.
- [ ] 5.2 Assert unique city ids in `OnTheFly::new`, mirroring `DistanceMatrix::new`; verify a duplicate-id input is rejected.
- [ ] 5.3 Add the shared conformance suite covering every backend, every distance type, and the edge/error paths (`distance_between(a,a)`, `distance_by_pos(p,p)`, unknown id in `tour_length`, unknown id in `distances_from`); verify `Matrix` and `OnTheFly` both pass.
- [ ] 5.4 Add pairwise equivalence tests for every city pair across all four distance types, including a synthetic CEIL_2D fixture; verify exact equality.
- [ ] 5.5 Add deterministic solver equivalence tests (`nn`, `sfc`, `2opt`, `or_opt`, `bhk`) against both backends; verify identical tours and costs.
- [ ] 5.6 Add a seeded LK equivalence test on a GEO fixture against both backends; verify identical tours and costs, and that the cost equals the GEO recomputation.
- [ ] 5.7 Verify `from_cities_unrounded` remains test-only and is not represented by either backend; verify the conformance suite excludes it explicitly.
- [ ] 5.8 Add a conformance case with 1-based and non-contiguous city IDs (for example 10, 20, 30); verify `pos2city_id`/`city_id2pos` round-trip, both backends agree, and the reported tour preserves the original IDs.

## 6. Phase 2 — Selection policy and backend comparison

- [ ] 6.1 Add `DistanceBackend`, `to_problem` keyed off `has_explicit_weights()`, and `from_points`; verify explicit weights + a coordinate `--distance-type` override still use the materialized backend.
- [ ] 6.2 Replace the `distance_matrix::build(..).unwrap()` panic with an error return; verify coordinate input + `--distance-type explicit` returns a descriptive error without panicking.
- [ ] 6.3 Add the threshold constant and boundary tests at threshold−1, threshold, and threshold+1; verify `Auto` picks the expected backend.
- [ ] 6.4 Extend `src/config.rs` to parse a top-level `distance_backend`, add the CLI flag, and add `tests/pipeline_config_test.rs` coverage; verify valid values parse, unknown values are rejected, and the CLI wins over TOML without a clap `default_value` shadowing it.
- [ ] 6.5 Wire the API, WASM, and Qt constructors to the shared policy constructor; verify each crate builds and one end-to-end solve per surface succeeds.
- [ ] 6.6 Fix the WASM scoring-only paths (`compare_tours_from_input`, `tour_distance_from_input`) to pick `Matrix` for explicit weights and `OnTheFly` for coordinates; verify an EXPLICIT TSPLIB input still scores in WASM.
- [ ] 6.7 Add the `on-the-fly` config family (`on-the-fly`, `on-the-fly-3opt`, `on-the-fly-fixed`) to `tier-distance.toml`, make every config in `tier-distance*.toml` pass `--distance-backend` explicitly, then run the comparison sweep with at least five runs per cell; verify the same cells appear under both backends, only `status: ok` rows are compared, and the report states the memory ratio and runtime ratio per family.
- [ ] 6.8 Set the per-family runtime tolerance and the memory expectation from the sweep using medians/distributions across the repeated runs, and record them in `docs/benchmarks/distance-source.md`; verify the numbers match the raw rows and no claim rests on a single run.
- [ ] 6.9 Add `scripts/check-distance-benchmarks.py` that reads the local JSONL and fails when a runtime or memory bound is violated; verify a deliberately slower run fails the check.
- [ ] 6.10 Flip the default to `Auto` only after tasks 6.8 and 6.9; verify the documented threshold equals the measured one.
- [ ] 6.11 Add the 100k-city SFC end-to-end measurement in a fresh process; verify it returns a valid tour with peak RSS below 1 GiB.
- [ ] 6.12 Document the flag, threshold, and memory/CPU trade-off; verify the documented commands run as written.
- [ ] 6.13 Document the manual per-release gate and keep the comparison run file out of `publish-benchmarks.sh`; verify the public leaderboard's configs are unchanged and the comparison report is linked from the benchmark docs.

## 7. Phase 3 — Candidate-list move evaluation for local search

- [ ] 7.1 Extend the KD-tree k-nearest candidate lists to `two_opt` and `or_opt` (LK already has them), bounding move evaluation rather than adding a `DistanceSource`; verify each city's candidate set on a fixture, address the narrow `n_nearest` default of 3, and use `has_coordinates()` to skip candidate pruning on EXPLICIT placeholder coordinates.
- [ ] 7.2 Verify `two_opt`, `or_opt`, and `lin_kernighan` still score moves with exact distances from the problem's source; verify each returns a valid tour.
- [ ] 7.3 Measure memory and runtime against the full-neighbourhood evaluation on `a280` and `att532`; verify the report gains before/after rows.
- [ ] 7.4 Add a quality test comparing candidate-list tours to the published optima (`a280` 2579, `att532` 27686) using seeded or median runs; verify the gap is within the documented tolerance.
- [ ] 7.5 Document candidate-list usage, its raw-Euclidean geo-ranking caveat, and its tolerance, and note the expected baseline updates for `2opt`/`or_opt`; verify the document matches the benchmark report.

## 8. Phase 4 — Compact EXPLICIT storage

- [ ] 8.1 Add width detection and `u16`/`u32` storage for EXPLICIT distances when all weights are non-negative integers; verify exact round-trips for 16-bit and 32-bit ranges.
- [ ] 8.2 Fall back to `f32` for non-integer or negative weights; verify every distance is preserved exactly.
- [ ] 8.3 Implement the trait primitives over the compact representation; verify the existing EXPLICIT tests pass.
- [ ] 8.4 Compare compact-path tour costs against the `f32` path; verify every cost is identical.
- [ ] 8.5 Measure EXPLICIT parse-time and stored peak RSS separately, evaluate a consuming `into_problem` to drop the `raw_distances.clone()`, and record sizes beyond the feasible parse limit as not-measured; verify the report states both figures.
- [ ] 8.6 Document the compaction rule and measured EXPLICIT memory reduction; verify the numbers match the benchmark.

## 9. Phase 5 — SFC at 100k through normal dispatch

- [ ] 9.1 Verify `sfc` selected through the normal solver dispatch at 100k (reusing the task 6.11 measurement) produces a valid tour and a reported cost equal to the recomputed tour length.
- [ ] 9.2 Verify there is no separate SFC entry point and the solver remains reachable only through the `Solvers` dispatch; verify the CLI and metadata paths both resolve it.
- [ ] 9.3 Update the SFC documentation and README to describe the 100k execution path; verify the documented command runs.

## 10. Integration verification

- [ ] 10.1 Run `cargo test --workspace` and the separate scaling/`--ignored` tasks; verify both are green, noting that `default-members = ["."]` otherwise skips the CLI and API.
- [ ] 10.2 Run `cargo clippy --workspace -- -D warnings`; verify no warnings.
- [ ] 10.3 Run `cargo fmt --check`; verify no diffs.
- [ ] 10.4 Run `openspec validate distance-source --strict`; verify it passes.
- [ ] 10.5 Build the WASM target, transpile with `jco`, and verify the ambient `teeline-wasm.d.ts` matches any changed export.
- [ ] 10.6 Build and smoke-test `teeline-qt`; verify a solve still renders progress.
- [ ] 10.7 Run `graphify update .`; verify the graph is current.
- [ ] 10.8 Re-run the documented comparison and scaling commands; verify the accepted numbers are reproducible.

## 11. Stacked PR delivery

- [ ] 11.1 Open PR 1 (`docs/constitution-distance-source`) against `master` containing only task 1.1; verify the diff is limited to `constitution.md`.
- [ ] 11.2 Open PR 2 (`bench/distance-backend`) against PR 1 for group 2; verify `--dry-run`, the scaling script, and the committed baseline run as documented.
- [ ] 11.3 Open PR 3 (`refactor/add-distance-source-trait`) against PR 2 for tasks 3.1–3.2; verify the trait and matrix impl compile with no `TspProblem` field change.
- [ ] 11.4 Open PR 4 (`refactor/use-distance-source`) against PR 3 for tasks 3.3–3.12; verify the no-regression sweep and that the diff is mechanical.
- [ ] 11.5 Open PR 5 (`fix/lk-declared-metric`) against PR 4 for group 4; verify the seeded LK tests and the re-baseline note.
- [ ] 11.6 Open PR 6 (`feat/on-the-fly-backend`) against PR 5 for group 5; verify the conformance and equivalence suites.
- [ ] 11.7 Open PR 7 (`feat/distance-backend-selection`) against PR 6 for group 6; verify the comparison sweep, thresholds, and 100k ceiling.
- [ ] 11.8 Open PR 8 (`perf/candidate-lists`) against PR 7 for group 7; verify the quality and memory report.
- [ ] 11.9 Open PR 9 (`perf/compact-explicit`) against PR 8 for group 8; verify exactness and the EXPLICIT memory report.
- [ ] 11.10 Open PR 10 (`docs/sfc-100k`) against PR 9 for groups 9–10; verify the full integration checks.
- [ ] 11.11 After each merge, retarget the next PR to the new base and verify its diff contains only its own scope.

## Workflow follow-up

- Archive the change once the stack has merged and the accepted benchmark numbers are published; Phases 3–5 may archive with the same change if completed, or be split into follow-up changes.
- Add the nightly small-instance smoke once the thresholds stabilize; it is not part of this change.
- `city-id-position` follows this stack after PR 10; do not start it against an intermediate branch.

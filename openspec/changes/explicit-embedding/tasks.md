# Tasks

## 1. Embedding module

- [x] 1.1 Add `src/tsp/embedding.rs` with `mds_embed(n, distance)` using `f64` internals, double-centering, zero-mean trigonometric start vectors (`cos`/`sin`), per-iteration re-centering, unshifted deflation with a recomputed shift, Gram–Schmidt against the first axis, and sign canonicalization for both axes; verify a unit test reconstructs a planar point set up to translation, rotation, and reflection.
- [x] 1.2 Add isotropic (square and a regular hexagon), collinear, high-aspect-ratio, non-Euclidean (`ring6_explicit`), repeated-derivation, and `n = 2` tests; verify every case is finite and deterministic, the isotropic case keeps two dimensions, and the collinear case does not fabricate a second axis.
- [x] 1.3 Register the module in `src/tsp/mod.rs` and document why classical MDS, why hand-rolled power iteration, the `f64` choice, the O(n²) scratch, the EXPLICIT-only scope, and the relationship to `scripts/lib/problem-minimap.mjs`; verify `cargo test -p teeline embedding` passes and the docs explain the limits.
- [x] 1.4 Write ADR 012 recording the derived-layout decision, the `has_real_coordinates` versus `DistanceSource::is_coordinate_metric` split, and the corrected power-iteration recipe, then add it to `docs/adr/README.md`; verify the index links it and notes the `city-id-position` ADR-number collision.

## 2. Parse-time embedding

- [x] 2.1 Add `has_real_coordinates` to `TspLibData`, defaulting to `true` in `new`, plus a builder that clears it; verify every existing `TspLibData::new` call site still compiles and reports real coordinates.
- [x] 2.2 In `process_lines`, when `cities.is_empty() && raw_distances.is_some()`, build the city list from `mds_embed` instead of `grid_coords` and clear the flag; verify `gr17` and `ring6_explicit` parse with `has_real_coordinates == false` and non-placeholder coordinates, while `bays29` and `bayg29` (display data) stay `true`.
- [x] 2.3 Verify `TspLibData::cities()`, `distance_matrix().point_at(pos)`, and `TspProblem::new(data.cities().to_vec(), data.distance_matrix()?)` agree on `gr17`; verify `tests/space_filling_curve_test.rs`'s `load_problem("gr17.tsp")` now gets the derived layout.
- [x] 2.4 Update the stale comments in `src/tsp/distance_matrix.rs` ("whose cities are TSPLIB grid placeholders"), `src/tsp/distance_source.rs`, and `docs/adr/010-distance-source.md`; verify no comment still claims EXPLICIT cities are always placeholders.

## 3. Problem construction and geometric solvers

- [x] 3.1 Add `LayoutPolicy { Derived, RequireReal }` with additive `to_problem_with_layout` and `into_problem_with_layout`; keep the existing constructors meaning `Derived`; verify the existing problem-construction tests pass unchanged.
- [x] 3.2 Make `RequireReal` return a descriptive error before building a problem from an instance without real coordinates; verify no `TspProblem` is returned and the error names the policy.
- [x] 3.3 Add acceptance tests: `sfc`, `fourier`, and `som` return valid permutations on `gr17`, `sfc`'s cost is lower than a tour built from the pre-change grid coordinates, `branch_bound` returns a valid tour on `gr17`, and `berlin52` keeps its exact tours.
- [x] 3.4 Correct `tests/three_opt_test.rs` to build `gr17` from `to_problem(DistanceBackend::Auto)` so the explicit weights are used, and seed the solver with the NN tour or relax the comparison to `<=`; verify both gr17 tests pass.
- [x] 3.5 Document the derived-layout behaviour and its quality caveat on the `sfc`, `fourier`, and `som` algorithm pages; verify each page states what happens when coordinates are absent.

## 4. Lin-Kernighan exact candidates

- [x] 4.1 Add `CandidateList::build_from_distances(distances, k)` that ranks neighbours by `distance_by_pos` with a city-ID tie-break and skips non-finite distances using `f32::total_cmp` plus a finite check; verify a unit test where geometric and matrix order differ and a `NaN` case.
- [x] 4.2 Use the matrix path in `lin_kernighan` whenever `!distances.is_coordinate_metric()`, including EXPLICIT instances with display data; verify `lk` on `gr17` returns a valid tour, beats the placeholder-coordinate baseline, and is deterministic with a fixed seed, and that `bayg29`/`bays29` still return valid tours.
- [x] 4.3 Add a regression test that `two_opt` and `or_opt` still take the full-scan path on EXPLICIT input; verify the tests assert the no-candidate branch.
- [x] 4.4 Document LK's exact-matrix-candidate fallback on its algorithm page; verify the page states that it covers all EXPLICIT instances and does not depend on layout quality.

## 5. Benchmark and web provenance

- [ ] 5.1 Stamp `layout: "real" | "derived"` on each raw row in `scripts/bench-matrix.sh`; verify a `gr17` row is `derived` and a `berlin52` row is `real`.
- [ ] 5.2 Extend `scripts/publish-benchmarks.sh` to treat `layout` as optional (old JSONL and `tests/e2e/publish-benchmarks.bats` must keep working), fall back to `minimapDerived` parsed from `docs/problems/*.md`, and emit `layout` on both `problems/{dataset}.json` and `algorithms/{solver_id}.json` under `results.{dataset}`; verify both shards carry the field.
- [ ] 5.3 Update `teeline-web/src/lib/benchmark.ts` and its tests, plus `ProblemResults` and `AlgorithmBenchmarks`, to read and render the derived-layout indicator; verify the `gr17` algorithm page labels the row.
- [ ] 5.4 Document the additive field in `docs/benchmarks/schema.md` (no `/bench/v3/` bump) and verify the doc matches the emitted JSON.

## 6. Integration checks

- [ ] 6.1 Run `cargo test --workspace` and verify it is green.
- [ ] 6.2 Run `cargo clippy --workspace -- -D warnings` and verify no warnings.
- [ ] 6.3 Run `cargo fmt --check` and verify no diffs.
- [ ] 6.4 Add a budget-limited integration test that every registered solver returns a valid permutation and finite cost on `gr17`, setting small budgets for all ten `AppOptions` structs (`heuristic`, `sa`, `ga`, `cs`, `fpa`, `lk`, `fourier`, `sfc`, `som`, `aco`); verify it completes and passes.
- [ ] 6.5 Compare CLI output before and after: coordinate fixtures (`berlin52`, `att48`, `burma14`) must be byte-identical; `gr17` must improve; `bayg29`/`bays29` LK tours may change and must be re-measured and reflected in `bench/solvers.json` notes if a recorded cost moves.
- [ ] 6.6 Build the out-of-workspace `teeline-wasm` and `teeline-qt` targets and verify both build and show the derived layout on `gr17`.
- [ ] 6.7 Run `./tests/bats/bin/bats tests/e2e/bench-matrix.bats tests/e2e/publish-benchmarks.bats`, plus `npm --prefix teeline-web test` and `npm --prefix teeline-web run build`; verify the harness, publisher, and web changes pass.
- [ ] 6.8 Run `graphify update .` and verify the graph is current.

## Workflow follow-up

- Land the constitution input-contract amendment as its own PR **before** this change (constitution §8).
- Rebase `candidate_list.rs` against the in-flight `city-id-position` change; whichever lands second adjusts the newtypes, and `city-id-position` must renumber its planned ADR 011.
- Decide whether to align `scripts/lib/problem-minimap.mjs` with the corrected MDS recipe or document the divergence.
- Open the PR against `master` as `feat/explicit-embedding`; archive after the merge and after the `gr17`/`berlin52` comparisons are verified.

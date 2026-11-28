# Tasks

## 1. Shared Held–Karp path DP

- [x] 1.1 Add `src/tsp/held_karp.rs` with `held_karp_path(matrix: &[f32], m: usize) -> Option<(f32, Vec<usize>)>`: packed `(m + 2)²` local matrix (`0..m` interior, `m` start, `m + 1` end), iterative mask DP, `INFINITY` for unreached states, and an `m <= 26` bound. Verify: unit tests cover `m = 0` (direct edge), `m = 1`, and the bound failure.
- [x] 1.2 Implement backward reconstruction by re-computing the argmin with the forward first-strict-`<` tie-break (no parent table, no tolerance) and declare the module from `src/tsp/mod.rs`. Verify: two calls on the same matrix return the identical order.
- [x] 1.3 Add the brute-force oracle: random symmetric and asymmetric `(m + 2)²` matrices for `m = 3..=8`, including ties and integer-valued entries. Verify: the DP cost equals the minimum over all interior permutations and the returned order realizes it; `cargo test -p teeline held_karp` passes.

## 2. Bellman–Held–Karp refactor

- [x] 2.1 Capture the pre-refactor cost and route for `burma14`, `gr17`, `ulysses16` and a few random 3–12-city instances. Verify: the captured values are recorded in the PR description for the post-refactor comparison.
- [x] 2.2 Rewrite `bellman_karp::solve` onto `held_karp_path` (local `i < n - 1` → `Position::new(i)`, local `n - 1`/`n` → the anchor), delete `solve_bhk`, `read_optimal_route`, `approx`, `show_table`, `FlagSet`, `DPTable`, `UNKNOWN_DISTANCE`, and keep the `CityChange`/`PathUpdate`/`Done` messages. Verify: post-refactor cost equals the captured cost, every route is a valid permutation, and the existing `bellman_karp` unit tests, `tests/solvers_integration.rs`, and `tests/distance_source_conformance.rs` (burma14 GEO) pass.
- [x] 2.3 Bump `docs/algorithms/bellman-held-karp.md` version and add a changelog row for the refactor (forward route order, tie-breaking, `--verbose` table removed). Verify: the page renders and its Complexity row still matches `nav-data.ts` in the web test.

## 3. POPMUSIC window and splice

- [x] 3.1 Add `src/tsp/popmusic.rs` with `DEFAULT_WINDOW = 14`, `MAX_PASSES = 4`, the standard `solve(problem, _opts, progress_tx, init_tour)` signature, the `n <= 2` return, `R = min(14, n)`, the `m < 2` guard, warm-start consumption or an internal `nearest_neighbor::solve` seed (with `None` progress), and `PathUpdate`/`Done` output. Verify: unit tests for `n = 0..4` return valid tours without panicking, a supplied warm start is used, and the channel receives path updates plus exactly one `Done`.
- [x] 3.2 Implement window extraction with modular indexing, the per-window local matrix, the `held_karp_path` call, the same-order `f32` current-cost sum with the `eps = 1e-6 · max(cur, 1)` improvement guard and identity skip, and the write-back into the same slots. Verify: unit tests assert the result is a permutation, the window endpoints are unchanged, the tour cost never increases, a wrap-around window across index 0 is handled, and a deliberately bad seed improves at `n = 5` and `n = 6`.

## 4. POPMUSIC scan and termination

- [x] 4.1 Implement the two-offset scan (`0` and `R / 2`, stride `R`), one pass per pair of offsets, the `MAX_PASSES` cap, and the early stop when a pass splices nothing. Verify: unit tests assert the early stop fires after a non-improving pass, the pass cap is respected, and running the solver on its own output makes zero splices and returns the same tour.
- [x] 4.2 Add determinism and degenerate-case unit tests (already-optimal small tour, repeated runs). Verify: `cargo test -p teeline popmusic` passes and two runs on `berlin52` return the same route.

## 5. Seeding and pipeline integration

- [x] 5.1 Add `tests/popmusic_test.rs` with fixture loading for `berlin52`, `gr17` (EXPLICIT) and `a280`. Verify: each run returns a valid permutation, a finite cost, and a cost equal to the recomputed `tour_length`; `cargo test --test popmusic_test` passes.
- [x] 5.2 Add pipeline tests. Verify: `pipeline(nn, popmusic)` is no worse than `nn` on `berlin52` and `a280`, and a distinct warm-start tour is the one POPMUSIC starts from.

## 6. Core registration

- [x] 6.1 Register `Solvers::Popmusic` in the module declarations, enum, `variants()`, `FromStr`, `all_meta()`, `SOLVER_LIST` (25 → 26), `solve_with_context`, and `auto_expand_with_nn`. Verify: `popmusic`/`pop-music` resolve to the same solver; `teeline solve popmusic -i tests/fixtures/berlin52.tsp` returns a valid tour; the auto-expand tests cover popmusic and assert it is not shuffle-seeded.
- [x] 6.2 Add the solver row to `README.md` and `AGENTS.md` (file, algorithm, aliases). Verify: both tables list the solver and link the new algorithm page.

## 7. Web, metadata and documentation

- [x] 7.1 Add `docs/algorithms/popmusic.md` (frontmatter with `id: "popmusic"`, `version: "1.0.0"`, `hasExplainer: false`; description, complexity `O(passes · n · R · 2^R)`, options "none", usage, references, changelog). Verify: markdownlint passes and the Complexity row matches the nav entry.
- [x] 7.2 Add the `bench/solvers.json` entry (`family: "meta"`, `cli: "popmusic"`, `doc: "popmusic"`) and `teeline-web/src/nav-data.ts` (`SOLVER_META.popmusic` plus the Metaheuristic group), leaving `EXPLAINER_SOLVERS` untouched. Verify: the web `nav-data.test.ts` and `solver-index.test.ts` pass and `tsc --noEmit` is clean.
- [x] 7.3 Add a `recommendation_for` hint for `popmusic` in `teeline-wasm/src/lib.rs`. Verify: the WASM tests pass and the algorithm list includes the solver as a metaheuristic with no parameters.

## 8. Benchmark measurement and publication

- [x] 8.1 Build release and run `scripts/bench-matrix.sh bench/matrix/tier-a.toml --solver popmusic`; record the chosen `R` and window/pass timings. Verify: every Tier A instance returns a valid tour, no run times out, and the meta 300-second budget holds.
- [ ] 8.2 Measure determinism over repeated runs and set `deterministic: true` with `determinism_evidence` in `bench/solvers.json`; publish via the benchmark-solver skill. Verify: the recorded evidence matches the measurement and the published gap data is consistent with the tier.

## 9. Integration verification

- [x] 9.1 Run `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, the full `cargo test` suite, and the web tests/typecheck. Verify: all pass.
- [x] 9.2 Run `openspec validate add-popmusic-solver --strict`. Verify: validation passes with no warnings.

## Workflow follow-up

- Archive the change after review and benchmark publication.
- Update the `popmusic` row in `docs/algorithm-candidates.md` from accepted to implemented.

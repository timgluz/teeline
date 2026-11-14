# Tasks

## 1. Curve geometry and normalization

- [x] 1.1 Implement Hilbert `xy2d` with `u64` indices and Morton bit interleaving in `src/tsp/space_filling_curve.rs`, with unit tests asserting bijection and Manhattan-adjacency of consecutive indices for `p = 1..=6`. Verify: `cargo test -p teeline space_filling_curve` passes both property tests.
- [x] 1.2 Implement normalization in `f64`: bounding box, `uniform` and `stretch_to_square` modes, `EPSILON` zero-span guard, round-and-clamp to `[0, 2^p - 1]`, and order validation for `1..=16`. Verify: unit tests for max-edge clamp, identical/collinear points, rejection of `p = 0` and `p = 17`, and the `p = 16` boundary pass with `cargo test -p teeline space_filling_curve`.
- [x] 1.3 Write `docs/algorithms/space-filling-curve.md` covering the algorithm, the full parameter table, seam/warm-start trade-offs, and a runnable CLI example. Verify: `markdownlint docs/algorithms/space-filling-curve.md` passes and the documented example command outputs a valid tour.

## 2. Tour assembly

- [x] 2.1 Sort cities by `(curve_index, tie_key, input_index)` and implement the `input_order`, `lexicographic`, and `nearest_prev` tie-breakers. Verify: unit tests assert each tie-breaker's ordering and repeat determinism; `cargo test -p teeline space_filling_curve` passes.
- [x] 2.2 Implement rotation (0/90/180/270), mirroring, and the multi-orientation best-of ensemble that keeps the shortest tour. Verify: unit tests assert every transform yields a valid permutation, the ensemble cost is `<=` every single orientation, and two identical runs match; `cargo test -p teeline space_filling_curve` passes.
- [x] 2.3 Handle 0/1/2-city, identical, and collinear inputs; emit the placeholder `PathUpdate` + `Done` progress messages; return a `Solution` whose `total` matches the recomputed tour length. Verify: unit tests cover each degenerate case without panics and assert cost consistency; `cargo test -p teeline space_filling_curve` passes.

## 3. Core registration and options

- [x] 3.1 Add `Solvers::SpaceFillingCurve`, the `sfc`/`hilbert`/`space_filling_curve` aliases in `variants()` and `FromStr`, the `SolverMeta` entry, and the dispatch arm; bump `SOLVER_LIST` from `[SolverInfo; 22]` to `23` and add its constructive-category entry. Verify: `cargo build -p teeline` succeeds, `variants_all_round_trip_through_fromstr_except_presets` still passes, and a new alias-resolution test passes.
- [x] 3.2 Add `SfcOptions` (`Default`, `validate`, `from_toml`, `from_cli`) and `AppOptions.sfc`, following the `FourierOptions` pattern. Verify: unit tests cover defaults, each validation error, and a TOML round-trip; `cargo test -p teeline` passes.
- [x] 3.3 Wire `[sfc]` stage parsing in `src/config.rs`, including per-solver sub-table validation and the valid-field lists. Verify: config tests accept a valid `[sfc]` table and reject `[sfc]` on a different solver; `cargo test -p teeline config` passes.
- [x] 3.4 Add `sfc` to the solver tables in `README.md`, `CLAUDE.md`, and `AGENTS.md` (aliases, category, complexity, options). Verify: `markdownlint README.md CLAUDE.md AGENTS.md` passes and each table lists `sfc`. (No `CLAUDE.md` exists in this repo; the guidance file is `AGENTS.md`.)

## 4. CLI

- [x] 4.1 Add CLI flags for curve type, order, aspect mode, rotation, mirror, multi-orientation, and tie-breaker; wire the `solver_options_from_args` arm and help text. Verify: `cargo test -p teeline-cli` passes a new test exercising every flag, and `target/debug/teeline solve sfc --help` lists all flags.

## 5. Pipeline

- [x] 5.1 Add the `SpaceFillingCurve` arm to `stage_warnings` for non-first stages and confirm it is absent from `auto_expand_with_nn` and `auto_expand_with_shuffle`. Verify: unit tests assert a warning at stage 1 and none when first; `cargo test -p teeline pipeline` passes.

## 6. HTTP API

- [x] 6.1 Add `SfcConfig` to `teeline-api/src/models/request.rs`, map it in `services/tsp_service.rs`, and export it from `openapi.rs`. Verify: `cargo test -p teeline-api` passes and a request selecting `sfc` with SFC configuration maps to the expected options.

## 7. WASM bridge

- [x] 7.1 Add the `sfc` description, `constructive` category, parameter list, and `AppOptions.sfc` builder in `teeline-wasm/src/lib.rs`; update the WIT export if the option shape changes. Verify: `task build:wasm` succeeds and `cargo test --manifest-path teeline-wasm/Cargo.toml` passes.
- [x] 7.2 Update `teeline-wasm/tests/wasm_component.rs`: add `sfc` to the alias list, run the solver, and assert its `constructive` kind and parameter list. Verify: `cargo test --manifest-path teeline-wasm/Cargo.toml` passes all SFC assertions.
- [x] 7.3 If the WIT changed, regenerate bindings with `task wasm:bindings` and commit `teeline-wasm/js-bindings/teeline_wasm.js`. Verify: `task web:build` reports no `[MISSING_EXPORT]` for the new export.

## 8. Web options

- [x] 8.1 Add the SFC types/parameters to `teeline-web/src/teeline-wasm.d.ts` (the ambient declaration shadows the generated one) and `solver-options.ts`; add the algorithm doc/nav entry the web index requires. Verify: `npx --prefix teeline-web tsc --noEmit --project teeline-web/tsconfig.json` passes and `task web:test` passes.

## 9. Integration and quality

- [x] 9.1 Add `tests/space_filling_curve_test.rs` using only TSPLIB instances under 2,000 cities with published optima: `berlin52` (52 cities, optimum 7542) and `att532` (532 cities, optimum 27686, multi-orientation enabled) must return valid permutations with cost `< 1.5 ×` the published optimum, and pipelines `sfc → 2opt` and `sfc → lk` on `berlin52` must return a tour no worse than the SFC tour. Verify: `cargo test --test space_filling_curve_test` passes.
- [x] 9.2 Register `sfc` in `bench/solvers.json` (`family: "constructive"`, doc `space-filling-curve`) and any matrix coverage entry that enumerates solvers. Verify: `task bench:tierA -- --only sfc` (or the preliminary tier) includes the `sfc` row, and `task web:test` still passes the solver-index consistency checks.

## 10. Integration gate

- [x] 10.1 Run the full repo gate: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo test`, `task build:wasm`, `cargo test --manifest-path teeline-wasm/Cargo.toml`, `npx --prefix teeline-web tsc --noEmit --project teeline-web/tsconfig.json`, and `task web:test`. Verify: every command exits `0` and the pre-commit hook is green.

## 11. Code-review fixes

- [x] 11.1 Make `lexicographic` break same-cell ties on the original coordinates and replace the inconsistent-key test with a consistent same-cell case. Verify: `cargo test -p teeline space_filling_curve` passes.
- [x] 11.2 Sanitize invalid `SfcOptions` values inside the public `solve` (clamp order/rotation/multi-orientation) and add a no-panic test. Verify: `cargo test -p teeline space_filling_curve` passes.
- [x] 11.3 Cap `nearest_prev` buckets at 64 cities, falling back to input order to keep O(n log n), and add the oversized-bucket test. Verify: the test passes and the spec/doc state the guard.
- [x] 11.4 Add `require_equals(true)` and value parsers to the SFC CLI flags, rename the generic flags to `--sfc-*`, and add boundary/negative tests. Verify: `cargo test -p teeline-cli` passes and `teeline solve --multi-orientation sfc -i …` works.
- [x] 11.5 Harden the WASM bridge with saturating conversions and a 90° rotation step; add non-default and invalid-option tests. Verify: `cargo test --manifest-path teeline-wasm/Cargo.toml` passes.
- [x] 11.6 Add OpenAPI example hints to `SfcConfig` and numeric-rejection/partial-fallback API tests. Verify: `cargo test -p teeline-api` passes.
- [x] 11.7 Add the `tracing::info!` start log, the `init_tour` test, and move the re-export next to its module. Verify: `cargo fmt -- --check` and `cargo test -p teeline` pass.
- [x] 11.8 Re-run the full gate (fmt, clippy, tests, wasm build/tests, tsc, web tests). Verify: every command exits `0`.

## Workflow follow-up

- Archive the change after review and a green integration gate.
- Verification stays limited to TSPLIB instances under 2,000 cities with published optima until #647 (distance-matrix memory) lands; 100k support and measurement are tracked there.

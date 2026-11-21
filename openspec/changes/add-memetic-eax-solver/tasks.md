# Tasks

## 1. Tour graph and candidate lists

- [x] 1.1 Add `src/tsp/memetic_eax.rs` with a fixed-degree undirected tour graph (two neighbour slots per city, O(1) edge lookup and removal) and tour↔graph conversion in dense positions. Verify: unit tests assert degree two for every city, edge add/remove round-trips, and a tour→graph→tour round-trip on a small instance.
- [x] 1.2 Add candidate-list construction clamped to `min(candidate_k, n - 1)`, using the coordinate builder for coordinate metrics and the distance-matrix builder otherwise. Verify: unit tests cover clamping, `n < 2`, and EXPLICIT input ranked by matrix distance.

## 2. AB-cycle decomposition and crossover

- [x] 2.1 Implement union-multigraph construction and AB-cycle extraction with A/B origin tags. Verify: unit tests on hand-built parent pairs assert every cycle alternates origin and has even length.
- [x] 2.2 Apply selected cycles as a symmetric difference and assert degree two after each application. Verify: unit tests assert the resulting edge set equals the symmetric difference and that shared edges are never treated as changes.
- [x] 2.3 Handle identical parents with a double-bridge perturbation and polish. Verify: a test with identical parents returns a valid tour without looping on an empty cycle set.

## 3. Population initialization

- [x] 3.1 Implement randomized constructive starts (top-3 randomized nearest neighbour and shuffled tours) polished with candidate-restricted 2-opt, plus the warm-start seeded fraction. Verify: tests assert the seed tour is present in the population and every member is a valid tour.
- [x] 3.2 Implement the distinct-local-optima fallback and population-size clamping. Verify: tests on an instance with fewer optima than `pop_size` still fill a valid population, and `pop_size` of 0 and 1 are handled.

## 4. Subtour patching

- [x] 4.1 Implement component detection and greedy smallest-component bridging with candidate edges, plus the full-scan fallback when no candidate edge crosses components. Verify: tests assert degree two is preserved, one component remains, and a no-candidate-case test exercises the fallback.
- [x] 4.2 Add debug-build invariant checks after patching and polish. Verify: tests under `debug_assertions` panic on a deliberately disconnected graph.

## 5. Dirty-edge local polish

- [x] 5.1 Implement candidate-restricted 2-opt seeded by bridge endpoints and propagated from accepted improving swaps. Verify: tests assert polish never increases cost, leaves a single cycle, and evaluates a dirty vertex before untouched vertices.
- [x] 5.2 Reuse preallocated scratch buffers for the pass. Verify: code review shows no per-move allocation in the inner loop, and the benchmark task records the measured elapsed time.

## 6. E-set strategies, pairing, replacement, termination

- [x] 6.1 Implement `single_ab`, `rand`, and `hybrid` E-set selection, including the `hybrid` switch after `stall_epochs` stagnant generations. Verify: tests assert each strategy selects a non-empty E-set and that `hybrid` switches.
- [x] 6.2 Implement shuffled distinct pairing with population edge cases. Verify: tests for population sizes 0, 1, 2, odd and even assert no member is paired with itself.
- [x] 6.3 Implement family child pools, strict-improvement replacement, global-best and stagnation tracking, and seeded RNG. Verify: tests assert equal-cost children do not replace, cheaper children do, the 2× stagnation stop fires, and two runs with the same seed match.

## 7. Solver entry and reconstruction

- [x] 7.1 Implement the `solve(problem, opts, progress_tx, init_tour)` entry, O(n) tour reconstruction, `Solution` cost consistency, `PathUpdate`/`Done` progress, and `n < 4` handling. Verify: tests assert a complete permutation, cost equal to `tour_length`, and the cyclic order for tiny instances.
- [x] 7.2 Emit `tracing` snapshots (generation, global best, stall count, cycle count, components before patching, bridges added). Verify: a test subscriber captures at least one generation event.

## 8. Options and core registration

- [ ] 8.1 Add `MemeticEaxOptions` (`Default`, `validate`, `from_toml`, `from_cli`) and `AppOptions.memetic_eax`. Verify: unit tests cover defaults, each validation error, and a TOML round-trip.
- [ ] 8.2 Register `Solvers::MemeticEax` in the enum, `variants()`, `FromStr`, `SolverMeta`, `SOLVER_LIST` (23 → 24), `AppOptions`, dispatch, and confirm absence from both `auto_expand_*` lists. Verify: alias round-trip tests and `cargo test -p teeline` pass.

## 9. TOML and CLI

- [ ] 9.1 Wire the `[memetic_eax]` stage table in `src/config.rs`, including valid-field lists and the per-solver belongs check. Verify: config tests accept a valid table and reject it on a different solver.
- [ ] 9.2 Add CLI flags (`--epochs`, `--pop-size`, `--children-per-pair`, `--strategy`, `--stall-epochs`, `--candidate-k`, `--seed`) and the `solver_options_from_args` arm. Verify: a CLI test exercises every flag and `teeline solve memetic_eax --help` lists them.

## 10. Pipeline

- [ ] 10.1 Consume `init_tour` as a seeded population member; confirm the solver is absent from `auto_expand_with_nn`, `auto_expand_with_shuffle`, and `stage_warnings`. Verify: a pipeline test asserts `pipeline(nn, memetic_eax)` is no worse than `nn`, and the auto-expand tests assert no prefix.

## 11. HTTP API

- [ ] 11.1 Add `MemeticEaxConfig` to `models/request.rs`, the `SolverConfigs` field, the `make_app_options` mapping, and the `openapi.rs` export. Verify: `cargo test -p teeline-api` passes and a request carrying `memetic_eax` config maps to non-default options.

## 12. WASM bridge

- [ ] 12.1 Add the solver to `build_opts`, `kind_for` (metaheuristic), `params_for_solver`, and `recommendation_for`, and update the WIT export if the option shape changes. Verify: `cargo test --manifest-path teeline-wasm/Cargo.toml` passes with list/params/solve assertions.
- [ ] 12.2 Regenerate JS bindings if the WIT changed. Verify: `task build:wasm` succeeds and `task web:build` reports no missing export.

## 13. Web

- [ ] 13.1 Add `docs/algorithms/memetic-eax.md` with `id: "memetic_eax"` and a Complexity row matching `SOLVER_META`; add `SOLVER_META`, the Metaheuristic `SOLVER_GROUPS` entry, bump the solver count 22 → 23, and update `teeline-wasm.d.ts` and `solver-options.ts`. Verify: `npx --prefix teeline-web tsc --noEmit --project teeline-web/tsconfig.json` and `task web:test` pass, including the solver-index/nav-data cross-checks.

## 14. Documentation and benchmark registration

- [ ] 14.1 Add the solver to the `README.md` and `AGENTS.md` solver tables. Verify: `markdownlint README.md AGENTS.md` passes and both tables list `memetic_eax`.
- [ ] 14.2 Register in `bench/solvers.json` (`family: "meta"`, `deterministic: false`, `doc: "memetic-eax"`), add the missing `kroA100` optimum to `bench/optima.json`, and reconcile the EUC_2D metric note on `berlin52`/`a280` with the current rounding behavior. Verify: `task web:test` stays green and a dry-run publish produces a non-null kroA100 gap.

## 15. Benchmarks and integration quality

- [ ] 15.1 Add integration tests for aliases, CLI/TOML options, pipeline use, and ATSP rejection through the public API. Verify: `cargo test --test memetic_eax` passes.
- [ ] 15.2 Run five-seed benchmarks on `berlin52`, `kroA100`, `a280`, and `dsj1000` within the 300-second meta budget and record machine, build mode, seeds, and elapsed time. Verify: `task bench:tierA -- --only memetic_eax` completes and the report states the measured gaps against the targets.

## 16. Integration gate

- [ ] 16.1 Run the full gate: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo test`, `task build:wasm`, `cargo test --manifest-path teeline-wasm/Cargo.toml`, `npx --prefix teeline-web tsc --noEmit --project teeline-web/tsconfig.json`, and `task web:test`. Verify: every command exits 0.

## Workflow follow-up

- Archive the change after review and a green integration gate.
- Track real web-explainer telemetry for the solver as a separate follow-up.

---
id: 019fdd09-4d32-7c72-84b2-f52f5a1d0b5a
slug: tasks/greedy-edge-construction
title: "Greedy Edge Construction Solver"
type: task
status: completed
priority: medium
tags: [solver, algorithm, feature]
---

## Overview

Add a new deterministic, parameter-free constructive TSP solver: greedy edge construction. Sorts all pairwise edges shortest-to-longest and greedily accepts each edge unless it would give a city degree 3+ or close a sub-cycle before all n cities are covered (Kruskal-style construction, using union-find for the degree/cycle checks). Intended as a better drop-in seed than `nn` ahead of `two_opt`/`lin_kernighan` in a pipeline.

Full design plan (reviewed by Opus, approved by user): `/home/timgluz/.claude/plans/i-would-like-to-cheeky-pine.md`

## Goals

- Implement `src/tsp/graph.rs`: reusable `UnionFind`, `sorted_edges`, `hamiltonian_cycle_to_path` primitives (`pub(crate)`).
- Implement `src/tsp/greedy_edge.rs`: the solver itself (aliases `greedy_edge` / `gec`).
- Register the solver across `src/tsp/mod.rs` (enum, variants, FromStr, all_meta, SOLVER_LIST, dispatch) and `teeline-wasm/src/lib.rs` (`kind_for`).
- Wire it into the pipeline (TOML `[[stage]] solver = "greedy_edge"`) — verified no `config.rs` code changes needed, just tests.
- Add unit + integration test coverage (including a randomized property test for `select_edges` and a pipeline-seed regression test).

## Acceptance Criteria

- [x] `src/tsp/graph.rs` created with `UnionFind`, `sorted_edges`, `hamiltonian_cycle_to_path`, each with unit tests
- [x] `src/tsp/greedy_edge.rs` created with `solve()` matching the `christofides.rs`/`nearest_neighbor.rs` signature convention
- [x] `select_edges` accept rule implemented and covered by a randomized property test (n=4..12)
- [x] Solver registered in all `src/tsp/mod.rs` touch points (enum, variants, FromStr, all_meta, SOLVER_LIST bumped to 20, dispatch match, stage_warnings)
- [x] `teeline-wasm/src/lib.rs` `kind_for()` updated
- [x] `tests/greedy_edge_test.rs` integration tests passing on berlin52 (validity, cost-matches-reported, empirical quality ceiling measured post-implementation, pipeline-seed-for-two_opt)
- [x] `src/config.rs` tests confirming TOML pipeline stage parsing works
- [x] `cargo test`, `cargo clippy -p teeline -p teeline-cli -- -D warnings`, `cargo clippy -p teeline-api -- -D warnings`, `cd teeline-wasm && cargo check` all pass
- [x] Manual CLI smoke tests pass (`solve gec`, `solve greedy_edge --verbose`, `solvers --output-format json`, `pipeline --steps gec,two_opt`)
- [x] CLAUDE.md solver table updated
- [x] Follow-up GitHub issue filed for deferred teeline-web work: https://github.com/timgluz/teeline/issues/392

## Context

- Branch: `feat/greedy-edge-construction`
- Deferred: teeline-web docs/explainer/landing-page surface — tracked in issue #392, not in this task.
- Algorithm design originated in a prior brainstorming session (not a GitHub issue).

## Progress Log

### 2026-08-07 (post-completion code review)

A max-effort automated code review of this branch's diff (`master...feat/greedy-edge-construction`) found and fixed 2 real bugs that the completion evidence above missed, plus 4 smaller cleanups. Confirms the lesson already on file in `feedback_verify_build_output_not_just_exit_codes`: green `cargo check`/`cargo test --workspace` is not the same as green *everywhere the change has effect* — `teeline-wasm` was excluded from the workspace, so its own test suite was never actually executed.

**Bugs fixed:**
- `teeline-wasm/tests/wasm_component.rs:426` — `test_list_algorithms_returns_all_solvers` hardcoded `assert_eq!(algorithms.len(), 19, ...)`. This diff bumped `SOLVER_LIST` to 20 entries but never updated this test, because `teeline-wasm` is excluded from the main workspace (`exclude = ["teeline-wasm"]` in root `Cargo.toml`) and the completion evidence only ran `cargo check` there, not `cargo test` — so the count mismatch compiled cleanly and was never executed. Fixed: `19`→`20`, added `"gec"` to the expected-id list. (Could not execute this specific test in the review sandbox due to an unrelated pre-existing `wasip2`/`io-lifetimes` toolchain gate on `--tests`; confirmed via `cargo check` that the plain lib target is unaffected, and the fix is a 2-line static-count/list correction verified by reading `list_algorithms()` → `list_solvers()` → `&SOLVER_LIST`.)
- `teeline-api/src/services/tsp_service.rs:81` — this diff's bundled fix making `Solvers::from_str("tabu")` resolve (previously only `"tabu_search"` worked, despite `SolverMeta.alias` already advertising `"tabu"`) had a side effect: `make_app_options()` maps solver-name strings to config sub-structs by hand and only recognised `"tabu_search"`, so a `/solve` or `/pipeline` request using `"solver": "tabu"` now resolves to `Solvers::TabuSearch` successfully but silently drops any `configs.tabu.heuristic` override, falling back to defaults with no error. Fixed: `"tabu" | "tabu_search" => ...`, matching the pattern already used for `nn`/`2opt`/`3opt`/`pso`/`gsa` in the same match. Added a regression test: `test_make_app_options_tabu_alias_applies_same_config_as_tabu_search`.

**Cleanups fixed:**
- `src/tsp/graph.rs` `sorted_edges()` — the sort comparator (`partial_cmp(...).unwrap_or(Equal)`) panics with "user-provided comparison function does not correctly implement a total order" when fed NaN distances (reproduced independently: TSPLIB's `f32::from_str` coordinate parsing accepts the literal token `"nan"`, and a deterministic large-`n` repro triggered the panic in ~4% of trials). Switched to `sort_unstable_by(|a,b| a.0.total_cmp(&b.0))` — `total_cmp` is a real total order over all f32 bit patterns (never panics), and the unstable sort is also cheaper (no tie-break stability is required; the existing property test already forces duplicate-distance ties).
- `src/tsp/graph.rs` `hamiltonian_cycle_to_path()` — the cycle-walk's `.find(|&x| x != prev).unwrap_or(adj[cur][0])` fallback contradicted its own doc comment ("panics rather than silently returning a corrupt/duplicated path"): a parallel edge between the same two positions would silently produce a wrong/duplicated path instead of panicking. Not reachable via the current sole caller (`select_edges` never emits duplicate edges), but a real gap for the future reuse this module's own header comment invites ("a future MST-via-Kruskal rewrite of christofides... or a savings-algorithm solver"). Changed the fallback to `panic!` with a clear message.
- `src/tsp/pipeline.rs` `stage_warnings()` — the new `GreedyEdge` warm-start-discard warning copy-pasted a second `if i > 0 && *solver == X` block instead of extending the existing `NearestNeighbor` one. Consolidated both into one `match` (mirroring the style already used 2 lines below for `BellmanKarp`/`BranchBound`). Note for a future task: `Christofides`, `Fourier`, and `KohonenSom` also unconditionally ignore `_init_tour` and get no warning either — this consolidation didn't extend coverage to them (that's a behavior change beyond this diff's own lines, deferred).

**Reported but not fixed (judgment calls, see review rationale):**
- `select_edges()`'s `uf.connected()`-then-`uf.union()` redundant `find()` calls: tried removing the redundant call, but that makes `UnionFind::connected` dead code under plain `cargo build` (only test code calls it after the change) which trips `-D warnings`. Reverted — not worth the tradeoff for a handful of near-O(1) `find()` calls.
- `sorted_edges()` recomputing distances via `distance_by_pos()` instead of consuming `DistanceMatrix::distances()`'s already-flat array directly — real redundant work, but reordering the loop to match the flat array's storage order risks a silent per-pair mislabeling that the existing tests (which only check count/sortedness, not per-pair distance correctness) wouldn't catch. Left as-is.
- Doubled `tour_length` computation and doubled identity-`Vec` allocation for the placeholder progress message in `greedy_edge::solve()` — both byte-for-byte match `christofides.rs`'s existing (unmodified) pattern; fixing only the new file would be inconsistent, and fixing both is out of this diff's scope.
- Test-helper duplication (`load_tsp`/`make_problem`/`is_valid_tour`) across `tests/greedy_edge_test.rs` and 7 other integration test files — pre-existing convention (no shared helper module currently holds these), out of scope to refactor here.

All fixes verified: `cargo build --workspace`, `cargo test --workspace` (336+ lib tests + all integration suites, 0 failures), `cargo clippy --workspace --all-targets -- -D warnings` clean on every touched file (pre-existing, unrelated clippy failures remain in untouched `distance_matrix.rs`/`kdtree.rs`, confirmed present on `master` too), `cargo fmt --check` clean.

Files touched by this review: `src/tsp/graph.rs`, `src/tsp/pipeline.rs`, `teeline-api/src/services/tsp_service.rs`, `teeline-wasm/tests/wasm_component.rs`. Reviewed, re-verified (`cargo test --workspace` 336+ tests green, both CI clippy invocations clean, `cargo fmt --check` clean), committed as `4182014` and pushed to `feat/greedy-edge-construction` (PR #393).

### 2026-08-07
- Implemented `src/tsp/graph.rs` (UnionFind, sorted_edges, hamiltonian_cycle_to_path) and `src/tsp/greedy_edge.rs` (select_edges + solve), TDD-first. All unit tests including a randomized n=4..12 property test pass.
- Registered `Solvers::GreedyEdge` across `src/tsp/mod.rs` (enum, variants, FromStr, all_meta, SOLVER_LIST bumped 19→20, dispatch match), `teeline-wasm/src/lib.rs` `kind_for()`/`recommendation_for()`, and `pipeline.rs`'s `stage_warnings` (mirrors nn's seed-discard warning).
- Along the way, a new registration-consistency test (`all_meta_names_and_aliases_round_trip_through_fromstr`) caught a pre-existing bug unrelated to this task: `all_meta()` listed `tabu` as tabu_search's alias but `FromStr`/`variants()` never accepted it. Fixed as a one-line addition since the new test now guards it.
- Added `tests/greedy_edge_test.rs` integration tests. Measured actual berlin52 quality (9954.06, ~32% above the 7542 optimum — worse than the general 14-25% literature range due to berlin52's isolated points forcing expensive closing edges) before setting the empirical-quality ceiling to 11000, per Opus review feedback not to hardcode an unmeasured number.
- Added `src/config.rs` pipeline TOML tests; confirmed (per Opus review) that zero `config.rs` code changes were needed for pipeline wiring.
- Verified: `cargo test --workspace` all green (336+ lib tests, all integration/e2e suites), `cargo fmt --check` clean, both CI clippy invocations (`-p teeline -p teeline-cli` and `-p teeline-api`) clean, `teeline-wasm cargo check` clean. `teeline-qt cargo check` skipped — no Qt toolchain (`qmake`) in this environment, matches plan's "best-effort" note.
- Manual smoke tests: `solve gec`/`solve greedy_edge --verbose` on berlin52.tsp both work; `solvers --output-format json` lists the new entry; `pipeline --steps gec,two_opt` improves cost from 9954.06 → 8415.55, confirming the pipeline-seed value proposition.
- Filed follow-up issue #392 for teeline-web docs page, explainer, and landing-page pills (deferred by design decision).
- CLAUDE.md solver table updated with `greedy_edge.rs` row.

## Completion Evidence

- Files added: `src/tsp/graph.rs`, `src/tsp/greedy_edge.rs`, `tests/greedy_edge_test.rs`
- Files modified: `src/tsp/mod.rs`, `src/tsp/pipeline.rs`, `src/config.rs`, `teeline-wasm/src/lib.rs`, `CLAUDE.md`, `docs/benchmarks.md`
- `cargo test --workspace`: all green, no failures
- `cargo clippy -p teeline -p teeline-cli -- -D warnings` and `cargo clippy -p teeline-api -- -D warnings`: clean
- `tests/bats/bin/bats tests/e2e/*.bats`: 49/49 passing
- Follow-up issues: https://github.com/timgluz/teeline/issues/392 (teeline-web surface), https://github.com/timgluz/teeline/issues/394 (savings-algorithm solver), https://github.com/timgluz/teeline/issues/395 (ACO solver — user's stated next pick)
- **PR #393 merged** to master at commit `65d551c` (2026-08-07). Local `master` fast-forwarded, feature branch `feat/greedy-edge-construction` deleted (local + remote, already auto-deleted on merge).
- `docs/benchmarks.md` updated post-merge with real measured berlin52 numbers: gec standalone +31.9% gap, gec→2opt +11.5% gap (SA-level quality, >30x cheaper).

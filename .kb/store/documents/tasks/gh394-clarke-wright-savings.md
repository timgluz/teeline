---
id: 019fe0ad-f5e7-7612-a803-b906e1dc94ae
slug: tasks/gh394-clarke-wright-savings
title: "GH #394: Clarke-Wright savings algorithm (cw) TSP solver"
type: task
status: completed
priority: medium
tags: [solver, algorithm, constructive, gh394]
---

## Overview

Add the Clarke-Wright savings algorithm as a new constructive TSP solver (alias `cw`), reusing the `UnionFind` / `select_edges` / `hamiltonian_cycle_to_path` scaffolding introduced for `greedy_edge` (PR #393). Resolves [GitHub issue #394](https://github.com/timgluz/teeline/issues/394), which `graph.rs`'s own header doc comment explicitly called out as a reuse candidate.

Fully planned in this session via an interactive Q&A with the user (3 design decisions), then implemented TDD-first on a feature branch.

## Algorithm

1. **Hub** = city nearest the coordinate centroid (mean of all `cities[].coords`), found via an O(n) linear scan using squared distance. Order-independent, unlike a fixed city-0 hub. Chosen because `TspProblem` carries no pre-built KD-tree and a single nearest query is cheaper as a scan than building one (the KD-tree only pays off across many queries, as in `nearest_neighbor`/`fourier`).
2. **Savings** for every pair `(i,j)`: `s(i,j) = d(hub,i) + d(hub,j) - d(i,j)`, sorted **descending** via `f32::total_cmp` (same NaN-safety rationale as `graph::sorted_edges`).
3. Hub-involving pairs have savings `0` (since `d(hub,hub)=0`), so they sink to the bottom of the descending sort and fill the hub's two edges only when needed — guaranteeing the run terminates with exactly `n` edges forming one Hamiltonian cycle, reusing `graph::select_edges` unchanged.
4. `graph::hamiltonian_cycle_to_path` → map positions to city IDs.

Mechanically it differs from `greedy_edge` only in its sort key (savings, descending, vs raw distance, ascending) and its hub reference.

## Design decisions (locked with user this session)

1. **Hub-node selection**: centroid-nearest via O(n) linear scan (not fixed city-0, not KD-tree). User asked about reusing the existing KD-tree; clarified that `TspProblem` carries no pre-built tree and a single query is cheaper as a scan.
2. **Identity**: `Solvers::ClarkeWright`, name `"clarke_wright"`, alias `"cw"`.
3. **`select_edges` reuse**: extracted greedy_edge's private `select_edges` into `graph.rs` as shared `pub(crate) fn select_edges` (the `graph.rs` header already anticipated this). greedy_edge now calls it; both solvers share the accept/reject primitive.

## Goals

- Extract `select_edges` into `graph.rs` as a shared `pub(crate)` primitive; refactor `greedy_edge` to use it (no behaviour change).
- Implement `src/tsp/savings.rs`: the solver + `hub_position` (centroid-nearest) + `sorted_edges_by_savings`.
- Register `ClarkeWright` across all touch points.
- Cover it with unit + integration tests following `greedy_edge`'s conventions.

## Acceptance Criteria

- [x] `src/tsp/graph.rs`: extracted `pub(crate) fn select_edges(n, &[(f32,u32,u32)])`; greedy_edge's `select_edges_*` unit tests (incl. the randomized n=4..12 property test) moved here; closing assert message generalized from "greedy edge selection" to solver-neutral wording.
- [x] `src/tsp/greedy_edge.rs`: calls `graph::select_edges`; local duplicate removed (−162 lines); solve end-to-end tests retained.
- [x] `src/tsp/savings.rs` (new): `solve()` mirroring greedy_edge's shape; `hub_position` (centroid + linear nearest); `sorted_edges_by_savings` (descending via `total_cmp`); 9 inline unit tests.
- [x] Core registration in `src/tsp/mod.rs`: module decl, `ClarkeWright` enum variant, `variants()` (`clarke_wright`/`cw`), `FromStr`, `all_meta()`, `SOLVER_LIST` (21→22 + SolverInfo), `solve_with_context` dispatch. Not added to `auto_expand_with_nn`/`auto_expand_with_shuffle` (parameter-free constructive, like greedy_edge/christofides).
- [x] `src/tsp/pipeline.rs`: `stage_warnings` entry mirroring greedy_edge ("discards warm-start seed").
- [x] `teeline-wasm/src/lib.rs`: `kind_for` (constructive arm) + `recommendation_for` (`cw` line). No `.d.ts`/WIT change (dynamic enumeration via `list_algorithms`).
- [x] `tests/clarke_wright_test.rs` (new): 4 integration tests (berlin52 validity, cost-matches-reported, empirical quality ceiling measured post-implementation, pipeline-as-seed-for-two_opt).
- [x] `cargo test` all green (375 unit + 4 clarke_wright + 4 greedy_edge, 0 failures), `cargo fmt` clean, no new clippy warnings in changed files, `cargo component build` (WASM) succeeds.
- [x] Manual smoke tests: `solve cw` / `solve clarke_wright` both → 8378.97 on berlin52; `solvers` lists `clarke_wright`/`cw`/heuristic; pipeline warning fires correctly for `pipeline --steps nn,clarke_wright,2opt`.

## Context

- Branch: `feat/clarke-wright-savings-solver`.
- **Precedent gap noted**: `greedy_edge` (`gec`) shipped to core/CLI/WASM but was never added to `docs/algorithms/` (no `greedy-edge.md`) nor `teeline-web`'s `nav-data.ts` SOLVER_META. Per user decision, the docs/web follow-ups are **`cw` only** — the `gec` gap remains a separate pre-existing issue.
- Empirical quality: measured berlin52 = 8379 (~11% over optimal), notably better than `greedy_edge`'s ~9954 (~32% over optimal), since the savings ordering avoids the expensive closing edges that pure distance-greedy forces on berlin52's isolated points.

## Progress Log

### 2026-08-08
- Read issue #394 and traced the full `greedy_edge` scaffold (graph.rs, greedy_edge.rs, mod.rs registration, pipeline, WASM, integration test) to map all touch points.
- Ran an interactive planning Q&A with the user: 3 design decisions (hub selection, alias/identity, select_edges extraction) + 1 follow-up clarification (KD-tree vs linear scan for hub lookup).
- Implemented on `feat/clarke-wright-savings-solver`, TDD-first:
  - Extracted `select_edges` to `graph.rs` (shared); refactored `greedy_edge.rs`.
  - Created `savings.rs`; registered `ClarkeWright` across mod.rs (6 touch points), pipeline.rs, WASM lib.rs.
  - Created `tests/clarke_wright_test.rs`; measured berlin52 = 8379 before setting the empirical ceiling to 9200 (~10% above measured, matching greedy_edge_test's convention).
- Fixed two compile issues during implementation: graph.rs's moved property test needed `use rand::RngExt;`; removed an unused binding in a savings test.
- Full verification green: build, 375 unit + 8 integration tests, fmt, clippy (changed files clean), WASM component build, CLI sanity (both aliases, solvers catalogue, pipeline warning).
- Committed (`8a9b574`, pre-commit fmt+clippy hooks passed), pushed, opened PR #402.
- Filed 5 follow-up issues (#403–#407) for the deferred docs/web/api/explainer scope.

## Completion Evidence

- Branch: `feat/clarke-wright-savings-solver`, commit `8a9b574`.
- PR: https://github.com/timgluz/teeline/pull/402 (open, pending review/merge; `Closes #394`).
- `cargo test`: all suites green, 0 failures (375 lib tests including 9 in `savings` + 5 moved `select_edges` tests in `graph`, 4 dedicated integration tests).
- `cargo fmt`: clean. `cargo clippy --lib --tests -p teeline`: no warnings in changed files.
- `cargo component build --manifest-path teeline-wasm/Cargo.toml --target wasm32-wasip2`: succeeds.
- Manual CLI: `solve cw`/`solve clarke_wright` → 8378.97 on berlin52; `solvers` lists the new entry; pipeline warning fires for `clarke_wright` at stage > 0.
- Follow-up issues: #403 (docs page), #404 (teeline-web nav-data), #405 (WASM rebuild+redeploy), #406 (teeline-api verify), #407 (interactive explainer, deferred).

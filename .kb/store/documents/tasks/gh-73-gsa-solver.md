---
id: 019ebfd9-6c05-7ac0-9d3b-ee9ca71279f8
slug: tasks/gh-73-gsa-solver
title: "feat: implement Gravitational Search Algorithm (GSA) solver (GH #73)"
type: task
status: completed
priority: high
tags: [algorithm, metaheuristic, wasm]
---

## Overview

Add a Gravitational Search Algorithm (GSA) solver as an educational example of the Newtonian physics / pairwise-force metaheuristic family. Every agent is a candidate tour; mass = normalised fitness; heavier agents attract lighter ones through a swap-based velocity update.

GH issue: #73. Branch: `feat/gh-73-gsa-solver`.

## Design Decisions

- **Mass normalization**: spread-based Rashedi 2009 — `m_i = (worst_cost - cost_i) / Σ(worst_cost - cost_j)`; fallback to uniform `1/N` when sum < ε (all agents equal cost)
- **Position acceptance**: PSO-style — always apply velocity, track gbest
- **Inertia**: fixed `INERTIA_W = 0.5` (linear decay only if benchmarks justify it)
- **G decay**: `G(t) = G0 · exp(−α · t / T)`, G0=1.0, α=5 (tune against berlin52)
- **Kbest**: fixed `⌈N/2⌉` agents attract (v2: linear K decay — follow-up issue)
- **v_max**: `⌈0.35 · n_cities⌉` (same as PSO)
- **N agents**: `opts.n_nearest.max(DEFAULT_N_AGENTS=25)`
- **WASM**: exposed in this PR

## Goals

- New metaheuristic solver for TSP with physics-inspired exploration
- Fully integrated: CLI + WASM catalog + docs + benchmarks

## Acceptance Criteria

- [ ] `src/tsp/gravitational_search.rs` — solve() with 3 unit tests (valid tour, init tour, uniform mass NaN guard)
- [ ] `src/tsp/mod.rs` — module, enum variant, aliases, auto_expand_with_shuffle, find_solver, SOLVER_LIST (16→17), all_meta(), solve_problem dispatch
- [ ] `teeline-wasm/src/lib.rs` — recommendation_for, params_for_solver (shared_heuristic_params)
- [ ] `tests/solvers_integration.rs` — 2 integration tests + gravitational_search import
- [ ] `teeline-wasm/tests/wasm_component.rs` — bump 16→17, add "gsa" to expected ID list
- [ ] `docs/algorithms/gravitational-search.md` — algorithm doc
- [ ] G0/α tuned against berlin52 release build
- [ ] `docs/benchmarks.md` — 1-2 GSA rows added
- [ ] `CLAUDE.md` — solver table row
- [ ] `README.md` — metaheuristic section entry
- [ ] `cargo test` passes clean

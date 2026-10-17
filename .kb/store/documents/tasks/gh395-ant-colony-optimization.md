---
id: 019fe03e-555f-7d12-bdd2-4821e6d268a0
slug: tasks/gh395-ant-colony-optimization
title: "Ant Colony Optimization (ACO) TSP solver"
type: task
status: completed
priority: medium
tags: [solver, metaheuristic, aco, gh395]
---

## Overview

Add Ant Colony Optimization as a new metaheuristic TSP solver (alias `aco`), following the `HeuristicOptions`-embedding pattern used by `CSOptions`/`FPAOptions`. Resolves [GitHub issue #395](https://github.com/timgluz/teeline/issues/395), flagged as the natural next algorithm after greedy edge construction (`gec`, PR #393) and the savings-algorithm placeholder (#394).

Fully planned and reviewed (second pass by Opus against the live codebase) in this session. Not yet implemented — this task exists so the plan survives to a future implementation session. The complete, reviewed plan is also saved at `/home/timgluz/.claude/plans/please-plan-issue-395-fizzy-flamingo.md` on the planning machine, but that path is local/ephemeral — this document is the durable copy.

## Goals

- Implement a classic Ant System (AS) ACO solver in `src/tsp/ant_colony.rs`.
- Wire it into the solver registry, TOML pipeline config, and CLI — **core-first scope only** (explicitly excludes `teeline-api`, `teeline-wasm`, `teeline-qt`, `teeline-web` — those are follow-up issues, since ACO's tunables don't fit WASM's existing `mutation-probability` slot the way CS/FPA's do, and touching WASM means a WIT + jco regen, a distinct chunk of work).
- Cover it with unit + integration tests following this codebase's established stochastic-solver test conventions (validity-only, no multi-run averaging, no hardcoded optimality ceiling except the confirmed `#[ignore]`-gated pattern Fourier uses).
- Document it (`CLAUDE.md` table row, new `docs/algorithms/ant-colony.md`).

## Acceptance Criteria

- [x] `AcoOptions` struct (heuristic + alpha/beta/evaporation_rate/num_ants) with `Default` (overriding `heuristic.epochs` to 150, NOT inheriting the base 10,000), `validate()`, `from_toml()`, `from_cli()`.
- [x] `src/tsp/ant_colony.rs`: pure pheromone helpers (`evaporate_and_floor`, `deposit_edge`, `deposit_tour`, `roulette_select`, plus a `select_next` fallback-chain helper added during implementation) each with RNG-free unit tests, plus the full `solve()` loop.
- [x] Core registration in `src/tsp/mod.rs`: module decl, `Solvers::AntColony` variant, `variants()`, `auto_expand_with_shuffle()`, `all_meta()`, `SOLVER_LIST` (20→21), `FromStr`, `solve_with_context()` dispatch, `AppOptions.aco` field.
- [x] `test_app_options_has_no_flat_epoch_fields` updated — confirmed the exhaustive-literal compile break during implementation exactly as predicted (E0063), fixed. Also had to fix an identical exhaustive-literal break in `teeline-api/src/services/tsp_service.rs` (out-of-scope crate, but the workspace must still compile — added `aco: None` there with a comment noting API wiring is a follow-up).
- [x] `test_auto_expand_shuffle_for_stochastic_solvers` gains an `AntColony` assertion.
- [x] `src/config.rs`: `TomlTableProvider` `"aco"` arm, sub-table ownership check entry, unknown-key error string AND doc-comment updated, new tests (5 added).
- [x] `teeline-cli/src/main.rs`: `AcoOptions` import inside `CliArgsProvider::provide`, `--alpha`/`--beta`/`--evaporation-rate`/`--num-ants` flags (kebab-case long, "ACO: " help prefix), `CliArgsProvider::provide` arm, auto-expand help text update, `test_solver_options_aco_num_ants_parsed` flag-parsing test.
- [x] `tests/ant_colony_test.rs` (dedicated file, mirrors `fourier_test.rs`) with an explicit `fast_opts()` helper and 3 tests (berlin52 validity, warm-start, non-contiguous city ids).
- [x] `tests/fixtures/pipeline_aco.toml` + round-trip test in `tests/pipeline_config_test.rs`.
- [x] `CLAUDE.md` solver table row; new `docs/algorithms/ant-colony.md`; `docs/benchmarks.md` placeholder row.
- [x] `cargo build -p teeline-cli`, `cargo test --workspace` (364 lib tests + all integration suites, 0 failures), `cargo fmt --check`, and both CI clippy invocations (`cargo clippy -p teeline -p teeline-cli -- -D warnings`, `cargo clippy -p teeline-api -- -D warnings` — verified these are the *actual* CI commands via `.github/workflows/*.yml`, not `--workspace --all-targets` which has 23 pre-existing unrelated failures on master) all green.
- [x] Manual smoke test: `teeline solve aco` on berlin52 auto-expands to `pipeline(shuffle, aco)` and completes in ~1.1s (debug build) with a +3.3% gap (7792.57 vs optimal 7544.37); with `--beta 3.0` got +1.2% (7634.50). `pipeline --steps nn,aco --epochs 0` returned cost 8980.918 — an exact match to NN's known berlin52 result from benchmarks.md, confirming the seed tour passes through unmodified.

## Context

### Algorithm decisions (confirmed with user this session)
- **Ant System (AS)**, not ACS or Elitist AS: fully probabilistic transition rule, single global pheromone update (evaporate + **all-ants** deposit) per epoch.
- **No candidate-list restriction** — transition probability considered over all unvisited cities each step.
- **τ0 bootstraps from `init_tour`** when provided and its cost `L > 0.0`: `τ0 = num_ants / L`. Falls back to flat `τ0 = 1.0` + `tracing::warn!` only when no usable `init_tour` exists. Since ACO joins `auto_expand_with_shuffle()`, a bare CLI `teeline solve aco` always auto-expands to `pipeline(shuffle, aco)`, so `init_tour` is normally populated — the flat-fallback path is mainly reached via direct library use, not typical CLI usage.
- **Warm-start**: seed `best`/`best_cost` from `init_tour`, plus one extra pheromone deposit pass along its edges before epoch 0 (so it actually biases epoch-1 construction, since AS ants — unlike CS/FPA's persistent population array — are stateless between epochs).
- **Ants are stateless**: no persistent per-ant memory across epochs (deliberate deviation from CS/FPA's mutated-population-array pattern), only the shared pheromone matrix persists.

### Pheromone matrix design (new territory — no existing solver maintains O(n²) mutable state)
- Dense `Vec<f32>` sized `n*n`, row-major, indexed by **position** (`city_id2pos()` output, never raw city id — TSPLIB fixtures like berlin52 are 1-indexed).
- **Critical bug caught in review and must be implemented, not optional**: evaporation needs a floor (`tau_min = tau0 * 1e-4`) applied after every `*= (1 - evaporation_rate)` multiply. Without it, an un-deposited edge decays to exact `f32` `0.0` in ~141 epochs at typical parameters (τ0≈3.3e-3, evaporation_rate=0.5) — well inside this task's own 150-200 epoch smoke test — making that edge permanently unreachable regardless of heuristic distance. This is a real bug, not a theoretical edge case.
- Deposit: `Δτ = 1/tour_cost`, symmetric (write both `[u*n+v]` and `[v*n+u]`), for every ant every epoch (not iteration-best-only — that would be Elitist AS, a different variant).
- Transition weight: precompute `eta_beta` (heuristic term, static per run) before the epoch loop; precompute `tau_alpha` (pheromone term) once **per epoch**, not once per ant-step (avoidable ~m·n²/2 vs n² `powf` calls — a real performance difference, not micro-optimization, given ACO's already-heavier O(epochs·ants·n²) complexity vs CS/FPA's O(epochs·pop·n)).
- `distance_by_pos` returns `Result<f32, &'static str>` — must `.unwrap_or(0.0)`, an earlier draft of this design missed that and wouldn't have compiled.
- `beta` needs an upper bound in `validate()` (e.g. `<= 10.0`) — `(1e6)^beta` overflows f32 around beta≈7 given the `.max(1e-6)` distance floor, reachable with coincident cities + a high user-supplied beta.
- `roulette_select` must explicitly return the last weight's index if the accumulation loop exhausts without crossing `r * sum` (f32 rounding over 50+ terms makes this a real, not theoretical, path) and the call site must guard `sum <= 0.0 || !sum.is_finite()` with a fallback to eta-only proportional selection (not uniform-random, which would discard distance information entirely).
- `ProgressMessage::PathUpdate` must send tours converted to **city ids** via `pos2city_id()`, not raw positions — `Route` is documented as city-id space, and CS/FPA both send city ids; sending positions would draw a scrambled route in the live progress window on any 1-indexed TSPLIB file.

### Default values
| Field | Default | Why |
|---|---|---|
| alpha | 1.0 | standard pheromone-influence baseline |
| beta | 2.0 | modern common practice (ACS-era), kept low so transitions stay meaningfully probabilistic rather than collapsing toward greedy nearest-city choice |
| evaporation_rate | 0.5 | common AS/ACS default; validated as open `(0.0, 1.0)`, same convention as `SAOptions.cooling_rate` |
| num_ants | 25 | matches CS/FPA's population-size convention |
| `AcoOptions::default().heuristic.epochs` | ~100-200, **override, do not inherit base 10,000** | ACO's O(epochs·ants·n²) is 25-140× more expensive per epoch than CS/FPA/GA's O(epochs·pop·n); inheriting the base default is a genuine hang risk (minutes on berlin52, far worse on larger instances) for any bare `AcoOptions::default()` construction, e.g. a CLI run without `--epochs` or a test that forgets to override it. Follow `LKOptions::default()`'s precedent (mod.rs:1036-1048) of overriding rather than inheriting. |

### Verified against live code (HEAD 65d551c) this session
Two Explore agents and one Plan agent mapped every registration touch point, options-struct pattern, and test convention against the real files (not assumptions) — see line numbers embedded throughout the Acceptance Criteria above. A second independent review pass (different model, Opus) re-verified all of it against the live code and caught 5 real bugs in the first draft (compile break from the exhaustive `AppOptions` literal test, the hang-risk default epochs, the pheromone-underflow-to-zero bug, the `Result`-typed `distance_by_pos` misuse, and the position/city-id mismatch in progress updates) plus several lower-severity gaps (missing β upper bound, missing `n<=2` guard, two hardcoded config.rs strings needing updates, a missing CLI flag-parsing test). All of these are folded into the Acceptance Criteria and design notes above — this is not a first draft, it's a reviewed plan.

## Progress Log

### 2026-08-08
- Explored codebase (3 parallel Explore agents): metaheuristic solver structure (CS/FPA/PSO), full registration-touch-point checklist (cross-referenced against gec's PR #393 diff), config/CLI/test wiring patterns.
- Confirmed with user: core-first scope (defer teeline-api/wasm/qt/web to follow-ups), Ant System variant (not ACS), no candidate-list restriction, τ0 bootstraps from init_tour with flat-constant+log fallback, warm-start seeds best + one deposit pass.
- Ran a Plan agent to design the full implementation; verified several of its line-number/API claims by hand against live source.
- Ran an independent Opus review pass against live code; it caught 5 real bugs (listed above) and ~8 lower-severity gaps. Folded all fixes into the plan.
- Plan approved by user.
- Implemented the full plan on branch `feat/gh395-ant-colony-optimization`: `AcoOptions` (mod.rs), `src/tsp/ant_colony.rs` (solver + 4 pheromone helpers + `select_next` fallback chain + 14 inline tests), core registration (mod.rs), `src/config.rs` wiring (5 new tests), `teeline-cli/src/main.rs` wiring (1 new test), `tests/ant_colony_test.rs` (3 tests), `tests/fixtures/pipeline_aco.toml` + 1 round-trip test, `CLAUDE.md`/`docs/algorithms/ant-colony.md`/`docs/benchmarks.md` docs.
- Caught and fixed a second exhaustive-struct-literal compile break the plan hadn't anticipated: `teeline-api/src/services/tsp_service.rs` also constructs `AppOptions` exhaustively. Added `aco: None` there (API wiring is out of scope for this task, but the workspace must compile).
- Verified the actual CI clippy commands via `.github/workflows/*.yml` rather than assuming `--workspace --all-targets` (which has 23 pre-existing failures on master, unrelated to this change) — used the real commands instead.
- All acceptance criteria met and checked off above. Full verification pass green (build, 364+ tests, fmt, clippy, manual CLI smoke tests). No implementation deviations from the reviewed plan.

## Completion Evidence

- Branch: `feat/gh395-ant-colony-optimization`, commit `1c2c056`.
- PR: https://github.com/timgluz/teeline/pull/396 (open, pending review/merge).
- Follow-up issues filed for deferred scope: #397 (teeline-api), #398 (teeline-wasm — WIT fields + bindings regen), #399 (teeline-qt), #400 (teeline-web — nav-data + explainer decision), #401 (README.md/docs/wasm.md solver-table debt, covers both `gec` and `aco` gaps).
- `cargo test --workspace`: all suites green, 0 failures (364 lib tests including 14 in `ant_colony`, 9 `AcoOptions`-specific in `mod.rs` tests, 3 dedicated integration tests, 5 config.rs tests, 1 CLI flag test, 1 pipeline round-trip test).
- `cargo fmt --check`: clean.
- `cargo clippy -p teeline -p teeline-cli -- -D warnings` and `cargo clippy -p teeline-api -- -D warnings`: both clean (these are the actual CI invocations, confirmed via workflow files).
- Manual CLI runs on berlin52: bare `teeline solve aco --epochs 150` → 7792.57 (+3.3% gap) in ~1.1s; with `--beta 3.0` → 7634.50 (+1.2% gap); `pipeline --steps nn,aco --epochs 0` → 8980.918, exactly matching NN's documented berlin52 result, confirming the passthrough contract.

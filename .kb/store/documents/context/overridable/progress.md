---
id: 019e2683-38bd-7a40-b44d-1c29e6e3eaf6
slug: context/overridable/progress
title: "Progress"
type: context
status: active
priority: medium
tags: [161-]
---

## Current Status

**Phase: Interactive-explainers series complete — all 21 explainers merged (2026-08-16).**

Local `master` synced to `314c82e` (PR #476 merge). Every algorithm on tspsolver.com now has
an interactive explainer (10 pre-Astro + 11 added since: greedy-edge, savings, aco,
stochastic-hill, or-opt, 3-opt, christofides, branch-bound, bhk, 2-opt, nn), plus an
`/algorithms/` index page with complexity column. CI green; astro check + tsc + vitest +
Playwright (chromium via `.pw-browsers`) all pass locally.

## 2026-10-12

- **`tasks/bench-sa-honours-epoch-budget` completed** (`72007e0`): SA combined its stopping rules
  with `||`, so `epochs` was a floor rather than a cap and raising it changed nothing. Now `&&`,
  with the SA default raised to 150,000 so the temperature schedule still governs a normal run.
  Unblocked the plateau and CLI-help prerequisites.
- Groundwork for solver benchmarking: spec `decisions/benchmarking-design`, 10 tasks, and 9 new
  ADR/task documents newly tracked in git (`.kb/store/` was previously gitignored, so all specs
  and ADRs existed only on one machine).

## Completed Work

- Core solver framework (Solvers enum, SolverOptions, Solution, KDTree, DistanceMatrix)
- 11 algorithms: BHK, Branch-Bound, NN, 2-opt, 3-opt, SA, Tabu, GA, Stochastic Hill, PSO, CS, FPA
- eframe progress visualisation window (#40)
- TSPLIB parser + explicit matrix support (#38)
- GitHub Actions CI (build + test + clippy)
- GitKB knowledge base bootstrapped (2026-05-14)
- Clippy warnings fixed, CI green (#50) — 2026-05-15
- Pipeline solver (#66) — 2026-05-17
- GA seeding improvements (#79, PR #82) — 2026-05-20
- **TOML config for pipeline per-stage overrides (#81, PR #83) — 2026-05-22 (merged)**
- **parse-and-solve WASM export (#147, PR #150) — 2026-06-06 (merged)**
  - `parse_and_solve` WIT export: auto-detects JSON vs TSPLIB, runs solver
  - `teeline-wasm` tests moved to own package (PR #152, issue #151)
  - Dedicated `build-wasm` CI job parallel to `build`
- **parse WIT export + ParsedProblem (#131, PR #153) — 2026-06-06 (merged)**
  - `parsed-problem` record: name, comment, distance-type, cities
  - Web client can call `parse` to get city list for rendering before solving
  - `kd_to_city` helper extracted; TDD: 4 wasmtime + 2 Vitest tests added
  - teeline-web: `ParseRequest`/`ParseResult` types in worker.ts
  - Example datasets bundled: berlin52, burma14, ulysses22
- **Step 02 solver config UI + Step 03 precondition checklist (#133, PR #155) — 2026-06-07 (merged)**
  - `solver-config.ts`: pure data module, 13 solvers with param definitions; 19 Vitest tests
  - `solver-form.ts`: DOM wiring — pills, dropdown, config panel, checklist, Run button gate
  - Playwright MCP used for browser verification (4 scenarios)
- **Step 04 — run, SVG canvas, results & run history (#134, PR #157) — 2026-06-07 (merged)**
  - `canvas.ts`: renderTour, scaleCoords, buildPolylinePoints (SVG-based)
  - `results.ts`: formatGap, formatRuntime, computeRouteLength, RunRecord, showResult
  - Run history panel with click-to-replay; "↻ try another solver" flow
  - 57 Vitest tests; Playwright MCP: 5 browser scenarios
- **CI path-filter fix (#156, PR #158) — 2026-06-07 (merged)**
- **Sentry error tracking (PR #159) — 2026-06-07 (merged)**
  - `@sentry/browser` + `@sentry/vite-plugin`; tunnel endpoint `/tunnel`
- **Step G — downloads + responsive polish (#135, PR #160) — 2026-06-07 (merged)**
  - `download.ts`: buildTourText/Csv/Json, serializeSvg, triggerDownload; 16 Vitest tests
  - 4 download buttons (.tour/.csv/.json/.svg); shared `crypto.randomUUID()` per solve
  - Responsive: phone <576px hides stepper labels; desktop ≥1024px 2-col Step 04 grid
  - 73 Vitest tests total
- **v1.0.0 release + CF Pages deploy pipeline — 2026-06-07 (merged)**
  - PRs #161-#172: CF Pages deploy CI, Sentry tunnel, MIME fixes, WASM copy, jco `--name` fix
  - deploy-web.yml: downloads WASM from latest GH release → jco transpile → CF Pages
  - teeline-solver.wasm (wasip1) published as GH release asset
- **Wassette integration (#176, PR #178) — 2026-06-07 (merged)**
  - WIT extensions: `algorithm-info`, `compare-result`, `duration-ms` types
  - New exports: `list_algorithms`, `compare` (run N solvers, return side-by-side)
  - `solve_with_cities` returns `duration_ms` timing
  - 30 wasmtime tests pass (including list_algorithms + compare)
  - release.yml: adds publish-wassette job (wasip2 build + ORAS push to ghcr.io/timgluz)
  - teeline-solver-wassette.wasm available as Wassette MCP component
  - Wassette MCP verified: 5 tools loaded, berlin52 solved (SA: 7,889 vs optimal 7,542)
- **Interactive explainer series (GH #429–#434, PRs #468–#476) — 2026-08-15/16 (merged)**
  - **Stochastic Hill Climbing explainer (#433, PR #468)** — 8 cities, seeded RNG, 3 curated scenarios (quick_convergence seed 371 / rugged 917 / needle 1548); faithful port of `src/tsp/stochastic_hill.rs`
  - **Or-opt explainer (#432, PR #469)** — Or-1/2/3 relocation moves, `scanBestMove` with `-1e-3` threshold, `two_opt_stuck` scenario (starts from the actual 2-opt two-optimal tour)
  - **3-opt explainer (#434, PR #470)** — all 7 move cases, hexagon pattern diagram per case; also fixed a stochastic-hill step race
  - **Shared `explainer-cities.ts` refactor (PR #471)** — `CITIES_10/12/8`, `makeDist`/`makeTourLength`/`makeDm`, prebound dist/tourLength; extracted from an `ocr` finding
  - **Christofides explainer (#431, PR #472)** — Prim → odd → greedy matching → multigraph → Hierholzer → shortcut; ratio readout vs brute-forced optimum; scenarios 1.00× (circle) to 1.387× (worst_case)
  - **Branch & Bound explainer (#430, PR #474)** — search-tree visualization, MST lower bound, closed-cycle fix ("TSP is about closed circles"), fixed 740px tree container, 14 tests incl. random-instance brute-force sweep
  - **Bellman–Held–Karp explainer (#429, PR #475)** — DP-over-subsets table animation, 6 cities; squash-merge absorbed the #473 menubar content
  - **Topbar dropdown → algorithms index + complexity column (PR #476)** — `/algorithms/` index with groups + Complexity column + aria-labels; `SOLVER_META.complexity` mirrors docs (21/21), enforced by a consistency test; all `docs/algorithms/*.md` gained a Complexity meta row
  - Every explainer: pure `*-algo.ts` sim + Preact `*.tsx` + vitest + Playwright e2e, verified with `ocr` review before merge

## In Progress

- Session wrapped up 2026-08-16 (explainer series shipped). Next session: pick from Remaining Work — web candidates are #438 (papers citing dataset) and #323 (teeline-excel); auth Phase 0 is parked.

## Blocked

- Nothing currently blocked

## Remaining Work (by issue)

Rust solver side:
- #85 Expand BATS e2e tests + wire into CI/CD
- #86 Replace validate_options with garde derive macro on SolverOptions
- #87 Solver-specific options structs (SAOptions, GAOptions) resolved by PipelineStage
- #46 Z3 exact reference solver

Web side (open, untouched this session):
- **#438** — feat(web): add "Papers citing this dataset" section to problem pages
- **#323** — teeline-excel Office Add-in

Parked:
- WebAuthn auth (see `tasks/webauthn-auth-replace-clerk`) — design accepted; Phase 0 (Pages Functions scaffold) not started

## TSPLIB Known Optima (for regression tests)

| Instance | Cities | Optimal |
|----------|--------|---------|
| bayg29   | 29     | 1610    |
| att48    | 48     | 10628   |
| berlin52 | 52     | 7542    |
| a280     | 280    | 2579    |

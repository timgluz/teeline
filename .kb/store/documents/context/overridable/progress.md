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

**Phase: the problem-page publication pipeline shipped (2026-10-11).** Every `/problems/{id}/` page
now carries a minimap and a 1200x630 social card served from R2 (`minimaps/v1/`), the docs are
frontmatter-only, and the publisher refuses to publish a page whose card the bucket cannot serve.

Local `master` synced to `dd0ee444` (the #588 merge). `teeline-web` on astro 7.3.5, node 24.21.
Verified on the merged heads: 165-page build, 588 vitest, `tsc`/`astro check` clean, 17 BATS for the
publisher, the non-auth Playwright suite green in CI as the new `e2e-web` job. The solver side is
unchanged except that `EUC_2D` now rounds as TSPLIB defines it (#575, followed by #587 in the page
generator).

Open tickets: **11 draft**, 1 active (`blog-passkeys-instead-of-clerk`, waiting on feedback for the
recipe post), 0 blocked. One open PR (#555, renovate) and two open issues (#438, #323).

## 2026-10-11

- **Minimaps and social cards moved to R2** (#584, #585): 111 maps + 111 cards published under
  `minimaps/v1/`, verified by reading all 223 objects back. `docs/problems/*.md` went from 19.6 MB of
  inline SVG to 58 KB of frontmatter, so a regeneration is a reviewable diff rather than 223k lines.
- **All nine `EDGE_WEIGHT_FORMAT` layouts parse** (#583), which unblocked `si175`/`si535`/`si1032`
  and removed the one substitution the tier set needed.
- **`EUC_2D` is TSPLIB-rounded everywhere it reports a cost**: #575 in the solver, #587 in the page
  generator, which also had two latent defects the audit exposed — routes were parsed one id per
  line (pr1002 scored a subset: 105695 against 259045) and nothing required a route to name every
  city (rd100's 11-of-100 ids scored 4344 against a 7910 optimum).
- **The non-auth Playwright suite runs in CI** (#586), as `e2e-web`, provisioning the released WASM
  component. It immediately mattered: five WebMCP specs had been asserting the home page for forms
  that live on `/solve/`.
- **The publisher gained a guard that cannot pass over a 404** (#588): a page linking
  `og/<id>.png` that the index has no `og` entry for stops both the publish and the `--verify` path.
- Every PR was reviewed with `ocr` after opening it, which found real defects in two of them (the
  permutation hole in #587, the card-404 hole in #585) and was wrong twice about #585's tests.
- Tickets closed: `problem-minimaps-all-instances` (superseded), `problem-minimaps-as-r2-assets`,
  `problem-social-cards`, `tsplib-all-edge-weight-formats`, `ci-runs-only-auth-e2e`,
  `fix-euc-2d-rounding`, `minimap-pipeline-guards`, `gh-286-api-key-auth`, `gh-185-fourier-solver`.
  Five benchmark tickets were closed earlier for the same reason: their PRs had merged while they sat
  in `active`/`draft`.

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
- **Problem minimaps + social cards on R2 (PRs #583, #584, #585, #588) — 2026-10-11 (merged)**
  - `scripts/lib/problem-minimap.mjs`: TSPLIB layout → classical MDS (double-centre + power
    iteration with deflation), density grid above 3000 cities, standalone SVG 340x200
  - `scripts/publish-minimaps.sh`: per-object upload, `index.json` hash manifest, `--verify` reads
    every object back over the public URL, and a card check that fails loudly before either path
  - `teeline-web/scripts/gen-og-images.mjs`: SVG → 1200x630 PNG with Playwright, hashes recorded in
    the same index
  - `BaseLayout`/`DocsLayout` take an optional card; problem pages pass it and advertise the raster
    in `Dataset` JSON-LD
  - 111 maps + 111 cards live under `minimaps/v1/`; 17 BATS, 10 Playwright scenarios, byte-identical
    regeneration
- **`EUC_2D` nint rounding (#575, #587) — 2026-10-11 (merged)** — the solver's metric and the page
  generator's derivation now agree with TSPLIB, verified against all 32 shipped tours
- **`EDGE_WEIGHT_FORMAT`: all nine layouts (#583) — 2026-10-11 (merged)** — table-driven parser tests
- **`e2e-web` in CI (#586) — 2026-10-11 (merged)** — non-auth suite on every web PR

## In Progress

- Nothing claimed. Session wrapped up 2026-10-11 with the problem-page pipeline shipped and the
  board reconciled (every ticket whose PR had merged was closed; see the log above).

## Blocked

- Nothing currently blocked

## Remaining Work (by issue)

Web side:
- **#438** — "Papers citing this dataset" section on problem pages (top candidate; same pipeline as
  the complexity column and the minimaps)
- **#555** — open PR (renovate: ignore preact major updates) — mergeable, checks green; its premise
  predates astro 7.3.5
- **#323** — teeline-excel Office Add-in (unrelated, untouched)

Open benchmark tickets (11 drafts): `bench-tier-a-campaign` (raw data answers most criteria; one
to-convergence config unaccounted for), `bench-docs-spec-update` (`docs/benchmarks.md` still not
retired), `bench-single-solver-skill`, `bench-regression-history`, `bench-lk-use-shared-plateau-option`,
`ci-cli-e2e-bats-workflow` (the suite runs inside `rust.yml`, not as its own workflow),
`migrate-jaq-for-toml-queries`, `solver-progress-listener`, `blog-benchmarks-duckdb`,
`gh-66-pipeline-solver`, `ship-missing-optimal-tours` (25 problems publish an optimum but ship no
`.opt.tour`)

Rust side, older issues still open: #85 (BATS in CI/CD — partly overtaken by `rust.yml` running
`tests/e2e/`), #86 (garde derive), #87 (solver-specific options structs), #46 (Z3 reference solver)

Parked: WebAuthn auth ([[tasks/webauthn-auth-replace-clerk]]) — Phase 0 not started; the API-key
line it relates to has moved on (#310, #313, #462)

## TSPLIB Known Optima (for regression tests)

| Instance | Cities | Optimal |
|----------|--------|---------|
| bayg29   | 29     | 1610    |
| att48    | 48     | 10628   |
| berlin52 | 52     | 7542    |
| a280     | 280    | 2579    |
| eil51    | 51     | 426     |
| eil76    | 76     | 538     |
| kroA100  | 100    | 21282   |
| tsp225   | 225    | 3916    |
| pr1002   | 1002   | 259045  |

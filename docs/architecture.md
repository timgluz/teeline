# Architecture

How the Teeline codebase is organised today. The governing principles live in
[`constitution.md`](constitution.md); dated decisions live in [`docs/adr/`](adr/).

## Project shape

Teeline is a Cargo workspace plus several independently built subprojects. The root
`teeline` crate is the pure solver library; every other crate consumes it.

| Path | Crate / artifact | Role |
| --- | --- | --- |
| `.` | `teeline` | Pure solver library. No GUI or transport dependencies. |
| `teeline-cli/` | `teeline-cli` | CLI binary `teeline` (solve, pipeline, convert, solvers). |
| `teeline-api/` | `teeline-api` | Axum REST API at api.tspsolver.com (Fly.io). |
| `teeline-wasm/` | `teeline-wasm` | WebAssembly Component Model build (`wasm32-wasip2`). Excluded from the root workspace. |
| `teeline-qt/` | `teeline-qt` | Qt 6/QML desktop GUI (separate workspace; requires Qt 6.7+). |
| `teeline-web/` | — | Astro/Preact site at tspsolver.com. |
| `teeline-excel/` | — | Office Add-in subproject. |

The root workspace members are `teeline`, `teeline-cli`, and `teeline-api`.

## Core library (`src/`)

- `src/lib.rs` exposes `pub mod config` and `pub mod tsp`, plus `DistanceType`.
- `src/tsp/mod.rs` holds the solver catalogue and dispatch:
  - `Solvers` enum — one variant per algorithm.
  - `find_solver(name)` — alias-aware parsing (`"nn"` → `NearestNeighbor`).
  - `solve_problem(solver, &TspProblem, &AppOptions)` — public entry point for external
    crates.
  - `solve_with_context(...)` — internal dispatcher adding a progress sender and an
    optional warm-start tour.
  - `Solvers::all_meta()` — CLI/UI catalogue (name, alias, kind).
- `TspProblem` — the instance: `cities: Vec<KDPoint>` plus a precomputed
  `DistanceMatrix`.
- `AppOptions` — pure configuration shell; one optional option struct per solver
  (`SAOptions`, `GAOptions`, `LKOptions`, …) plus the shared `HeuristicOptions`.
- `Solution` — a tour plus its cost: private `route` and `cities`, public `total`
  computed from the distance matrix at construction.
- `Route` — ordered city IDs with shuffling and 2-opt helpers used by solvers.
- `KDPoint` / `DistanceMatrix` / `NearestResult` — spatial types (see
  [patterns](patterns.md)).
- `ProgressMessage` (`src/tsp/progress.rs`) — the typed progress contract:
  `CityChange`, `PathUpdate`, `EpochUpdate`, `Done`, `Restart`, `OptimalTour`.

Each solver lives in its own `src/tsp/<solver>.rs` and exposes a `solve(...)` function.
The dispatcher adapts the per-solver option type; the pipeline calls solvers in sequence,
threading the best tour forward as the next stage's warm start.

## Data flow

```text
file / stdin ──▶ TSPLIB or JSON parser ──▶ TspProblem (cities + DistanceMatrix)
                                                │
                                                ▼
                          solve_problem / solve_with_context
                                                │
                     ┌──────────────────────────┼─────────────────────────┐
                     ▼                          ▼                         ▼
                 Solution                ProgressMessage             CLI stdout /
              (tour + total)            over mpsc channel          API JSON / WASM
                                                │
                                                ▼
                                 teeline-qt drains and renders
```

## Progress and threading

The library has no GUI toolkit dependency. A solver receives an optional
`mpsc::Sender<ProgressMessage>` and emits updates while it runs; consumers decide where
and how to render them.

- `teeline-cli` runs the solver and keeps stdout reserved for the result, so logging goes
  to stderr via `tracing`.
- `teeline-qt` runs the solver on a worker thread, drains the channel, and forwards
  updates to QML on the Qt main thread.
- `teeline-api` runs blocking solvers via `tokio::task::spawn_blocking`.
- `teeline-wasm` calls `solve_problem` without a progress channel.

## teeline-api layering

`teeline-api` follows a hexagonal, service-oriented split (see constitution §5):

```text
src/routes/    thin Axum handlers: extract → call one service → map response
src/services/  business logic behind TspSolverService / SolverRegistryService traits
src/models/    request/response DTOs
src/auth.rs    ApiKeyVerifier port (static key + external auth service)
lib.rs         build_api_router() / build_router() wiring and AppState
```

`AppState` holds `Arc<dyn TspSolverService>` and `Arc<dyn SolverRegistryService>`, so
handlers depend on interfaces and tests can substitute implementations.

## Build and test layout

- Unit tests live inline in each module (`#[cfg(test)]`).
- Library integration tests live in `tests/`.
- CLI end-to-end tests live in `tests/e2e/` as BATS scripts.
- Per-subproject tests: `teeline-api/tests/`, `teeline-wasm/tests/`,
  `teeline-web/` (vitest + Playwright), `teeline-qt/`.
- `Taskfile.yml` is the canonical local entry point (`task --list`).

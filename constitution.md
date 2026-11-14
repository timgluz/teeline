# Teeline Constitution

## Preamble

This document is the governing set of principles and constraints for the Teeline project.
It is more durable than the README (which documents how to use the software) and more
authoritative than AGENTS.md (which documents how agents operate day-to-day). Every
proposed change — an OpenSpec change, an ADR, or a pull request — must be consistent with
this document. Where a proposal conflicts with it, the constitution wins: amending it is a
deliberate act (see [Governance](#8-governance)) that must precede the proposal, not trail
it.

## 1. Vision

Teeline is a learning-oriented solver for the symmetric Traveling Salesman Problem, written
in Rust. Its purpose is to understand TSP algorithms deeply by implementing them correctly
and idiomatically — not to compete with production solvers such as Concorde.

It is a reference codebase: correctness and code clarity matter more than raw performance.

## 2. Core Principles

1. **Correctness first.** A slow correct solver beats a fast wrong one. No optimisation
   justifies an incorrect or unverified result.
2. **Idiomatic Rust.** Follow Rust conventions; Clippy runs with `-D warnings` in CI.
3. **Algorithmic completeness.** Implement at least one representative algorithm from every
   major TSP family (exact, constructive, local search, metaheuristic, approximation).
4. **Benchmark against reality.** TSPLIB known optima are the ground truth; tour quality is
   reported as a gap against those optima, never asserted from the solver's own output
   alone.

## 3. Non-negotiable Constraints

Hard constraints every change must preserve:

- **Input contract:** TSPLIB format, 2D Euclidean coordinates (`NODE_COORD_SECTION` or
  `DISPLAY_DATA_SECTION`), read from a file or stdin.
- **Output contract:** tour cost followed by ordered city IDs (`<tour_cost> <optimised_flag>`
  on the first line, then space-separated city IDs). This is the CLI's stable interface.
- **One dispatch mechanism:** every algorithm is a variant of the `Solvers` enum and is
  reachable only through the single `tsp::solve` dispatch. No solver may introduce a second
  entry point.
- **City IDs are 1-based** (TSPLIB convention); coordinates are stored as `f32`.
- **No `unsafe` code** unless unavoidable and justified in writing.
- **Exact algorithms are bounded:** Bellman–Held–Karp and Branch-and-Bound are exponential;
  they must never be presented as general-purpose or used on more than ~20–30 cities.

## 4. Architecture Guardrails

- **Solver registration** happens in `src/tsp/mod.rs` in four coordinated places: the
  `Solvers` enum, the `FromStr` impl, the `solve_with_context` dispatch arm, and
  `Solvers::all_meta()`. Updating fewer than all four is a defect.
- **`Solution` is the single tour representation** (`route`, `cities`, `total`). `total` is
  computed from the distance matrix when the solution is constructed; build a new
  `Solution` when the route changes rather than mutating one in place.
- **Spatial queries** go through the shared `nearest(&pt, n)` interface — `KDTree`
  (preferred) or `DistanceMatrix` (brute-force fallback) — returning `NearestResult`.
- **Composition via pipeline.** Local-search and stochastic solvers warm-start from a
  constructive seed rather than a random/sequential tour: deterministic hill-climbers seed
  from `nn`, stochastic solvers from `shuffle` (per the auto-expansion rules).
- **Progress and threading:** the library is GUI-free and reports progress as
  `ProgressMessage` values over an `mpsc` channel. The Qt GUI (`teeline-qt`) owns its event
  loop, drains the channel on a worker thread, and renders on the Qt main thread. Never add
  a GUI toolkit dependency to the solver library.
- **The root `teeline` crate is the pure solver library**, shared by all subprojects
  (teeline-cli, teeline-api, teeline-qt, teeline-wasm, teeline-web, teeline-excel). Solver logic lives
  there, never duplicated into a subproject.

## 5. API Design Principles (teeline-api)

The REST API follows a hexagonal, service-oriented layering. Dependencies point inward;
the web framework and external systems are adapters, not the core.

- **Lean routes.** A handler does HTTP work only — extract the request, call one service
  method, record metrics, map errors to a response. No business logic, no solver/parsing
  code, and no data access in a route.
- **Business logic in services.** Services own all domain logic (input→problem conversion,
  solver/config mapping, pipeline execution) and expose it behind a trait
  (`TspSolverService`, `SolverRegistryService`). Handlers depend on the interface, never
  the implementation.
- **Data only via repositories.** Any persistence or external-data access goes through a
  dedicated port/repository interface (for example `ApiKeyVerifier` for auth) — never
  reached directly from routes or services. Where such access does not yet exist, a new one
  is introduced as a repository, not ad-hoc calls.
- **Hexagonal (ports and adapters).** Dependencies are injected as `Arc<dyn Trait>` in
  `AppState`. Axum and external systems (the auth service) are adapters, swappable and
  mockable in tests without touching the core services.

## 6. Design Principles (teeline-web)

The web frontend follows a strict dark, restrained aesthetic — "no AI slop". The TSP
route/edge-line motif is the brand; colour comes from a fixed token palette; typography uses
Inter (weight 600 maximum) with JetBrains Mono for numbers and tokens; motion is functional
only; gradients, glassmorphism, decorative shadows, and count-up/parallax effects are
banned.

The full, exact rules (palette hex values, type scale, anti-pattern checklist) live in
[`docs/web-design.md`](docs/web-design.md). This constitution fixes the *principle* — dark,
restrained, route-motif, no decorative chrome — and defers the *exact values* to that
reference.

## 7. Relationship to ADRs

This constitution states what must always hold. Architecture Decision Records (ADRs) in
[`docs/adr/`](docs/adr/) record specific, dated decisions and their rationale. An ADR may
not contradict the constitution; a decision that would require violating a principle must
amend the constitution first and record the ADR against the amended text.

## 8. Governance

- The constitution is amended deliberately, in a PR whose scope is the constitution itself —
  not piggybacked onto an unrelated change.
- An amendment must state which principle or constraint it changes and why.
- Trivial typographic fixes are exempt from the amendment process but still go through
  review.
- The constitution is the single source of truth for principles; other documents may
  duplicate but never contradict it.

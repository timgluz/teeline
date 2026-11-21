# Proposal

## Why

Teeline's strongest symmetric-TSP results come from local search (Lin-Kernighan reaches
berlin52's optimum), but the project has no solver that recombines *multiple* high-quality
tours. Edge Assembly Crossover (EAX) is the best-known recombination operator for the TSP;
a memetic EAX solver adds a genuinely different evolutionary search family and a strong
standalone / warm-start solver. Issue #645 specifies the algorithm and its acceptance bar.

## What Changes

- Add `src/tsp/memetic_eax.rs`: an undirected fixed-degree `TourGraph`, AB-cycle
  decomposition, a `single_ab` / `rand` / `hybrid` E-set strategy, subtour patching,
  dirty-edge candidate 2-opt polish, and family-based replacement (a parent's best child
  replaces it only when strictly cheaper).
- Add a `MemeticEaxOptions` config struct (`epochs`, `pop_size`, `children_per_pair`,
  `strategy`, `stall_epochs`, `candidate_k`, `seed`) carried on `AppOptions`.
- Seed the population from randomized constructive starts (top-3 randomized nearest
  neighbour and shuffled tours) polished with candidate-restricted 2-opt; consume an
  optional warm-start tour as a seeded fraction, so `pipeline(nn, memetic_eax)` is
  meaningful.
- Register `Solvers::MemeticEax` with aliases `memetic` and `eax` in every solver
  registry, dispatcher, CLI, TOML, HTTP API, WASM and web surface; document it and add it
  to the benchmark index.
- Symmetric-only: `TYPE: ATSP` is already rejected by the parser, so the solver documents
  the constraint and adds no redundant runtime rejection.
- **Out of scope:** interactive web explainer telemetry. No snapshot transport exists
  today (the GA explainer is a client-side simulation); real EAX telemetry is a separate
  follow-up. This change emits `tracing` snapshots only.
- **BREAKING:** none. Additive solver and options only.

## Capabilities

### New Capabilities

- `memetic-eax`: the memetic Edge Assembly Crossover solver — population initialization,
  AB-cycle crossover, subtour patching, local polish, replacement/termination, symmetric
  instance constraint, configuration, registration, pipeline composition and TSPLIB
  quality expectations.

### Modified Capabilities

None. No existing requirement changes: the parser's ATSP rejection already establishes
the symmetric contract, and `distance-source` / `tour-construction` behavior is unchanged.

## Impact

- **Core:** new `src/tsp/memetic_eax.rs`; `src/tsp/mod.rs` (six registries +
  `AppOptions` + dispatch + auto-expand lists); `src/config.rs` stage tables;
  `src/tsp/pipeline.rs` (`stage_warnings` only if seed semantics require it); reuse of
  `graph.rs`, `candidate_list.rs` and `lin_kernighan.rs::double_bridge`.
- **CLI:** `teeline-cli/src/main.rs` flags, option mapping and help text.
- **API:** `teeline-api/src/models/request.rs` (`MemeticEaxConfig`,
  `SolverConfigs`), `services/tsp_service.rs`, `openapi.rs`.
- **WASM:** `teeline-wasm/src/lib.rs` (`build_opts`, kind, params, recommendation), WIT /
  tests, regenerated JS bindings.
- **Web:** `teeline-web/src/teeline-wasm.d.ts`, `solver-options.ts`, `nav-data.ts`
  (solver count 22 → 23, Metaheuristic group), `docs/algorithms/memetic-eax.md`.
- **Bench:** `bench/solvers.json` (`family: "meta"`, `deterministic: false`,
  `doc: "memetic-eax"`); add the missing `kroA100` entry to `bench/optima.json` so its
  published gap is computable.
- **Docs:** algorithm page, README and AGENTS solver tables.
- **Dependencies:** none added; no `unsafe`.

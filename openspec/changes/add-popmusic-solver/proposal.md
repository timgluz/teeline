# Proposal

## Why

Teeline's exact solvers are capped at `EXACT_MAX_CITIES = 24`, so the only scalable
improvement operators today are local search (`2opt`, `or_opt`, `lk`) and metaheuristics.
POPMUSIC turns a *small* exact Held–Karp path DP into a scalable improvement operator: it
repeatedly re-optimizes short contiguous tour windows with fixed endpoints and splices the
result back. It is deterministic, needs no training, works on coordinate metrics and
symmetric `EXPLICIT` matrices, and adds the project's first decomposition-family solver
(issue #770). The exact path DP already exists inside `bellman_karp.rs`; extracting it lets
both solvers share one verified implementation instead of duplicating the recurrence.

## What Changes

- Extract the exact fixed-endpoint Held–Karp path DP into `src/tsp/held_karp.rs` as a
  shared primitive, and refactor `bellman_karp.rs` onto it. BHK keeps its optimal cost;
  its route order and tie-breaking may change, and the `--verbose` DP-table dump is
  removed.
- Add `src/tsp/popmusic.rs`: cyclic window extraction/splice, the two-offset scan schedule
  (`R = min(14, n)`, `MAX_PASSES = 4`, strict improvement with an f32 epsilon, early stop),
  a consumed warm start or internal nearest-neighbour seed, and deterministic output.
- Register `Solvers::Popmusic` with aliases `popmusic` / `pop-music`, add it to
  `auto_expand_with_nn`, solver metadata and the benchmark index, and expose it through
  the CLI, TOML, HTTP API, WASM and web surfaces.
- Add `docs/algorithms/popmusic.md`; bump `docs/algorithms/bellman-held-karp.md` for the
  refactor.
- **BREAKING:** none. The solver is additive and BHK remains exact with the same output
  contract (tour cost followed by ordered city IDs).

## Capabilities

### New Capabilities

- `popmusic`: the POPMUSIC decomposition solver — cyclic window optimization with fixed
  endpoints, the two-offset scan and early stop, seeding/pipeline composition,
  determinism, registration and cross-surface exposure, and TSPLIB quality expectations.
- `held-karp-path`: the shared exact fixed-endpoint shortest-path DP — brute-force-equivalent
  optimality, deterministic reconstruction, bounded window size, and Bellman–Held–Karp's
  exactness preserved by using it.

### Modified Capabilities

None. No existing requirement changes.

## Impact

- **Core:** new `src/tsp/popmusic.rs` and `src/tsp/held_karp.rs`; `src/tsp/bellman_karp.rs`
  refactor; `src/tsp/mod.rs` (module declarations, `Solvers` enum, `variants()`, `FromStr`,
  `SolverMeta`, `SOLVER_LIST` 25 → 26, dispatch, `auto_expand_with_nn`).
- **CLI/TOML/API/QT:** picked up automatically through `Solvers::from_str`, `variants()`,
  `list_solvers()` and the existing fallback arms; the solver has no option struct.
- **WASM:** existing `build_opts` / `kind_for` / `params_for_solver` fallbacks cover it;
  optional `recommendation_for` hint.
- **Web:** `bench/solvers.json` (`family: "meta"`, `doc: "popmusic"`),
  `teeline-web/src/nav-data.ts` (`SOLVER_META.popmusic`, Metaheuristic group), and the new
  algorithm page.
- **Docs:** new algorithm page, BHK page version/changelog bump, README/AGENTS solver-table
  rows.
- **Bench:** Tier A `meta` run and publish.

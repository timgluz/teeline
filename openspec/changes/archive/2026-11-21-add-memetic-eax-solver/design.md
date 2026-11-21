# Design

## Context

See `proposal.md` — Why, and `specs/memetic-eax/spec.md` for the behavior contract. The
current state that shapes this design:

- `src/tsp/mod.rs` holds **six** solver registries that must stay in sync (`Solvers`
  enum, `variants()`, `FromStr`, `SolverMeta`, the fixed-size `SOLVER_LIST`, the
  `solve_with_context` dispatch), plus `AppOptions` and the `auto_expand_*` policy.
- The recent `add-sfc-solver` change established the cross-surface checklist: options
  struct, `config.rs` stage table, CLI, pipeline, HTTP API, WASM, web and benchmarks.
- Reusable primitives already exist: `graph.rs::UnionFind` and
  `hamiltonian_cycle_to_path`, `candidate_list.rs` (both coordinate and
  distance-matrix builders), and `lin_kernighan.rs::double_bridge`.
- The TSPLIB parser rejects `TYPE: ATSP` outright and reconstructs every symmetric
  matrix from one triangle; `DistanceSource` has no directed-edge concept.
- The only progress transport is `ProgressMessage`
  (`CityChange`/`PathUpdate`/`EpochUpdate`/`Done`/`Restart`/`OptimalTour`); the GA web
  explainer is a client-side simulation, not solver telemetry.
- Benchmarks are driven by `bench/matrix/tier-a.toml` (`meta = 300` seconds,
  `runs = 5`) and gap comes from `bench/optima.json`; `dsj1000`, `berlin52` and `a280`
  are present, `kroA100` is not.
- Verification is capped at instances under 2,000 cities because the materialized
  distance matrix is O(n²); all four target instances (52/100/280/1000) fit.

## Goals / Non-Goals

**Goals:**

- Correct EAX with every invariant checked in tests: degree two, alternating even AB
  cycles, shared edges untouched, degree-preserving patching, single component, strict
  family replacement.
- One coherent options path (CLI, TOML, API, WASM, web) and reproducible seeded runs.
- A benchmark result that records its own basis rather than asserting success.

**Non-Goals:**

- Web explainer snapshots — no transport exists and adding one is a separate change.
- A "stop when the known optimum is reached" channel — no solver receives the optimum
  today (`--optimal-tour` is a post-run comparison).
- Asymmetric/ATSP support — rejected by the parser by contract.
- Scale beyond the existing O(n²) distance matrix ceiling.
- New crates, `unsafe`, or a second dispatch path.

## Decisions

### 1. Standalone `MemeticEaxOptions`, not embedded `HeuristicOptions`

Add `MemeticEaxOptions` (`Clone, Debug, PartialEq, Default`, `validate`, `from_toml`,
`from_cli`) and `AppOptions.memetic_eax`, following `FourierOptions`/`SfcOptions`.
`candidate_k` and `stall_epochs` are not `n_nearest`/`stagnation_epochs`: `stall_epochs`
is dual-purpose (hybrid strategy switch **and** a 2× stop threshold), and `pop_size` /
`children_per_pair` have no `HeuristicOptions` home. Embedding the shared struct would
leak unused fields into every other solver's config contract.

### 2. Add `seed: Option<u64>`

The issue's option table omits it, but the benchmark protocol runs five seeds per
instance and the `determinism_evidence` convention needs reproducible runs.
`LKOptions` already establishes the pattern. Unseeded runs use system entropy.

### 3. Internal fixed-degree undirected tour graph

Represent a tour as `nbr: Vec<[Position; 2]>` (two neighbours per city) plus a
`slot: Vec<[u8; 2]>` index for O(1) edge lookup and removal. EAX operates on undirected
edge sets and rewires arbitrary components; the LK-style next/prev arrays encode a
fixed orientation that EAX's symmetric difference and subtour patching do not preserve.
The graph is converted to/from the flat `Vec<usize>` city-ID tour only at the boundary,
using `hamiltonian_cycle_to_path`-style tracing in O(n).

### 4. AB-cycle decomposition over the union multigraph

Build the union of parent A and B edges, tagging origin. A-only and B-only edges form
the multigraph; traversing from an unvisited A-edge, alternating A→B→A, yields an AB
cycle. Validate even length and alternation; shared edges are absent from the union, so
they can never be treated as changes. Cycles are applied to A as a symmetric difference
(remove A-only edges, add B-only edges), which preserves degree two by construction.
When parents are identical the union is empty: perturb with `double_bridge` and polish
rather than iterating on an empty cycle set.

### 5. E-set strategies and pairing

Implement the issue's three strategies verbatim: `single_ab` (pick one cycle), `rand`
(independent 50% per cycle), `hybrid` (behave as `rand`, then switch to `single_ab` after
`stall_epochs` consecutive stagnant generations). Pair parents by shuffling a permutation
of population indices and taking adjacent pairs, skipping self-pairs; handle populations
of size 0/1 and odd sizes by leaving the unmatched member unpaired.

### 6. Subtour patching via components and candidate bridges

After applying an E-set, find connected components with `UnionFind` and greedily merge
the smallest component into another using the shortest candidate-list edge whose endpoints
lie in different components. Each bridge removes one edge per component and adds two, so
degree two is preserved and the component count drops by one. If no candidate edge
connects two components (possible when candidate lists are sparse), fall back to a full
scan of all city pairs for the cheapest bridge. Assert a single component after patching.

### 7. Dirty-edge candidate 2-opt

A candidate-restricted 2-opt pass seeded by the endpoints of newly added bridge edges:
push those vertices onto a work queue, accept improving reversals, and push the endpoints
of each accepted move. The scan is bounded by `candidate_k`, so a pass is O(dirty · k).
Every move is scored against the problem's exact distance source. This is deliberately
different from `two_opt.rs`'s full best-improvement pass: EAX needs propagation from the
newly disturbed region, not a global rescan of an already-local-optimal parent.

### 8. Family-based replacement and termination

Each parent accumulates a pool of `children_per_pair` offspring (crossover → patch →
polish). The best child in the pool replaces that parent only if strictly cheaper
(`child.total < parent.total`). This preserves niches and avoids a global best-replaces-
worst step. Track global best and stagnant generations; stop at `epochs` or after
`2 * stall_epochs` stagnant generations. The `hybrid` switch is the *earlier* use of
`stall_epochs`; the stop is the later, 2× use.

### 9. Candidate lists from the problem's metric

Build with `CandidateList::build` when `is_coordinate_metric()`, else
`CandidateList::build_from_distances`, mirroring `lin_kernighan.rs`. `candidate_k` is
clamped to `min(candidate_k, n - 1)`; the default 20 is above the local-search floor of
16. Candidate lists only prune; every accepted permutation is scored exactly.

### 10. Pipeline: consume the seed, do not auto-expand

The first stage's tour (or `init_tour`) seeds one population member; the remaining
members come from randomized constructors and their polished local optima. This makes
`pipeline(nn, memetic_eax)` and `pipeline(greedy_edge, memetic_eax)` meaningful rather
than silently discarding the seed. Because the seed is consumed, no `stage_warnings` arm
is needed. The solver is absent from both `auto_expand_with_nn` and
`auto_expand_with_shuffle` — it builds its own population. Alternative considered:
ignore the seed like `sfc` and warn (rejected — wastes the pipeline's information).

### 11. Symmetric constraint: rely on the parser, document it

`TYPE: ATSP` never reaches a solver, and `DistanceSource` cannot express a directed
edge, so a solver-level symmetry check would be unreachable code. The solver documents
the constraint; a separate parser change could validate that a `TYPE: TSP` full matrix
is actually symmetric, but that would contradict the parser's documented and tested
lower-triangle-wins behavior and is out of scope.

### 12. "Stop on known optimum" is not implemented

No solver receives the optimum, and `solve_with_context` has no such parameter. Threading
one through the dispatch, API, WASM and pipeline would be a cross-cutting change for one
solver. The termination contract is epochs + stagnation only; reaching a TSPLIB optimum is
a benchmark outcome, not a stop condition.

### 13. Registration is the full cross-surface checklist

Add the variant, `variants()` aliases, `FromStr`, `SolverMeta`, `SOLVER_LIST` (bump
`[SolverInfo; 23]` → `24`), dispatch, `AppOptions`, `config.rs` stage tables and
per-solver validation, CLI flags, API `SolverConfigs` mapping, WIT/WASM builder and
params, web `teeline-wasm.d.ts`/`solver-options.ts`, `nav-data.ts` (`SOLVER_META` 22 → 23,
Metaheuristic group), `bench/solvers.json` and the algorithm doc. Round-trip tests plus
the existing registry-consistency tests cover the core; the web suite pinpoints any
missing metadata.

### 14. Benchmarks: targets are recorded, not asserted silently

The four issue targets are the acceptance bar, measured by the existing tier harness
(`runs = 5`, `meta` budget 300 s). `kroA100` must be added to `bench/optima.json` (with
the TSPLIB published source) or its gap is `null`. The EUC_2D `metric_note` on
`berlin52`/`a280` must be reconciled with the current rounding behavior before a 0.00%
gap is published. `bench/solvers.json` gets `family: "meta"`, `deterministic: false`
(stochastic, unmeasured) and `doc: "memetic-eax"`.

### 15. Telemetry: `tracing` only

Emit structured `tracing` events (generation, global best, stall count, cycle count,
components before patching, bridges added) under the existing logging setup. No new
`ProgressMessage` variant and no web explainer in this change; the issue's own
"identify a supported transport before designing a new one" is satisfied by the finding
that none exists.

## Risks / Trade-offs

- **Wrong AB-cycle or patch logic** → property tests for alternation, parity, degree,
  shared edges, component reduction, and a debug-build invariant check after each
  crossover/patch pass.
- **Hardware-sensitive benchmark targets** → record machine, build mode, seed and
  elapsed scope; report measured results instead of weakening checks (per issue).
- **`kroA100` has no optima entry** → add it before benchmarking; otherwise report
  `tour_cost` only.
- **Stale EUC_2D metric note** → verify the rounding behavior; if still unsettled,
  publish `tour_cost` and keep the gap target as a recorded aspiration.
- **Silent API config drop** → add `MemeticEaxConfig` and the `make_app_options` arm,
  with a request-level test proving the config reaches the solver.
- **Web suite hard-fails on missing metadata** → update `nav-data.ts` and add the doc
  page in the same change; the tests make the omission visible rather than silent.
- **Per-child allocation in hot loops** → reusable scratch buffers for graphs, cycles,
  components and candidate lists; no allocation proportional to `children_per_pair`
  inside the generation loop beyond the child pools.
- **Candidate lists are approximate on GEO/ATT/CEIL_2D** → exact scoring of every move
  and quality bounds on the benchmark instances.
- **Tiny/degenerate populations and instances** → clamp `pop_size`/`candidate_k`,
  handle n < 4 and identical parents explicitly.

## Migration Plan

Additive only. No data migration and no breaking API changes: an unknown solver name
still fails cleanly, and the new `SolverConfigs` field is optional. Rollback is deleting
the solver module and its registration entries; no persisted state depends on it.

## Open Questions

None that change the specs or approach. The only deferrable item is whether the web
explainer gets real telemetry, which is explicitly a follow-up.

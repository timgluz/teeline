# Design

## Context

See `proposal.md` for motivation. The current state that shapes the approach:

- `TspProblem` (`src/tsp/mod.rs:1902`) is `{ cities: Vec<KDPoint>, distances: DistanceMatrix }`; `DistanceMatrix` (`src/tsp/distance_matrix.rs:145`) packs the lower triangle into `items: Vec<f32>` of `n(n-1)/2` entries plus a city table and an id→position index.
- Matrix positions are the insertion order of the `cities` slice: `build_with` inserts `city_table.insert(i, *pt1)` while enumerating that slice, and every caller passes that same slice to `TspProblem::new`. A coordinate-indexed backend can therefore use `problem.cities` indices as positions.
- The matrix is built **with** the four TSPLIB primitives (`euc_2d_distance`, `geo_distance`, `att_distance`, `ceil_2d_distance`, `src/tsp/distance_matrix.rs:59–127`). `from_cities_unrounded` is a fifth, test-only path; no backend needs to represent it.
- `TspProblem::new` has **80** call sites in the main tree, mostly tests; keeping its call shape stable minimises churn.
- `TspProblem` crosses threads: `teeline-api` uses `tokio::task::spawn_blocking` (`tsp_service.rs:320,353`) and `teeline-qt` uses `std::thread::spawn` (`solver_engine.rs:248,280,371,415`). The backend must be `Send + Sync`.
- `teeline-wasm` and `teeline-qt` are **outside the workspace** (each declares its own `[workspace]`), and root `default-members = ["."]`, so a plain `cargo test`/`cargo clippy` does not cover the CLI, API, Qt, or WASM.
- `lin_kernighan::solve` builds its own EUC_2D matrix from `problem.cities` (`lin_kernighan.rs:41`) and never reads `problem.distances`. It therefore ignores GEO/ATT/CEIL_2D/EXPLICIT metrics and would still allocate O(n²) at 100k.
- EXPLICIT input has no real coordinates: the parser fills `grid_coords(dimension)` placeholders (`tsplib.rs:227-230`), while `distance_matrix()` keys off `raw_distances`. The CLI's `--distance-type` override (`main.rs:524-534`) mutates `distance_type` after parsing, so selection must **not** trust that field alone.
- `constitution.md` §4 names `DistanceMatrix` in the `Solution.total` and spatial-query guardrails. Per §8, the amendment is a separate PR and must precede this change.

## Goals / Non-Goals

**Goals:**

- One trait boundary; the backend is chosen once at construction and swapped without touching solver code.
- Byte-identity is structural: every backend shares one set of default methods (summation order, id mapping, edge cases) and differs only in the primitive `distance_by_pos`.
- Coordinate-only input at 100k cities runs `sfc` end-to-end under the documented memory ceiling.
- The memory-versus-runtime trade-off is measured on the **same cells** with the existing benchmark harness, with documented thresholds.
- Phase 1 is behaviour-preserving except for the explicit `lin_kernighan` bug fix, which is called out and re-baselined.

**Non-Goals:**

- No generic parameter on `TspProblem` or the solvers (`TspProblem<D>`); that ripples through every public signature and the WASM/API surfaces.
- No out-of-core, distributed, SIMD, or GPU distance kernels.
- No change to TSPLIB distance semantics, the output contract, or the `Solvers` dispatch path.
- Phases 3–5 are planned but are not prerequisites for the trait and on-the-fly core.

## Decisions

### D1: One trait, with defaults that make byte-identity structural

```rust
pub trait DistanceSource: Send + Sync {
    // Required primitives.
    fn num_cities(&self) -> usize;
    fn distance_by_pos(&self, a: usize, b: usize) -> Result<f32, &'static str>;
    fn pos2city_id(&self, pos: usize) -> Option<usize>;
    fn city_id2pos(&self, city_id: usize) -> Option<usize>;
    fn point_at(&self, pos: usize) -> Option<KDPoint>;
    /// False when the cities are TSPLIB EXPLICIT placeholders rather than real coordinates.
    fn has_coordinates(&self) -> bool { true }

    // Shared defaults, identical for every backend.
    fn distance_between(&self, a: usize, b: usize) -> Result<f32, &'static str> { /* id→pos → distance_by_pos */ }
    fn tour_length(&self, path: &[usize]) -> f32 { /* id→pos → tour_length_by_pos; unknown id → 0.0 */ }
    fn tour_length_by_pos(&self, path: &[usize]) -> f32 { /* closing edge first, as today */ }
    fn distances_from(&self, city_id: usize) -> Vec<f32> { /* scan positions */ }
    fn nearest(&self, target: &KDPoint, n: usize) -> NearestResult { /* scan positions */ }
    fn nearest_unvisited(
        &self,
        target: &KDPoint,
        is_candidate: &dyn Fn(usize) -> bool,
    ) -> Option<NearestResultItem> { /* min by (distance, id) */ }
}
```

Providing defaults is the key correctness decision: the matrix and on-the-fly backends cannot drift in summation order (`tour_length_by_pos` adds the closing edge first today), id handling, or tie-breaking, because only `distance_by_pos` differs. The error/edge behaviours are preserved deliberately: `distance_by_pos(p,p)` returns `Ok(0.0)` before range checking, an unknown id makes `tour_length` return `0.0`, and `distances_from` panics on an unknown id.

The hot-path translation in `tour_length` is single-pass and allocation-free; solvers call it inside candidate loops, and the pre-trait version allocated a position vector per call.

**Alternative considered:** a required method per backend (the current inherent API). It duplicates loops and invites drift, and the "byte-identical" claim would rest on every implementer redoing the arithmetic in the same order.

`nearest_unvisited` takes `&dyn Fn(usize) -> bool` rather than `impl Fn` so the trait stays object-safe. Call sites pass `&|id| ..`.

### D1a: Positions and city IDs stay distinct

`DistanceSource` positions are dense `0..n` indices into `TspProblem.cities`; city IDs are external identifiers (TSPLIB 1-based, API-supplied, possibly non-contiguous). The conversion boundary is `pos2city_id`/`city_id2pos`; `distance_by_pos` and `point_at` take positions, `distance_between`/`tour_length` take IDs, and `Solution.route` stays city IDs for the output contract.

This change does not type-enforce the distinction. The full `CityId`/`Position` newtype refactor, the `KDPoint::new` id-0 removal, the `nearest` exclusion-sentinel removal, and one canonical solver convention are captured in the follow-up `city-id-position` change. To keep this change honest in the meantime, the conformance suite includes a non-contiguous-ID case, and the existing 1-based TSPLIB fixtures already exercise IDs that differ from positions.

### D2: Store `Arc<dyn DistanceSource>`, not `Box<dyn>`

```rust
#[derive(Clone)]
pub struct TspProblem {
    pub cities: Vec<KDPoint>,
    pub distances: Arc<dyn DistanceSource>,
}
```

`Arc` makes `TspProblem::clone()` O(1); today it deep-copies the whole matrix, and `build_evaluator` in `branch_bound.rs:94` and `genetic_algorithm.rs:131` deep-copies it again into an `Rc`. The source is immutable, so sharing is safe; `DistanceSource: Send + Sync` makes the `Arc` shareable across the API's `spawn_blocking` and Qt's worker threads. **Alternatives:** `Box<dyn>` needs a `clone_box` method and still deep-copies on `TspProblem::clone`; a generic `TspProblem<D>` avoids dispatch but ripples generics into all solvers and the public API.

### D3: Keep `TspProblem::new` compatible; add constructors in two steps

```rust
impl TspProblem {
    pub fn new(cities: Vec<KDPoint>, distances: impl DistanceSource + 'static) -> Self;
    pub fn from_source(cities: Vec<KDPoint>, distances: Arc<dyn DistanceSource>) -> Self;
    // Added with DistanceBackend in Phase 2 (PR 7), not in the trait PRs:
    pub fn from_points(cities: Vec<KDPoint>, dt: DistanceType, backend: DistanceBackend) -> Result<Self, String>;
}
```

`new` wraps its argument, so all 80 `TspProblem::new(cities, dm)` call sites keep compiling. PR 4 adds only `new` and `from_source`; `from_points` cannot exist before `DistanceBackend` and `OnTheFly`, so it lands with the selection policy in Phase 2. It is deliberately **not** named `from_cities`, to avoid colliding with the existing "EUC_2D matrix from points" meaning.

`from_source` verifies the position invariant that solvers already assume (`greedy_edge.rs:56` indexes `cities[pos]`): `num_cities() == cities.len()` and `pos2city_id(i) == cities[i].id` for every `i`, at least under `debug_assert`. `OnTheFly` also asserts unique city ids, mirroring `DistanceMatrix::new`.

### D4: Land the trait in two steps to keep every PR green

PR 3 defines `DistanceSource` with its defaults and implements it for `DistanceMatrix` **while keeping the existing inherent query methods** (including `nearest_unvisited(impl Fn)`). The workspace therefore still compiles with `TspProblem.distances` concrete.

PR 4 removes the now-redundant inherent methods and, in the same PR, switches `TspProblem.distances` to `Arc<dyn DistanceSource>`, updates `nearest_unvisited` to `&dyn Fn`, and adds the `use DistanceSource` imports. Those changes must land together: removing an inherent method before the imports and field change would break every concrete caller.

`DistanceMatrix` implements the five required methods (`distance_by_pos`, `city_id2pos`, `pos2city_id`, `point_at`, `num_cities`) and inherits the six defaults. Matrix-only accessors (`distances()`, `len()`, `is_empty()`, `num_distances()`, `city_index()`) stay inherent and are not part of the trait.

Consequence: code calling a query method on a **concrete** `DistanceMatrix` needs `use ...::DistanceSource`, while calls through `problem.distances` (a `dyn`) do not. Affected files include `tests/test_kdtree_and_distance_matrix.rs`, `nn_determinism_test.rs`, `or_opt_test.rs`, `three_opt_test.rs`, `christofides_test.rs`, `greedy_edge_test.rs`, `savings_test.rs`, `lin_kernighan_test.rs`, the CLI (`main.rs:736`) and WASM (`lib.rs:573`). The goal is "every `TspProblem::new` call site compiles", not "no test file changes".

### D5: `OnTheFly` implements only the primitives

```rust
pub struct OnTheFly {
    cities: Vec<KDPoint>,                 // position-indexed, same order as TspProblem.cities
    city_idx: HashMap<usize, usize>,
    distance: fn(&KDPoint, &KDPoint) -> f32,
}
```

The constructor maps `DistanceType` to one of the four primitives, rejects `Explicit`, and mirrors `DistanceMatrix::build`'s `n < 2` rejection. `nearest`, `nearest_unvisited`, and `distances_from` are the shared defaults; only `distance_by_pos` computes. Using the same primitive function pointers is what makes the byte-identical guarantee structural. `point_at(pos)` returns `cities.get(pos).copied()`.

`has_coordinates` is the signal Phase 3 needs: `DistanceMatrix::new` (the explicit-weight constructor) overrides it to `false`, while `build`/`from_cities` and `OnTheFly` report `true`. A candidate list built from `grid_coords` placeholders would rank meaningless neighbours, so EXPLICIT input skips coordinate-based pruning.

Edge parity is explicit: `distance_between(a, a)` checks identity **before** the id lookup (as `DistanceMatrix` does), and `OnTheFly::distance_by_pos(p, p)` short-circuits to `Ok(0.0)` before bounds or the metric, because `geo_distance(p, p)` is `1.0`, not `0.0`.

### D6: Select by explicit weights, never by the declared type

```rust
pub enum DistanceBackend { Auto, Matrix, OnTheFly }

impl TspLibData {
    pub fn to_problem(&self, backend: DistanceBackend) -> Result<TspProblem, String>;
}
```

- `has_explicit_weights()` is the source of truth. EXPLICIT weights → always `Matrix`, regardless of `distance_type` (which `--distance-type` can override). Selecting `OnTheFly` for such input is an error.
- Coordinate input + `distance_type == Explicit` is an error, not a panic. This also fixes today's `distance_matrix::build(..).unwrap()` panic at `distance_matrix.rs:140` when `--distance-type explicit` is used on a coordinate file.
- Coordinate types: `Auto` → `Matrix` for `n <= threshold`, else `OnTheFly`. Threshold in one constant (candidate `20_000`), documented and boundary-tested.
- CLI `--distance-backend matrix|on-the-fly|auto` (default `auto`) and a **top-level** TOML key `distance_backend` (not per `[[stage]]`, because it applies to the whole problem). `src/config.rs` currently returns only `Vec<(Solvers, AppOptions)>`; it must gain a top-level config path (a `PipelineConfig` or a companion parser) plus `tests/pipeline_config_test.rs` coverage. CLI wins over TOML; invalid values are rejected. Do not give the clap arg a `default_value` if precedence is checked with `get_one` — either set it after merging TOML or inspect `ValueSource::CommandLine`, or the default silently overrides the TOML value.
- All constructors call `to_problem`/`from_points`: CLI `main.rs:540`, API `tsp_service.rs:44,52`, WASM `lib.rs:116,121,317,550,571`, Qt `solver_engine.rs:260,396`. The WASM scoring-only paths (`compare_tours_from_input`, `tour_distance_from_input`) must pick `Matrix` when `has_explicit_weights()` and `OnTheFly` otherwise — using `OnTheFly` unconditionally would reject valid EXPLICIT inputs that work today. The CLI's `compute_optimal_comparison` can reuse the problem's source.

### D7: Migrate in behaviour-preserving steps, with one explicit bug fix

0. **Prerequisite PR:** amend `constitution.md` §4 to name a distance source instead of `DistanceMatrix` in the `Solution.total` and spatial-query guardrails (§8 requires a constitution-only PR). Update AGENTS.md's "Core types" in this change.
1. Trait + defaults + matrix impl; `TspProblem.distances` becomes `Arc<dyn DistanceSource>`; helper parameters become `&dyn DistanceSource` (or `&Arc<dyn DistanceSource>` where the callee owns a clone, i.e. `build_evaluator`); add the `use DistanceSource` imports. Record a pre-refactor `matrix`-config baseline on the base commit and commit it to a tracked path (not `bench/runs/`, which is git-ignored); use it for the no-regression check.
2. Fix `lin_kernighan` to use `problem.distances`, and add an optional seed so the backend-equivalence test is reproducible. This **changes behaviour** on GEO/ATT/CEIL_2D/EXPLICIT instances (it is a bug fix, not a refactor), so it gets its own commit and re-baselined LK numbers.
3. Add `OnTheFly` + the shared conformance suite; no selection yet.
4. Add `to_problem`/`from_points`, the flag, and the backend-comparison sweep. Keep the default `Matrix` until the sweep confirms the runtime tolerance; then flip to `Auto` and record the threshold.
5. Phase 3 candidate lists — solver-side neighbourhood-limited move evaluation, not a `DistanceSource` backend (see Risks).
6. Phase 4 compact EXPLICIT; Phase 5 SFC 100k through the normal dispatch.

### D8: Benchmark on the existing harness, with explicit thresholds

Reuse `scripts/bench-matrix.sh` (GNU `time -v` peak RSS in a fresh process per cell, JSONL rows per `docs/benchmarks/schema.md`, per-row provenance). Do **not** build a parallel RSS harness.

- **Comparison tier:** `bench/matrix/tier-distance.toml`, writing `bench/runs/tier-distance.jsonl`. Phase 0 scaffolds the `matrix` family: `matrix` for the main solver set, `matrix-3opt` scoped to instances where `3opt` finishes, and `matrix-fixed` for `tabu --epochs 1000`; all take no backend flag (the pre-selection default). Phase 2 mirrors it as `on-the-fly`, `on-the-fly-3opt`, and `on-the-fly-fixed`. Before the default flips to `Auto`, every config in `tier-distance*.toml` must pass `--distance-backend` explicitly, or a `matrix` cell silently becomes on-the-fly above the threshold. Scope to coordinate instances so EXPLICIT cells do not produce rejection `error` rows.
- **Planner shape:** `bench-matrix-plan.py` requires every solver family in `bench/solvers.json` to appear in `[coverage]` and `[budgets]`, and it does not cross-product configs. Give each config an explicit `solvers = [...]` list and include `utility = []` (plus its budget entry) so the tier validates.
- **Solvers:** local `2opt`, `or_opt`, `lk`, and `3opt` only where it finishes; constructive `nn`, `sfc`; meta `tabu` fixed at `--epochs 1000` (`sa` is not used because a below-schedule `--epochs` is raised to the schedule); exact `bhk` only on `burma14`/`ulysses16` — `bhk` is `1 << (n-1)` and cannot touch the 52+ city fixtures.
- **Instances:** tracked fixtures `burma14`, `ulysses16`, `berlin52`, `a280`, `att532` form the default comparison tier. A separate `bench/matrix/tier-distance-large.toml` covers downloaded instances (`dsj1000`, `pla7397`, `pla33810`) and documents `download_data.sh` as a prerequisite; the planner aborts on a missing instance, so they must not sit in the default tier.
- **Fixed work, not timeouts:** measured on `att532`, `2opt` (0.2 s) and `or_opt` (14.5 s) finish, while `3opt` exceeds 60 s; `3opt` is therefore scoped to `a280` and smaller. `tabu --epochs 1000` is the fixed-work meta run. `2opt`/`or_opt`/`3opt` ignore `--epochs`; compare only `status: ok` rows.
- **Thresholds:** set before the sweep and recorded with the data — a per-family slowdown tolerance on median `wall_s` over five runs, a memory expectation (on-the-fly `peak_rss` ≤ coordinate/KD-tree overhead, and a documented factor lower than the matrix at/above the threshold), and the Phase 1 no-regression bound recorded in Phase 0.
- **Multi-run evidence:** the harness runs five runs per cell; every improvement claim is judged on the median and distribution, never a single run. A change that wins on one run but not on the median is not an improvement (constitution §2.5).
- **Threshold enforcement:** a dedicated `scripts/check-distance-benchmarks.py` reads the local JSONL and fails on a runtime or memory violation. Do not extend `scripts/read-benchmarks.sh`, which fetches published shards over HTTP.
- **Gate:** a manual per-release sweep on a documented reference machine; the report is committed and reviewed. A nightly small-instance smoke (fixed work, running the checker) is a follow-up once the thresholds stabilize; there is no per-PR CI job.
- **Publication:** internal-only. Write the comparison to a run file `publish-benchmarks.sh` does not consume; the public leaderboard continues to measure the default backend, unchanged.
- **100k scaling:** a separate run outside `cargo test` and outside the published tiers — synthetic generator, one fresh process per size, its own budget and output file. Matrix sizes beyond the machine's RAM are recorded as OOM/not-measured, never fabricated. The EXPLICIT row stops at the largest feasible size: `parse_explicit` materialises an n×n f32 matrix, so 100k is not generated and is recorded as not-measured.
- **Report:** `docs/benchmarks/distance-source.md`; commands `scripts/bench-matrix.sh bench/matrix/tier-distance.toml` (plus a `task bench:distance` entry) and the separately documented scaling command. Criterion stays only for construction time in `benches/distance_matrix.rs` with a `[[bench]]` entry.

## Risks / Trade-offs

- **Dyn dispatch in hot loops** — `two_opt`/`three_opt`/`lk` call `distance_between` in O(n²)–O(n³) loops. Measure the refactor against `tier-a.jsonl`; the hot paths already do two `HashMap` lookups per call, so the vtable call may be in the noise. Note that making a helper generic over `D: DistanceSource + ?Sized` while passing `&*problem.distances` still sets `D = dyn DistanceSource` and does **not** monomorphise; real monomorphisation needs a dispatch at the solver entry. Do not promise the fallback without a measurement.
- **Object safety** — `nearest_unvisited` must take `&dyn Fn(usize) -> bool`; one production call site (`nearest_neighbor.rs:50`) and six test call sites.
- **Public field type change** — `problem.distances` is `pub`; API/WASM/Qt/CLI and tests binding `&problem.distances` must be updated. WASM and Qt are outside the workspace, so `cargo clippy --workspace` misses them.
- **Byte-identity** — structural via shared defaults, but still guarded by a conformance suite over every backend, every distance type, and the error paths; a future backend that bypasses a primitive must fail it.
- **Selection correctness** — keying off `distance_type` reads `--distance-type` overrides and placeholder EXPLICIT coordinates; `has_explicit_weights()` is the only safe signal.
- **`lin_kernighan` behaviour change** — fixing its private matrix changes tours on GEO/ATT/CEIL_2D/EXPLICIT. Publish it as a bug fix with re-baselined numbers; do not fold it into the "no behaviour change" claim.
- **Solvers that remain O(n²) whatever the backend** — `greedy_edge`, `christofides`, `savings` (`graph.rs::sorted_edges`), `ant_colony` (n×n pheromone/eta), and `genetic_algorithm` (population size n). Document this; the 100k path is `sfc`, not a blanket claim for every solver.
- **EXPLICIT peak memory is parse-dominated** — `parse_explicit` builds a full n² matrix plus a token vector before packing (`tsplib.rs:304-315`), and `distance_matrix()` clones the packed result. Phase 4's compaction covers only the stored part; measure parse-time RSS and consider a consuming `into_problem` to drop the clone.
- **Measurement traps** — timeouts hide slowdown; process startup dominates small instances; `VmHWM` inside the shared `cargo test -- --ignored` process includes every other test. Each measurement runs in its own process.
- **Candidate lists are not a distance backend** — `two_opt`/`or_opt` still need exact distances for scoring; a candidate list bounds which moves are evaluated and uses O(n·k) memory. LK already builds KD-tree candidate lists (`lin_kernighan.rs:12-26`), and KD-tree neighbours rank by raw Euclidean distance, which is wrong for GEO latitude/longitude. Phase 3 extends that approach; it does not add a `DistanceSource` variant.
- **On-the-fly CPU cost** — acceptable for constructive/metaheuristic solvers; the tolerance and the Phase 3 move evaluation are the escape hatches.

## Migration Plan

- **Before this change:** the constitution amendment PR (§4) and its approval.
- No data migration. The API change is source-level and fully in-repo.
- Rollback lever: force `DistanceBackend::Matrix` everywhere (config only), so the trait can ship before the policy is trusted.
- Order: constitution amendment → Phase 0 benchmark harness/measurements → Phase 1 trait refactor (no behaviour change) + no-regression sweep → LK fix + re-baseline → `OnTheFly` + conformance → Phase 2 selection + comparison sweep → Phase 3 → Phase 4 → Phase 5.

## Delivery: stacked PRs

The plan ships as a stack, one reviewable concern per PR. Each PR targets the previous PR's branch, carries its own tests and docs, and passes `cargo test --workspace`, the repo gate `cargo clippy --workspace -- -D warnings`, and `cargo fmt --check` before review. After a PR merges, retarget the next one to the new base and confirm its diff shows only its own scope. Branches follow AGENTS.md's `type/short-description`; PRs open with `gh pr create`.

| # | Branch | Base | Scope | Tasks |
|---|---|---|---|---|
| 1 | `docs/constitution-distance-source` | `master` | Constitution §4 amendment only (§8) | 1.1 |
| 2 | `bench/distance-backend` | PR 1 | Tier scaffold (matrix config), scaling script, matrix baseline, report scaffold | 2.1–2.11 |
| 3 | `refactor/add-distance-source-trait` | PR 2 | Trait + defaults + impl; keep inherent methods; no field change; ADR 010 | 3.1–3.2, 3.13 |
| 4 | `refactor/use-distance-source` | PR 3 | AGENTS.md, remove inherent methods, field switch, helpers, imports, constructors, no-regression | 1.2, 3.3–3.12 |
| 5 | `fix/lk-declared-metric` | PR 4 | LK seed (incl. API `LKOptions` consumers) + metric fix + re-baseline | 4.1–4.3 |
| 6 | `feat/on-the-fly-backend` | PR 5 | `OnTheFly` + conformance/equivalence | 5.1–5.8 |
| 7 | `feat/distance-backend-selection` | PR 6 | Policy, `config.rs`, flag/TOML, WASM scoring, on-the-fly config, comparison, thresholds, gate/publication, 100k | 6.1–6.13 |
| 8 | `perf/candidate-lists` | PR 7 | KD-tree neighbour move evaluation | 7.1–7.5 |
| 9 | `perf/compact-explicit` | PR 8 | Compact EXPLICIT storage | 8.1–8.6 |
| 10 | `docs/sfc-100k` | PR 9 | SFC 100k verification, docs, integration checks | 9.1–9.3, 10.1–10.8 |

PRs 8 and 9 are independent of each other and may be reordered, or branched in parallel off PR 7, if the stack grows too long. PR 4 is the largest mechanical diff (the field-type change touches every consumer); PR 3 exists so that its review is the trait design alone.

## Open Questions

- The threshold default (20k candidate) — confirm with the Phase 0/2 data before flipping `Auto` on.
- The per-family runtime tolerance — set from the first comparison sweep; it must be documented before that phase merges.
- The candidate-list quality tolerance (Phase 3) — expressed as a gap against the published optimum; set from the measured comparison.
- Phase 5's SFC path may reduce to using the selected source unchanged; it stays in scope because the `tour-construction` spec requires 100k execution, not a separate entry point.

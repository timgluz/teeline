# Design

## Context

See `proposal.md` — Why. The current state that shapes the approach:

- `tsplib.rs::process_lines` fabricates `grid_coords(dimension)` when an EXPLICIT file has neither `NODE_COORD_SECTION` nor `DISPLAY_DATA_SECTION`, and `TspLibData` has no record that those coordinates are placeholders.
- `TspLibData::cities()` is the coordinate source for every parse surface: Qt (`teeline-qt/src/file_loader.rs`), WASM (`teeline-wasm/src/lib.rs` → `teeline-web`), and the API (`teeline-api/src/services/tsp_service.rs`). `Solution::cities()` is not used by those surfaces.
- `DistanceSource::has_coordinates()` exists and is `false` for an EXPLICIT `DistanceMatrix`; `two_opt` and `or_opt` already use it to fall back to a full scan.
- `TspProblem` is `{ cities: Vec<KDPoint>, distances: Arc<dyn DistanceSource> }`; every geometry-based solver and `branch_bound`'s beam read `problem.cities`.
- `CandidateList::build` ranks neighbours with a KD-tree over `cities`; `lin_kernighan` calls it unconditionally.
- `scripts/lib/problem-minimap.mjs` already derives a 2D layout for problem minimaps with classical MDS, and the problem page already labels it (`minimapDerived` in `docs/problems/*.md`). The Rust implementation must be correct where that script is fragile.
- The crate has no linear-algebra dependency (`num-complex` is the closest thing).
- The in-flight `city-id-position` change will add `CityId`/`Position` newtypes and a shared `CityIndex`; it also touches `candidate_list.rs` and claims ADR 011, which is already taken by `011-remove-codecov.md`.

## Goals / Non-Goals

**Goals:**

- Give every solver a meaningful layout or neighbourhood on coordinate-less EXPLICIT instances, without a new dependency.
- Put the derived layout in `TspLibData.cities` at parse time, so parse surfaces and `TspProblem` cannot disagree.
- Keep the derived layout deterministic and exact-scored (solvers still score moves with the real distance matrix).
- Preserve current tours on every coordinate-metric instance (`EUC_2D`, `GEO`, `ATT`, `CEIL_2D`).

**Non-Goals:**

- Recovering the real geography of an EXPLICIT instance; the layout is a distance-preserving sketch, not a georeference.
- Changing distance metrics, backend selection, or the CLI/API tour output.
- Making the layout exact for non-Euclidean matrices.
- Redesigning the web map; this change labels the layout that the problem page already derives.

## Decisions

### D1: Embed at parse time, replacing the fabricated grid coordinates

When `process_lines` finds `raw_distances.is_some()` and no coordinate/display section, it builds the city list from `mds_embed` instead of `grid_coords(dimension)`, and records `has_real_coordinates = false`. `TspLibData.cities`, `DistanceSource::point_at`, and `TspProblem.cities` then share one list by construction.

This is the critical correction to the earlier draft. Computing lazily inside `distance_matrix`/`to_problem` would leave `TspLibData::cities()` returning placeholders (stale Qt/WASM/API maps and test helpers such as `tests/space_filling_curve_test.rs`), and `to_problem(&self)` would either double-run MDS or pair embedded matrix coordinates with placeholder `TspProblem.cities`.

**Alternative rejected:** a separate `TspProblem.layout` field or lazy per-builder embedding. Both leave `cities()` stale and require every consumer to know about a second coordinate source.

### D2: Keep two distinct flags

`TspLibData.has_real_coordinates` means "the file supplied coordinates". `DistanceSource::has_coordinates()` keeps meaning "distances are computed from these coordinates" and stays `false` for every EXPLICIT matrix. After D1 the city list is meaningful, but the matrix is still the distance source, so `two_opt`/`or_opt` keep their exact full scan.

Update the stale comments while here: `distance_matrix.rs` ("whose cities are TSPLIB grid placeholders"), `distance_source.rs`, and `docs/adr/010-distance-source.md` all describe the old placeholder model.

**Alternative rejected:** reuse `has_coordinates()` as "a layout exists". That would silently switch `two_opt`/`or_opt` from an exact full scan to an approximate candidate list.

### D3: Corrected classical MDS via power iteration

Use `f64` for all internal arithmetic; cast to `f32` only on return. The recipe is:

1. Build squared distances and double-centre: `B = -½ J D² J` (`J = I - 11ᵀ/n`). `B·1 = 0`, exactly.
2. Axis 1: start from the zero-mean trigonometric vector `cos(2πi/n)`, not all-ones. All-ones is in `span(1)`, the nullspace of `B`; on `B + cI` it is an eigenvector with eigenvalue `c` and power iteration never leaves it. Shift by `c1 = max_i Σ_j |B_ij|`, and at every iteration re-centre (`w -= mean(w)`) before normalizing. Compute `λ1 = v1ᵀ B v1`.
3. Deflate the **unshifted** matrix: `B1 = B - λ1 v1 v1ᵀ`. Deflating the shifted matrix leaves `v1` with eigenvalue `c`, so for `λ2 ≤ 0` the second iteration collapses back onto `v1`. Recompute `c2 = max_i Σ_j |B1_ij|`.
4. Axis 2: start from `sin(2πi/n)` (linearly independent of the cosine), shift by `c2`, re-centre, and Gram–Schmidt against `v1` at every iteration. Compute `λ2 = v2ᵀ B1 v2`.
5. Canonicalize both signs: pick the lowest index attaining `max_j |v_j|` within `1e-12`, and negate the vector if that component is negative.
6. Coordinates: `x_i = v1_i √max(λ1,0)`, `y_i = v2_i √max(λ2,0)`, centred. Special-case `n = 2` to the one-dimensional pair `(0,0), (d,0)`.

The distinct starts matter: with `λ1 = λ2` (square, regular polygon, `ring6_explicit`), a shared start vector for both axes has no component left for `v2` and collapses the shape to a line. The unshifted deflation matters for collinear inputs and for elongated planar sets, where shifting before deflation makes the second spectral ratio ~1 and stalls convergence.

Complexity: O(n²·iters) time, O(n²) scratch — the same order as the EXPLICIT matrix it is derived from. EXPLICIT always uses the materialized backend, so this never runs on the 100k on-the-fly path.

**Alternatives rejected:** a linear-algebra dependency for a two-vector problem; landmark MDS now (defer until an EXPLICIT instance larger than a few thousand cities appears); reusing the fragile shift/deflation from `scripts/lib/problem-minimap.mjs` without correcting it.

### D4: Lin-Kernighan builds exact matrix candidates for every EXPLICIT input

Add `CandidateList::build_from_distances(distances: &dyn DistanceSource, k)` ranking by `distance_by_pos` (ties by city ID, non-finite distances skipped with `f32::total_cmp` + a finite check) and use it in `lin_kernighan` whenever `!distances.has_coordinates()`. Keep the KD-tree path for coordinate-metric sources.

This applies to EXPLICIT instances **with** display data too (`bayg29`, `bays29`): their street distances are a better neighbourhood signal than display coordinates, so LK's tours there may improve. The byte-for-byte guarantee therefore covers coordinate-metric instances, not EXPLICIT-with-display; the benchmark re-measures those cells.

**Alternative rejected:** branch on a new `has_real_coordinates` exposed through `DistanceSource`. It is more plumbing for a case where the matrix is the better signal.

### D5: `RequireReal` fails at problem construction

Add `LayoutPolicy { Derived, RequireReal }` (default `Derived`) and additive `to_problem_with_layout` / `into_problem_with_layout`. `RequireReal` checks `has_real_coordinates` and returns a descriptive error **before** building the matrix; solvers keep returning `Solution`, not `Result`. Existing constructors mean `Derived`; the CLI/API keep running `Derived`.

### D6: Benchmark and web provenance live in both shards

`bench-matrix.sh` stamps `layout: "real" | "derived"` on each JSONL row. `publish-benchmarks.sh` treats the field as optional (old logs and `tests/e2e/publish-benchmarks.bats` must keep working), falls back to `minimapDerived` parsed from `docs/problems/*.md`, and emits `layout` on **both** `problems/{dataset}.json` and `algorithms/{solver_id}.json` under `results.{dataset}`. The algorithm shard is the one that matters: `AlgorithmBenchmarks.tsx` never reads the problem shard. The web read layer and its tests get the additive field, and `ProblemResults`/`AlgorithmBenchmarks` render the derived-layout indicator. Additive fields do not break the v2 contract, so no `/bench/v3/` bump.

## Risks / Trade-offs

- **Non-Euclidean matrices produce an approximate layout** → tours remain valid (exact scoring) but SFC/Fourier/SOM quality is bounded by layout quality; label the run and state the limitation in the algorithm docs. LK does not depend on it (D4).
- **Numerical MDS failures** → the corrected recipe (D3) plus planar, isotropic, collinear, and high-aspect-ratio tests pin the cases the old shift/all-ones recipe got wrong.
- **O(n²) scratch on parse** → only the EXPLICIT matrix path, which is already O(n²); the on-the-fly 100k path never embeds. Note the ceiling in docs.
- **`TspLibData.cities` no longer means "file coordinates"** → D2's separate flag and the ADR make the distinction explicit; `AGENTS.md` and the docs are updated.
- **EXPLICIT-with-display tours change under LK** → expected improvement, but re-measure `bayg29`/`bays29` and update `bench/solvers.json` notes if a recorded cost moves.
- **`city-id-position` overlap** → `CandidateList::build_from_distances` and the newtypes touch `candidate_list.rs`; whichever lands second rebases. This change takes ADR 012; `city-id-position` must renumber its planned ADR 011.
- **Benchmark comparability** → derived-layout rows are labelled on the algorithm shard, so a reader cannot mistake an embedded run for a surveyed one.

## Migration Plan

1. Add `embedding::mds_embed` with the corrected recipe and its tests; no caller yet.
2. Add `TspLibData.has_real_coordinates`; replace `grid_coords` with the embedding in `process_lines`; update the stale comments.
3. Wire `LayoutPolicy` and the additive constructors; correction: the parse-time layout means the builders only read `self.cities`.
4. Add the LK matrix-candidate path and correct `tests/three_opt_test.rs` to use explicit weights.
5. Add the benchmark `layout` field (both shards) and the web rendering.
6. Update algorithm docs, `AGENTS.md`, and graphify.

Rollback is a revert of the parse-time embedding: `LayoutPolicy::RequireReal` gives the old early-failure behaviour without reverting the module.

**Constitution:** §3's input contract still says "2D Euclidean coordinates (`NODE_COORD_SECTION` or `DISPLAY_DATA_SECTION`)", while the code already accepts `EDGE_WEIGHT_SECTION`, GEO, ATT, and CEIL_2D. This change widens that gap. Per §8 the amendment is a separate PR whose scope is the constitution itself; it must land **before** this change.

## Open Questions

- The exact power-iteration count and convergence tolerance are calibrated by the planar/isotropic/high-aspect tests rather than fixed here.
- Whether to align `scripts/lib/problem-minimap.mjs` with the corrected recipe or document the divergence is a follow-up; the minimap is visual and already labelled.

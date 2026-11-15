# Proposal

## Why

TSPLIB EXPLICIT instances that carry no `NODE_COORD_SECTION` or `DISPLAY_DATA_SECTION` are parsed with fabricated grid coordinates. Solvers that read city geometry (`sfc`, `fourier`, `som`) and those that build KD-tree neighbourhoods (`lk`, `branch_bound`) silently optimise those placeholders: measured against the published optimum on `gr17`, the gaps are +158.8% (sfc), +127.7% (fourier), +142.9% (som) and +97.8% (lk). Matrix-only solvers (`nn`, `savings`, `greedy_edge`, `christofides`, `2opt`, `or_opt`) are correct, so this is not a distance problem — it is a missing-layout problem. Today the only alternatives are a meaningless tour or an early error; both leave 12 of the 17 tracked EXPLICIT datasets unsolved by the geometric family.

## What Changes

- Track whether an instance supplied real coordinates (a coordinate section or display data), separately from the fact that its weights are EXPLICIT.
- At parse time, replace the fabricated `grid_coords` placeholders with a deterministic 2D layout derived from the distance matrix with classical MDS (top-2 eigenvectors via a hand-rolled power iteration). Replacing them in `TspLibData.cities` means `TspLibData::cities()`, the Qt/WASM/API parse surfaces, `DistanceSource::point_at`, and `TspProblem.cities` all expose the same layout.
- Geometric constructors (`sfc`, `fourier`, `som`) operate on the derived layout and return valid tours instead of placeholder-order tours; `branch_bound` reads the same layout.
- `lk` builds its candidate list directly from the exact distance matrix for every EXPLICIT instance (with or without display data), mirroring the existing `two_opt`/`or_opt` guard; this changes LK's ranking on `bayg29`/`bays29`, where street distances are a better neighbourhood signal than display coordinates.
- The benchmark schema records `layout: real | derived` on both the problem shard and the algorithm shard, so published numbers are labelled rather than hidden; the web result components surface the label.
- Early failure is retained only through `LayoutPolicy::RequireReal` and for a degenerate matrix that cannot be embedded, not as the default behaviour.
- **BREAKING**: none. The CLI/API output contract is unchanged, and every coordinate-metric instance (`EUC_2D`, `GEO`, `ATT`, `CEIL_2D`) keeps its current tours. EXPLICIT instances are re-benchmarked because their geometry and LK neighbourhoods change.

## Capabilities

### New Capabilities

- `coordinate-embedding`: tracks coordinate presence and derives a deterministic 2D layout from a distance matrix for coordinate-less EXPLICIT instances, including which solvers consume the layout and how derived-layout runs are labelled.

### Modified Capabilities

- `distance-source`: the EXPLICIT materialized-backend rationale no longer states that parsing supplies only placeholders, and the EXPLICIT neighbourhood scenario changes from "skip coordinate-based pruning" to exact matrix-derived candidates for every EXPLICIT input.
- `tour-construction`: the SFC solver gains defined behaviour on EXPLICIT instances without coordinates (derived layout, valid tour, documented quality bound).

## Impact

- **Library**: new `src/tsp/embedding.rs`; `tsplib.rs` (coordinate-presence flag, parse-time embedding); `lin_kernighan.rs`/`candidate_list.rs` (matrix candidate path); `branch_bound` and the geometric solvers inherit `TspLibData.cities`.
- **Surfaces**: CLI/API/WASM tour output unchanged, but the parse responses (Qt `FileLoader`, WASM `parse`, API `ParseResponse`) now expose the derived layout; `bench/solvers.json` and both published shards gain a derived-layout note; `teeline-web` result components render it.
- **Dependencies**: none new; MDS is hand-rolled and mirrors `scripts/lib/problem-minimap.mjs`.
- **Tests**: `gr17` and `ring6_explicit` (EXPLICIT, no coordinates) for the parse path and each affected solver; `bays29`'s distance matrix directly for the non-Euclidean embedding case (the fixture has display data, so the parser does not embed it); `three_opt_test.rs` is corrected to use explicit weights; regression tests for the `two_opt`/`or_opt` full-scan and for coordinate instances.
- **Docs**: algorithm pages for `sfc`/`fourier`/`som`/`lk`, AGENTS.md core types, and a separate constitution amendment for the input contract (see the design's migration plan).
- **Sequencing**: the embedding module is ID-agnostic, but `CandidateList::build_from_distances` touches the same file as the in-flight `city-id-position` refactor; whichever lands second rebases it. ADR numbering: this change takes 012; `city-id-position`'s planned ADR 011 is already taken by `011-remove-codecov.md`.

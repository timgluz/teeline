---
id: 019fe687-423a-7ac2-b95d-5ac52a5f6d9b
slug: gh-436-explicit-matrix-geo-distance-support-in-was
title: "GH #436: EXPLICIT/MATRIX + GEO distance support in WASM"
type: task
status: completed
priority: high
---

## Summary

The WASM solver currently computes Euclidean distances from city coordinates for all TSPLIB problems, even when the problem specifies `EDGE_WEIGHT_TYPE: EXPLICIT` (pre-computed matrix) or `GEO` (great-circle formula). This makes solver results, optimal comparisons, and gap percentages meaningless for these problems.

## Acceptance Criteria

- [x] WASM `parse()` returns `distance_type: "EXPLICIT"` for explicit-weight problems
- [x] WASM `parse_and_solve` / `compare` use TSPLibData for correct DistanceMatrix (instead of always EUC_2D)
- [x] New WIT export `compare-tours-from-input` for distance-type-aware comparison
- [x] New WIT export `tour-distance-from-input` for distance-type-aware tour measurement
- [x] `DistanceType::Explicit` variant in core library
- [x] `tour_cost_from_matrix` / `compare_tours_from_matrix` in comparison.rs
- [x] `tour_cost_with_type` / `compare_tours_with_type` (GEO support) in comparison.rs
- [x] Tests: Rust unit, WASM integration, Vitest web worker
- [x] Web client uses `compareToursFromInput` for EXPLICIT/GEO problems

## Implementation

- PR: #439
- Branch: feat/gh-436-explicit-geo-distances
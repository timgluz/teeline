---
id: 019e5f6f-52e3-7860-93c5-a084a5417500
slug: tasks/gh-97-distance-matrix-fix
title: "fix: DistanceMatrix correctness and panic safety (issue #97)"
type: task
status: completed
priority: high
tags: [bug, panic-safety, distance-matrix]
---

## Overview

Fix correctness bugs and panic-safety issues in `src/tsp/distance_matrix.rs` identified in issue #97.

## Goals

- Fix off-by-one bounds guard in `distance_by_pos`
- Fix `distance_between` panicking instead of returning Err
- Fix `nearest()` panicking for unknown target city
- Fix `tour_length` hot-path unwrap
- Remove -1.0 sentinel from `distances_from_index`
- Add `tour_length_by_pos` (position-space, no HashMap lookups)
- Minor API improvements: `num_cities()`, `num_distances()`, by-value params

## Acceptance Criteria

- [x] `distance_by_pos(n, 0)` returns Err (not panic)
- [x] `distance_between(unknown_id, 0)` returns Err (not panic)
- [x] `nearest()` for unknown target returns empty result (not panic)
- [x] `tour_length` with unknown city ID returns 0.0 (not panic)
- [x] `distances_from_index` -1.0 sentinel removed
- [x] `tour_length_by_pos` added
- [x] `num_cities()` and `num_distances()` added
- [x] All existing tests still pass
- [x] 6 new regression tests added and passing

## Completion Evidence

- Commit: 5e47bb1
- Tests: 209 pass, 0 failed (`cargo test`)
- `cargo clippy -- -D warnings` clean
- Solver smoke tests: bhk tiny5=4.0 ✓, bhk gr17=2085 ✓, branch_bound=4.0 ✓, nn=4.0 ✓

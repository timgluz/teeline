---
id: 019e5f37-2412-79f1-9f8b-4c8d8b9b6319
slug: tasks/gh-95-kdtree-knn-fix
title: "fix: k-NN correctness and performance in kdtree (issue #95)"
type: task
status: completed
priority: high
tags: [bug, performance, kdtree]
---

## Overview

An Opus review of `src/tsp/kdtree.rs` identified critical correctness and performance issues. This task tracks the full refactor.

`KDNode::nearest()` has two related bugs for `n > 1`:

1. **Single-point propagation**: after recursing into the close subtree the code does
   `nearest_result.add(closest_result.point, closest_result.closest_distance())` — discards all but the single closest candidate found in the subtree.

2. **Wrong pruning radius**: the far-branch guard uses `nearest_result.closest_distance()` instead of the k-th farthest distance (`search_radius()`). When #1 is close, the guard fires early and prunes branches containing the 2nd…nth neighbours.

Together these mean `nearest(pt, n)` for `n > 1` may return fewer than `n` results with wrong IDs.

## Goals

- Fix k-NN correctness for n > 1
- Eliminate per-recursion NearestResult clones (&mut accumulator)
- Add search_radius() pruning to NearestResult
- KDPoint: [f32; 2] (Copy, zero heap allocs)
- partition_points: select_nth_unstable_by O(n), no clone
- Remove dead is_empty()/size from KDNode
- Hide KDNode internals (pub(crate))
- walk() FnMut, to_vec() drops RefCell

## Acceptance Criteria

- [x] Property test: `kdtree.nearest(pt, n)` == `distance_matrix.nearest(pt, n)` for n in 1..=4
- [x] `KDNode::nearest()` returns correct top-n results (verified by property test)
- [x] `NearestResult` passed by `&mut` reference (zero per-call clones)
- [x] `search_radius()` returns INFINITY until buffer full, then farthest distance
- [x] `KDPoint` uses `[f32; 2]` (Copy)
- [x] `partition_points` uses `select_nth_unstable_by`, no redundant clone
- [x] `KDNode` internals hidden (`pub(crate)`)
- [x] `is_empty()` / redundant `size` field removed
- [x] Inline comment documents the pruning invariant

## Completion Evidence

- Commits: fix/95-kdtree-knn branch (6 commits)
- Test results: 203 tests pass, 0 failed (`cargo test`)
- `cargo clippy -- -D warnings` clean
- New regression test: `test_knn_n_gt_1_matches_oracle` — was failing, now passing

## Progress Log

### 2026-05-25
- Created task, opened feature branch fix/95-kdtree-knn
- Wrote failing regression test `test_knn_n_gt_1_matches_oracle`
- Fixed `NearestResult::add()` admission gate (use `search_radius()` not `farthest_distance()`)
- Added `NearestResult::search_radius()`: INFINITY until buffer full
- Fixed `KDNode::nearest()`: `&mut NearestResult` accumulator, `search_radius()` pruning
- Removed dead `size` field and `is_empty()` from `KDNode`
- Changed `KDPoint` to `[f32; 2]` (Copy, zero heap allocs per point)
- `partition_points` now uses `select_nth_unstable_by` (O(n) vs O(n log n))
- `walk()` accepts `FnMut`, `to_vec()` drops `RefCell`
- Visibility: `KDNode`, `KDSubTree` → `pub(crate)`; test-only methods gated with `#[cfg(test)]`
- Final clippy pass: collapsed-if, remaining copy-clone warnings

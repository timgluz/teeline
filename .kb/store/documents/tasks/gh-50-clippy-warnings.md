---
id: 019e26ee-2f93-7ee1-8d94-0cbd729cf553
slug: tasks/gh-50-clippy-warnings
title: "Fix all Clippy warnings (GH #50)"
type: task
status: completed
priority: high
tags: [clippy, ci, style]
---

## Overview

Fix all Clippy warnings to restore CI. Running `cargo clippy -- -D warnings` was failing with 83 warnings across the codebase.

Resolves GitHub issue #50.

## Goals
- Pass `cargo clippy -- -D warnings` with zero warnings
- No behaviour changes — style/idiom fixes only
- All tests pass after changes

## Acceptance Criteria
- [x] `cargo clippy -- -D warnings` exits with code 0
- [x] `cargo test` still passes (62 tests)
- [x] No semantic changes to any algorithm

## Files Affected
`src/main.rs`, `src/tsp/mod.rs`, `src/tsp/bellman_karp.rs`, `src/tsp/branch_bound.rs`, `src/tsp/distance_matrix.rs`, `src/tsp/genetic_algorithm.rs`, `src/tsp/kdtree.rs`, `src/tsp/nearest_neighbor.rs`, `src/tsp/progress.rs`, `src/tsp/route.rs`, `src/tsp/stochastic_hill.rs`, `src/tsp/tsplib.rs`, `src/tsp/two_opt.rs`

## Completion Evidence
- Commit: b18292b
- `cargo clippy -- -D warnings` → 0 errors
- `cargo test` → 62 passed, 0 failed

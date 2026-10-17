---
id: 019eb74a-50cb-7d20-9a5b-f240c8ec160f
slug: tasks/gh-45-lin-kernighan
title: "feat: implement Lin-Kernighan heuristic solver"
type: task
status: completed
priority: high
tags: [algorithm, local-search]
---

## Overview
Implements the Lin-Kernighan heuristic (Option B — idiomatic Rust) as Solvers::LinKernighan,
aliased "lk" / "lin_kernighan". Uses 2-opt with sorted candidate edge lists + double-bridge
kicks (iterated local search style). Resolves GitHub issue #45.

## Acceptance Criteria
- [ ] Solvers::LinKernighan variant added, aliased "lk" and "lin_kernighan"
- [ ] src/tsp/lin_kernighan.rs implements solver using existing DistanceMatrix + Solution
- [ ] Tours within ~1–2% of optimal on berlin52 (optimal=7542)
- [ ] Runtime on berlin52 under 1 s (release build)
- [ ] Unit tests in lin_kernighan.rs; integration tests in tests/lin_kernighan_test.rs
- [ ] Progress reporting via existing channel works
- [ ] Version bumped to 1.0.1 in Cargo.toml and src/tsp/mod.rs VERSION const
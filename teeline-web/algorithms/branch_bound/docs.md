---
name: "Branch and Bound"
solver_id: "branch_bound"
title: "Branch and Bound — Teeline"
description: "Systematic enumeration of candidate tours with branch pruning. Exact solver — do not use on more than ~20 cities."
type_badge: "Exact"
---
# Branch and Bound

| | |
|---|---|
| **Alias** | `branch_bound` |
| **Type** | Exact |
| **Complexity** | Exponential worst-case; effective pruning often makes it practical for small instances |

## Description

Systematic enumeration of candidate tours that prunes any branch whose lower-bound cost already exceeds the best complete tour found so far. Explores the search tree depth-first, backtracking whenever it can prove no improvement is possible below the current node.

**Do not use on more than ~20 cities** — worst-case complexity is factorial.

## Usage

```bash
teeline solve branch_bound -i ./data/discopt/tsp_5_1.tsp
teeline solve branch_bound -i ./data/discopt/tsp_5_1.tsp --verbose
```

## References

- [EECS 281: Backtracking and Branch & Bound (YouTube)](https://www.youtube.com/watch?v=hNs7G1b2iFY&t=5480s)
- [GeeksForGeeks article](https://www.geeksforgeeks.org/traveling-salesman-problem-using-branch-and-bound-2/)

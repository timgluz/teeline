---
id: "branch_bound"
name: "Branch and Bound"
typeBadge: "Exact"
description: "Exact depth-first enumeration of candidate tours, pruned by a minimum-spanning-tree lower bound on the unvisited cities."
hasExplainer: true
---

# Branch and Bound

<!-- The #benchmarks anchor is rendered by AlgorithmBenchmarks, not by a heading in this file. -->
<!-- markdownlint-disable MD051 -->
| | |
| --- | --- |
| **Alias** | `branch_bound` |
| **Type** | Exact |
| **Complexity** | Exponential worst-case; effective pruning often makes it practical for small instances |
| **Benchmarks** | [published results ↓](#benchmarks) |

## Description

Exact depth-first enumeration of candidate tours. At each partial tour it sorts the unvisited
cities by a lower bound — the cost so far, plus the edge to the next city, plus a minimum
spanning tree over the start city and every remaining unvisited city — and prunes any branch
whose bound cannot beat the best complete tour found so far. A nearest-neighbour tour (or the
pipeline's warm-start tour) seeds that upper bound.

Every unvisited city is always a candidate. `--n_nearest` is a heuristic option used by other
solvers and is **ignored** here: restricting the candidate set would silently turn an exact
solver into a beam search.

**Do not use on more than 24 cities.** The dispatcher rejects larger inputs with an error
rather than letting an exponential search run until the caller gives up.

```text
procedure BranchAndBound(cities):
    best ← nearest_neighbor(cities)   // or the pipeline warm-start
    dfs(path = [city0], unvisited = rest, cost = 0)
    return best

procedure dfs(path, unvisited, cost):
    if unvisited is empty:
        best ← min(best, cost + d(last, city0))
        return
    mst ← minimum_spanning_tree({city0} ∪ unvisited)
    for c in sort_by(cost + d(last, c) + mst):
        if cost + d(last, c) + mst ≥ length(best):
            break
        dfs(path + [c], unvisited \ {c}, cost + d(last, c))
```

## Usage

```bash
teeline solve branch_bound -i ./data/discopt/tsp_5_1.tsp
teeline solve branch_bound -i ./data/discopt/tsp_5_1.tsp --verbose
```

## References

- [EECS 281: Backtracking and Branch & Bound (YouTube)](https://www.youtube.com/watch?v=hNs7G1b2iFY&t=5480s)
- [GeeksForGeeks article](https://www.geeksforgeeks.org/traveling-salesman-problem-using-branch-and-bound-2/)

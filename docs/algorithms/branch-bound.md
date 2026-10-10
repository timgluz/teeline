---
id: "branch_bound"
name: "Branch and Bound"
typeBadge: "Exact"
description: "Systematic enumeration of candidate tours that prunes any branch whose lower-bound cost already exceeds the best complete tour found so far."
hasExplainer: true
---

# Branch and Bound

| | |
| --- | --- |
| **Alias** | `branch_bound` |
| **Type** | Exact — **only when `--n_nearest` ≥ the number of cities** |
| **Complexity** | Exponential worst-case; effective pruning often makes it practical for small instances |

## Description

Systematic enumeration of candidate tours that prunes any branch whose lower-bound cost already exceeds the best complete tour found so far. Explores the search tree depth-first, backtracking whenever it can prove no improvement is possible below the current node.

**Do not use on more than ~20 cities** — worst-case complexity is factorial.

## Not exact at the default settings

This solver is labelled exact, and it is — but only when the candidate set covers every city. The
default `--n_nearest 3` restricts branching to the three nearest cities when extending a partial
tour, which turns the search into a **beam search**: it can no longer reach tours whose edges leave
that candidate set, so it returns valid but suboptimal tours. Measured against three instances with
known optima:

| Instance | Optimum | `--n_nearest 3` (default) | `--n_nearest 100` |
| --- | --- | --- | --- |
| burma14 | 3323 | 3651 (+9.9 %) | 3323 |
| gr17 | 2085 | 2187 (+4.9 %) | 2085 |
| ulysses16 | 6859 | 6909 (+0.7 %) | 6859 |

With the candidate set widened to all cities, all three optima are found.

This is easy to miss from the outside: the tour is still a valid tour, the solver still reports
success, and the gap only shows up if you compare against a known optimum or against
`bellman_karp`. **If you need a proof of optimality, pass `--n_nearest` at least equal to the
number of cities.** A small value buys a large speed-up and a quietly worse answer.

The trade-off is a property of candidate-list pruning in general, not a defect unique to this
solver — but it is documented here because this is the only solver that *claims* exactness while
defaulting to a pruned search.

```text
procedure BranchAndBound(cities):
    best ← nearest_neighbor(cities)   // initial upper bound
    queue ← [partial tour starting at city 0]
    while queue not empty:
        node ← pop_most_promising(queue)
        if lower_bound(node) ≥ length(best):
            continue                   // prune branch
        if node is a complete tour:
            best ← node
        else:
            for each unvisited city c:
                queue.push(extend(node, c))
    return best
```

## Usage

```bash
teeline solve branch_bound -i ./data/discopt/tsp_5_1.tsp
teeline solve branch_bound -i ./data/discopt/tsp_5_1.tsp --verbose
```

## References

- [EECS 281: Backtracking and Branch & Bound (YouTube)](https://www.youtube.com/watch?v=hNs7G1b2iFY&t=5480s)
- [GeeksForGeeks article](https://www.geeksforgeeks.org/traveling-salesman-problem-using-branch-and-bound-2/)

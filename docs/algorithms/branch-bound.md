# Branch and Bound

| | |
| --- | --- |
| **Alias** | `branch_bound` |
| **Type** | Exact |
| **Complexity** | Exponential worst-case; effective pruning often makes it practical for small instances |

## Description

Systematic enumeration of candidate tours that prunes any branch whose lower-bound cost already exceeds the best complete tour found so far. Explores the search tree depth-first, backtracking whenever it can prove no improvement is possible below the current node.

**Do not use on more than ~20 cities** — worst-case complexity is factorial.

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

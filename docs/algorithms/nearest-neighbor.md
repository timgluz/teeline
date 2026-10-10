---
id: "nn"
name: "Nearest Neighbor"
typeBadge: "Heuristic — constructive"
description: "Greedy construction heuristic: start from an arbitrary city and repeatedly move to the closest unvisited city until all cities have been visited. Uses a KD-tree for efficient nearest-neighbor queries."
hasExplainer: true
---

# Nearest Neighbor

| | |
| --- | --- |
| **Alias** | `nn`, `nearest_neighbor` |
| **Type** | Heuristic — constructive |
| **Complexity** | O(n²) worst case |
| **Deterministic** | Yes — given the same input, always the same tour |

## Description

Greedy construction heuristic: start from an arbitrary city and repeatedly move to the closest unvisited city until all cities have been visited.

Produces a complete tour quickly. Tour quality is typically 20–25 % above optimal on benchmark instances, but it serves as an excellent warm-start seed for local-search algorithms (2-opt, 3-opt, tabu search).

```text
procedure NearestNeighbor(cities):
    unvisited ← all cities
    tour ← [first city]
    remove start from unvisited
    while unvisited is not empty:
        next ← nearest city in unvisited to tour.last
        tour.append(next)
        remove next from unvisited
    tour.append(tour[0])   // close the loop
    return tour
```

## Determinism and ties

Given the same input this solver always returns the same tour. That was not always true: the
greedy step used to ask for "the nearest unvisited city among the `n_nearest` nearest overall",
which is not the same question. When all of those were already visited, a second selection path
took over, and it resolved exactly-equidistant cities by whichever the underlying `HashSet`
happened to yield first — an order that varies between processes. On `a280`, a drilling grid with
71 exact ties across its 279 greedy steps, that produced three different tour costs (3148.11,
3168.95, 3551.81) on three invocations of the same command.

Two things fixed it: `nn` now always chooses the true nearest unvisited city, and equidistant
candidates are resolved by **lowest city id**, which is a total order rather than an artefact of
iteration order.

## `--n_nearest`

`--n_nearest` is accepted for compatibility but **does not affect `nn`**: the greedy step always
evaluates every unvisited city, because "nearest neighbour" means the nearest, full stop.

The option still governs algorithms that use it as a candidate-list limit rather than a
correctness guarantee — `lin_kernighan` narrows its 2-opt candidate edges with it, and
`branch_bound` uses it to bound which cities may extend a partial path. For those solvers a small
value is a deliberate accuracy-versus-speed trade-off, and it is documented on their own pages.

## Performance

Each greedy step examines all remaining cities, so a full tour is O(n²) distance lookups. In
practice: ~10 ms at n=52, ~20 ms at n=532, ~120 ms at n=1655, and ~19 s at n=15112. The quadratic
term only becomes visible in the thousands of cities, and `nn` remains far cheaper than any
metaheuristic at the same size.

## Usage

```bash
teeline solve nn -i ./data/tsplib/berlin52.tsp
teeline solve nn -i ./data/tsplib/berlin52.tsp --verbose
```

## References

- *Algorithms and Data Structures in Action* — Marcello La Rocca
- [Nearest neighbour algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Nearest_neighbour_algorithm)

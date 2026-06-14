---
name: "Nearest Neighbor"
solver_id: "nn"
title: "Nearest Neighbor — Teeline"
description: "Greedy construction heuristic: start from an arbitrary city and repeatedly move to the closest unvisited city. Fast and excellent as a warm-start seed for local-search solvers."
type_badge: "Heuristic · constructive"
---
# Nearest Neighbor

| | |
|---|---|
| **Alias** | `nn`, `nearest_neighbor` |
| **Type** | Heuristic — constructive |
| **Complexity** | O(n log n) with KD-tree |

## Description

Greedy construction heuristic: start from an arbitrary city and repeatedly move to the closest unvisited city until all cities have been visited. Uses a KD-tree for efficient nearest-neighbor queries.

Produces a complete tour quickly. Tour quality is typically 20–25 % above optimal on benchmark instances, but it serves as an excellent warm-start seed for local-search algorithms (2-opt, 3-opt, tabu search).

## Usage

```bash
teeline solve nn -i ./data/tsplib/berlin52.tsp
teeline solve nn -i ./data/tsplib/berlin52.tsp --verbose
```

## References

- *Algorithms and Data Structures in Action* — Marcello La Rocca
- [Nearest neighbour algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Nearest_neighbour_algorithm)

# Nearest Neighbor

| | |
| --- | --- |
| **Alias** | `nn`, `nearest_neighbor` |
| **Type** | Heuristic — constructive |
| **Complexity** | O(n log n) with KD-tree |

## Description

Greedy construction heuristic: start from an arbitrary city and repeatedly move to the closest unvisited city until all cities have been visited. Uses a KD-tree for efficient nearest-neighbor queries.

Produces a complete tour quickly. Tour quality is typically 20–25 % above optimal on benchmark instances, but it serves as an excellent warm-start seed for local-search algorithms (2-opt, 3-opt, tabu search).

```text
procedure NearestNeighbor(cities):
    unvisited ← all cities
    tour ← [random start city]
    remove start from unvisited
    while unvisited is not empty:
        next ← nearest city in unvisited to tour.last
        tour.append(next)
        remove next from unvisited
    tour.append(tour[0])   // close the loop
    return tour
```

## Usage

```bash
teeline solve nn -i ./data/tsplib/berlin52.tsp
teeline solve nn -i ./data/tsplib/berlin52.tsp --verbose
```

## References

- *Algorithms and Data Structures in Action* — Marcello La Rocca
- [Nearest neighbour algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Nearest_neighbour_algorithm)

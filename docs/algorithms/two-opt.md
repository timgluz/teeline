# 2-opt

| | |
| --- | --- |
| **Alias** | `2opt`, `two_opt` |
| **Type** | Heuristic — local search |
| **Auto-seeds from** | `nn` (nearest neighbor) |

## Description

Local search algorithm that iteratively improves a tour by removing two edges and reconnecting the resulting segments in the only other valid way (reversing the segment between the two removed edges). Repeats until no improving swap exists — a local optimum.

Auto-expands to `pipeline(nn, 2opt)`: the nearest-neighbor tour seeds the search so the local optimizer starts in a good region rather than wasting iterations escaping a random tour.

```text
procedure TwoOpt(tour):
    improved ← true
    while improved:
        improved ← false
        for each pair (i, j) with i < j:
            new_tour ← reverse_segment(tour, i, j)
            if length(new_tour) < length(tour):
                tour ← new_tour
                improved ← true
    return tour
```

## Usage

```bash
# auto-expands to pipeline(nn, 2opt)
teeline solve 2opt -i ./data/tsplib/berlin52.tsp
teeline solve two_opt -i ./data/tsplib/berlin52.tsp --verbose

# skip seeding — start from input city order
teeline solve 2opt --no-seed -i ./data/tsplib/berlin52.tsp
```

## References

- [Section 20.4: The 2-OPT Heuristic (YouTube)](https://youtu.be/dYEWqrp-mho)
- [2-opt (Wikipedia)](https://en.wikipedia.org/wiki/2-opt)

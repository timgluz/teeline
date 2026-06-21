# Lin-Kernighan ILS

| | |
| --- | --- |
| **Alias** | `lk`, `lin_kernighan` |
| **Type** | Heuristic — iterated local search |
| **Auto-seeds from** | `nn` (nearest neighbor) |

## Description

Iterated Local Search (ILS) built around a candidate-list 2-opt move with the Lin-Kernighan gain criterion. Each iteration of the inner loop scans every edge of the tour and tests replacements restricted to a pre-built candidate list of the `k` nearest neighbours. The LK gain bound short-circuits the search: when the cheapest candidate edge already costs more than the edge being removed, no profitable swap exists further down the sorted list, so the scan stops early. The inner loop repeats until no improving move remains (a local optimum is reached).

Once the inner optimizer stalls, a **double-bridge** perturbation kicks the tour out of the current basin of attraction. Double-bridge is a non-sequential 4-opt move that splits the tour at four random cut points and reconnects the four segments in a different order — the resulting tour cannot be reached by any 2-opt or 3-opt move, so it provides a genuinely different starting point for the next optimization pass. The best tour seen across all restarts is kept; a configurable plateau counter terminates early if no improvement is found for `platoo_epochs` consecutive restarts.

Auto-expands to `pipeline(nn, lk)`: the nearest-neighbour tour provides a low-cost starting point, avoiding the wasted restarts that a random seed would require.

```text
procedure LinKernighan(cities, epochs):
    tour ← nearest_neighbor(cities)
    best ← tour
    for epoch in 1..epochs:
        tour ← two_opt_with_candidate_list(tour)
        if length(tour) < length(best):
            best ← tour
        tour ← double_bridge_kick(best)   // escape local optimum
    return best
```

## Options

| Flag | Description | Default |
| ------ | ------------- | --------- |
| `--epochs` | Number of ILS restarts | 10 000 |
| `--platoo_epochs` | Stop after this many non-improving restarts | 500 |
| `--n_nearest` | Candidate list size (k nearest neighbours per city) | 3 |
| `--max_depth` | Maximum 2-opt move depth (currently unused in the inner loop) | 5 |

## Usage

```bash
# auto-expands to pipeline(nn, lk)
teeline solve lk -i ./data/tsplib/berlin52.tsp

# verbose output (prints tour distance each improvement)
teeline solve lk -i ./data/tsplib/berlin52.tsp --verbose

# skip NN seeding — start from input city order
teeline solve lk --no-seed -i ./data/tsplib/berlin52.tsp

# wider candidate list and longer run
teeline solve lk -i ./data/tsplib/berlin52.tsp --n_nearest=5 --epochs=50000
```

## Benchmark

| Instance | Optimal | This solver | Gap |
|----------|---------|-------------|-----|
| berlin52 | 7 542 | ~8 100–8 300 | ~8–9% |

The gap reflects the 2-opt depth of the inner optimizer. True LK moves use sequential edge exchanges of depth ≥ 3, which this implementation approximates with the LK gain criterion applied to 2-opt moves. A depth-3 LK pass would reduce the gap to ≈ 1–2% (tracked in GH #184).

## References

- Lin, S. & Kernighan, B. W. (1973) — "An Effective Heuristic Algorithm for the Traveling-Salesman Problem", *Operations Research*, **21**(2), 498–516. DOI: 10.1287/opre.21.2.498
  (source of the gain criterion and candidate-list restriction used in the inner optimizer)
- [4-opt — Double bridge move (Wikipedia)](https://en.wikipedia.org/wiki/4-opt#Double-bridge_move)
- [Iterated Local Search (Wikipedia)](https://en.wikipedia.org/wiki/Iterated_local_search)

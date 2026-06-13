# 3-opt

| | |
|---|---|
| **Alias** | `3opt`, `three_opt` |
| **Type** | Heuristic — local search |
| **Auto-seeds from** | `nn` (nearest neighbor) |

## Description

Extension of 2-opt that removes three edges per iteration and reconnects the three segments in the best of the eight possible reconnection configurations. Each pass tries every triple of edges, keeps the single best improvement found, and repeats until no triple yields improvement.

Finds deeper local optima than 2-opt at the cost of O(n³) per pass. Typical tour quality is a few percentage points better than `nn → 2opt` at the expense of longer runtime.

Auto-expands to `pipeline(nn, 3opt)`.

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--epochs` | Maximum passes (0 = until convergence) | 0 |

## Usage

```bash
# auto-expands to pipeline(nn, 3opt)
teeline solve 3opt -i ./data/tsplib/berlin52.tsp

# as part of the thorough preset (nn → 3opt → SA)
teeline solve thorough -i ./data/tsplib/berlin52.tsp

# skip seeding
teeline solve 3opt --no-seed -i ./data/tsplib/berlin52.tsp
```

## References

- [3-opt (Wikipedia)](https://en.wikipedia.org/wiki/3-opt)
- Lin, S. (1965) — "Computer Solutions of the Traveling Salesman Problem", *Bell System Technical Journal*, 44, 2245–2269

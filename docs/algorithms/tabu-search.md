# Tabu Search

| | |
| --- | --- |
| **Alias** | `tabu_search` |
| **Type** | Heuristic — local search |
| **Auto-seeds from** | `nn` (nearest neighbor) |

## Description

Local search with short-term memory. Maintains a *tabu list* of recently visited solutions (or recently applied moves) to prevent cycling. At each step the algorithm selects the best non-tabu neighbor of the current solution, even if that neighbor is worse — allowing escape from local optima. Entries age out of the tabu list after a fixed tenure.

Auto-expands to `pipeline(nn, tabu_search)`.

## Options

| Flag       | Description        | Default |
|------------|--------------------|---------|
| `--epochs` | Maximum iterations | —       |

## Usage

```bash
teeline solve tabu_search -i ./data/tsplib/berlin52.tsp
teeline solve tabu_search -i ./data/tsplib/berlin52.tsp --epochs=500
```

## References

- [Tabu search (Wikipedia)](https://en.wikipedia.org/wiki/Tabu_search)
- *Heuristic Search*, Chapter 14.4
- Glover, F. (1989) — "Tabu Search — Part I", *ORSA Journal on Computing*, 1(3), 190–206

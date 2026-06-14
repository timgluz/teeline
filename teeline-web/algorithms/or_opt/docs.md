---
name: "Or-opt"
solver_id: "or_opt"
title: "Or-opt — Teeline"
description: "Local search that relocates segments of 1, 2, or 3 consecutive cities to a better position in the tour. Complementary to 2-opt — finds different local optima."
type_badge: "Heuristic · local search"
---
# Or-opt

| | |
|---|---|
| **Alias** | `or_opt`, `or-opt` |
| **Type** | Heuristic — local search |
| **Auto-seeds from** | `nn` (nearest neighbor) |

## Description

Local search algorithm that relocates *segments* of 1, 2, or 3 consecutive cities to a better position elsewhere in the tour. This is a *relocation* move, as opposed to the *reversal* move used by 2-opt and 3-opt — the two strategies find different local optima and are complementary.

Each pass scans all possible relocations across all three segment sizes (Or-1, Or-2, Or-3), applies the single best improving move found, and repeats until no relocation improves the tour (best-improvement strategy). Reversed insertion is also attempted for Or-2 and Or-3 moves.

Or-opt is a restricted form of 3-opt where one of the three segments is always inserted intact (or reversed) without any further reconnection.

Auto-expands to `pipeline(nn, or_opt)`.

### Move structure

```
Or-1 — relocate a single city:
  Before: ... A → [B] → C ... X → Y ...
  After:  ... A → C ... X → [B] → Y ...

Or-2 — relocate an adjacent pair:
  Before: ... A → [B → C] → D ... X → Y ...
  After:  ... A → D ... X → [B → C] → Y ...

Or-3 — relocate a triple:
  Before: ... A → [B → C → D] → E ... X → Y ...
  After:  ... A → E ... X → [B → C → D] → Y ...
```

### When to use

Or-opt works well as a post-processing step after any constructive or metaheuristic solver. Because it catches improvements that 2-opt misses (and vice versa), combining them via the pipeline is more effective than either alone:

```bash
teeline pipeline --steps=nn,2opt,or_opt -i ./data/tsplib/berlin52.tsp
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--epochs` | Maximum passes (0 = until convergence) | 0 |

## Usage

```bash
# auto-expands to pipeline(nn, or_opt)
teeline solve or_opt -i ./data/tsplib/berlin52.tsp

# explicit alias
teeline solve or-opt -i ./data/tsplib/berlin52.tsp

# skip seeding — start from input city order
teeline solve or_opt --no-seed -i ./data/tsplib/berlin52.tsp

# combine with 2-opt for deeper local search
teeline pipeline --steps=nn,2opt,or_opt -i ./data/tsplib/berlin52.tsp
```

## References

- Or, I. (1976) — *Traveling Salesman-Type Combinatorial Problems and Their Relation to the Logistics of Regional Blood Banking*, Northwestern University dissertation
- Applegate, D. et al. (2006) — *The Traveling Salesman Problem*, Princeton University Press
- [Or-opt (Wikipedia)](https://en.wikipedia.org/wiki/Or-opt)

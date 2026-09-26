---
id: "or_opt"
name: "Or-opt"
typeBadge: "Heuristic — local search"
description: "Local search algorithm that relocates segments of 1, 2, or 3 consecutive cities to a better position elsewhere in the tour."
hasExplainer: true
---

# Or-opt

| | |
| --- | --- |
| **Alias** | `or_opt`, `or-opt` |
| **Type** | Heuristic — local search |
| **Complexity** | O(n²) / pass |
| **Auto-seeds from** | `nn` (nearest neighbor) |

## Description

Or-opt is a local search algorithm that relocates *segments* of 1, 2, or 3 consecutive cities to a better position elsewhere in the tour.

The name comes from **İlhan Or**, who introduced the move in his 1976 Northwestern dissertation — despite the coincidence, the "Or" here is a surname, not the logical *OR*.

This is a *relocation* move, as opposed to the *reversal* move used by 2-opt and 3-opt, so the two strategies find different local optima and are complementary.

Or-opt is a restricted form of 3-opt: 3-opt may cut any three edges and reconnect them freely, whereas Or-opt always removes one contiguous segment and reinserts it — intact or reversed — keeping the rest of the tour untouched. It explores a smaller, cheaper-to-evaluate neighbourhood.

A relocation only touches a handful of edges, so its cost is computed locally: the length change is `-remove_gain + insert_cost`, where `remove_gain` is the saving from splicing the segment out of its position and `insert_cost` is the extra length from reattaching it at the destination. No full-tour recomputation is needed, which keeps each pass O(n²).

Auto-expands to `pipeline(nn, or_opt)`: the nearest-neighbour tour seeds the search so the local optimizer starts in a good region.

### Move structure

Every Or-opt move cuts three edges and reconnects three: the two edges binding the segment to its original neighbours, plus the one edge at the insertion site.

```text
Or-1 — relocate a single city:
  Before:  ... A → [B] → C ...  X → Y ...
  After:   ... A → C ...  X → [B] → Y ...
  Removed: A→B, B→C, X→Y    Added: A→C, X→B, B→Y

Or-2 — relocate an adjacent pair:
  Before:  ... A → [B → C] → D ...  X → Y ...
  After:   ... A → D ...  X → [B → C] → Y ...
  Removed: A→B, C→D, X→Y    Added: A→D, X→B, C→Y

Or-3 — relocate a triple:
  Before:  ... A → [B → C → D] → E ...  X → Y ...
  After:   ... A → E ...  X → [B → C → D] → Y ...
  Removed: A→B, D→E, X→Y    Added: A→E, X→B, D→Y
```

For Or-2 and Or-3 the segment may also be inserted in reverse (swapping which end meets X and Y).

### How it works

Each pass scans every relocation across all three segment sizes, keeps track of the single best-improving move, and applies it; this repeats until no relocation shortens the tour (best-improvement strategy).

```text
procedure OrOpt(tour):
    loop:
        best_delta ← 0
        best_move ← none
        for segment_len in {1, 2, 3}:
            for each segment S of length segment_len in tour:
                for each insertion position p not adjacent to S:
                    new_tour ← remove_and_insert(tour, S, p)
                    delta ← length(new_tour) - length(tour)
                    if delta < best_delta:
                        best_delta ← delta
                        best_move ← (S, p)
        if best_move is none:
            return tour
        tour ← remove_and_insert(tour, best_move.S, best_move.p)
```

### When to use

Or-opt works well as a post-processing step after any constructive or metaheuristic solver. Because it catches improvements that 2-opt misses (and vice versa), combining them via the pipeline is more effective than either alone:

```bash
teeline pipeline --steps=nn,2opt,or_opt -i ./data/tsplib/berlin52.tsp
```

## Options

| Flag       | Description                            | Default |
|------------|----------------------------------------|---------|
| `--epochs` | Maximum passes (0 = until convergence) | 0       |

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

- Or, I. (1976) — *Traveling Salesman-Type Combinatorial Problems and Their Relation to the Logistics of Regional Blood Banking*, Ph.D. dissertation, Northwestern University, Evanston, IL (Or-opt originally proposed here)
- Golden, B. L. & Stewart, W. R. (1985) — "Empirical Analysis of Heuristics", in E. L. Lawler, J. K. Lenstra, A. H. G. Rinnooy Kan & D. B. Shmoys (Eds.), *The Traveling Salesman Problem: A Guided Tour of Combinatorial Optimization*, John Wiley & Sons, pp. 207–249 (popularized Or-opt)
- Applegate, D. et al. (2006) — *The Traveling Salesman Problem*, Princeton University Press

---
name: "Christofides"
solver_id: "christofides"
title: "Christofides — Teeline"
description: "Approximation algorithm with a provable 1.5× optimal bound for metric TSP. Builds a tour via MST, minimum-weight matching, Eulerian circuit, and shortcutting."
type_badge: "Heuristic · constructive approximation"
---
# Christofides

| | |
|---|---|
| **Alias** | `christofides`, `chr` |
| **Type** | Heuristic — constructive approximation |
| **Approximation bound** | ≤ 1.5× optimal (EUC_2D only) |

## Description

The only TSP heuristic with a **provable worst-case bound**: the output tour is always within 1.5× the optimal length, provided the distance matrix satisfies the triangle inequality (TSPLIB EUC_2D instances do; arbitrary FULL_MATRIX instances may not).

The algorithm builds a tour from scratch through six deterministic steps:

```
1. Minimum Spanning Tree (Prim's, O(n²))
2. Identify odd-degree MST vertices
3. Greedy min-weight perfect matching on odd-degree vertices
4. Overlay MST + matching → multigraph (all degrees now even)
5. Eulerian circuit (Hierholzer's algorithm)
6. Shortcut repeated cities → Hamiltonian tour
```

Because it is **constructive** (not a local search), it does not use or benefit from a seed tour. Its output is a natural warm-start for Lin-Kernighan:

```bash
teeline pipeline --steps=christofides,lk -i ./data/tsplib/berlin52.tsp
```

On berlin52 this reduces the tour from 8707 (Christofides alone) to 8156 (+8.1% vs optimal).

### Why each step matters

**Step 1 (MST):** Gives a lower bound on OPT (MST cost ≤ OPT). Acts as the tour's skeleton.

**Steps 2–3 (Matching):** The MST has some odd-degree vertices; Euler's theorem requires all vertices to have even degree. A perfect matching on odd-degree vertices fixes this while adding the minimum extra edge weight.

**Step 5 (Eulerian circuit):** The combined multigraph is guaranteed connected and Eulerian (all even degrees). Hierholzer's algorithm finds the circuit in O(edges) time.

**Step 6 (Shortcut):** Skipping already-visited cities in the Eulerian walk never increases tour length when the triangle inequality holds — this is where the 1.5× proof closes.

### When to use

- You need a **guaranteed quality bound** (not just empirical quality).
- You need a **reproducible constructive tour** to seed another solver.
- You want to study classic approximation algorithm theory in a concrete implementation.

## Options

Christofides is parameter-free. `--epochs` and other options are accepted but ignored.

## Usage

```bash
# standalone
teeline solve christofides -i ./data/tsplib/berlin52.tsp

# short alias
teeline solve chr -i ./data/tsplib/berlin52.tsp

# as warm-start for LK (recommended usage)
teeline pipeline --steps=christofides,lk -i ./data/tsplib/berlin52.tsp

# combine with Or-opt instead of LK
teeline pipeline --steps=christofides,or_opt -i ./data/tsplib/berlin52.tsp
```

## References

- Christofides, N. (1976) — "Worst-case analysis of a new heuristic for the travelling salesman problem," Carnegie Mellon University report
- Applegate, D. et al. (2006) — *The Traveling Salesman Problem*, Princeton University Press, Chapter 2
- Hierholzer, C. & Wiener, C. (1873) — "Ueber die Möglichkeit, einen Linienzug ohne Wiederholung und ohne Unterbrechung zu umfahren", *Mathematische Annalen*, 6(1), 30–32
- [Christofides algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Christofides_algorithm)

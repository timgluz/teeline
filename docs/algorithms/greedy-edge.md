---
id: "greedy_edge"
name: "Greedy Edge Construction"
typeBadge: "Heuristic — constructive"
description: "Sorts every pairwise edge shortest-first and greedily accepts each one unless it would give a city degree 3+ or close a sub-cycle before all cities are covered (Kruskal-style). Unlike nearest-neighbor's city-to-city walk, it defers hard decisions — cheap edges are grabbed regardless of tour position."
hasExplainer: true
---

# Greedy Edge Construction

| | |
| --- | --- |
| **Alias** | `greedy_edge`, `gec` |
| **Type** | Heuristic — constructive |
| **Complexity** | O(n² log n) edge sort + O(n² α(n)) scan |

## Description

Sorts every pairwise edge shortest-first and greedily accepts each one unless it
would give a city **degree 3+**, or close a **sub-cycle before all n cities are
covered** — the same Kruskal-style construction an MST uses, with two extra
constraints that force a Hamiltonian path instead of a tree.

Unlike nearest-neighbor's city-to-city walk, greedy edge construction **defers
hard decisions**: cheap edges are grabbed regardless of where they sit in the
eventual tour, so the expensive edges it's eventually forced into tend to be
smaller and more evenly distributed. The standalone tour is rarely competitive,
but as a warm-start it lands local search in a better basin than nearest-neighbor
does (see [Benchmark](#benchmark) below).

```text
1. Sorted edges           all n(n−1)/2 pairs, ascending by distance
2. Kruskal-style scan     accept edge unless:
                              · an endpoint already has degree 2, or
                              · both endpoints share a union-find component
                                AND this is not the (n−1)th accepted edge
3. Close the cycle        the n-th accepted edge rejoins the two path ends
4. Walk                   traverse the single degree-2 cycle into an ordered path
```

### Why the two constraints matter

**Degree ≤ 2:** every city in a valid tour has exactly two incident edges. Once a
city reaches degree 2 it is "full" — any further edge touching it is rejected, or
the result could not be a single cycle through every city.

**No premature sub-cycle:** the union-find structure tracks which cities are
already connected by accepted edges. Accepting an edge that joins two cities
*already in the same component* would close a loop shorter than n — a dead-end
mini-cycle. The single exception is the **closing edge** (the (n−1)th accepted,
0-indexed): at that point every city has degree 2 except the two path endpoints,
so the final same-component edge is not just allowed but *required* to turn the
path into a cycle.

This is guaranteed to terminate with exactly n accepted edges on a complete
graph: both rejection reasons are monotone (degree never decreases, union-find
components never split), so by scan end any two degree-<2 positions must already
share a component — otherwise the edge between them would have been accepted.

```text
procedure GreedyEdge(cities):
    E ← sort_all_pairs_by_distance(cities)      // ascending
    UF ← union_find(n);  degree ← [0, …, 0];  accepted ← []
    for (u, v) in E:
        if degree[u] == 2 or degree[v] == 2:
            continue                            // would exceed degree 2
        if UF.connected(u, v) and |accepted| ≠ n − 1:
            continue                            // premature sub-cycle
        UF.union(u, v);  degree[u]++;  degree[v]++;  accepted.push((u, v))
        if |accepted| == n:
            break
    tour ← walk_cycle_into_path(accepted)
    return tour
```

## Options

Greedy edge construction is **parameter-free**. `--epochs`, `--n-nearest`, and all
other `HeuristicOptions` are accepted but ignored — the same as
[`christofides`](/algorithms/christofides/). Correctness relies on scanning the
*complete* pairwise edge set: restricting to a k-nearest candidate set (as
nearest-neighbor does) would break the termination guarantee and could strand a
city with no valid partner late in the scan.

## Usage

```bash
# standalone
teeline solve greedy_edge -i ./data/tsplib/berlin52.tsp

# short alias
teeline solve gec -i ./data/tsplib/berlin52.tsp

# as warm-start for LK (recommended usage)
teeline pipeline --steps=greedy_edge,lk -i ./data/tsplib/berlin52.tsp

# combine with 2-opt instead of LK
teeline pipeline --steps=greedy_edge,2opt -i ./data/tsplib/berlin52.tsp
```

## Benchmark

On **berlin52** (52 cities; optimal tour length **7544.37**). Greedy edge
construction is a constructive heuristic, so it shines as a *seed* for local
search rather than standalone — note how a worse standalone tour still seeds LK
to a better optimum than nearest-neighbor does:

| Solver | Standalone tour | Gap vs optimal | + LK polish | Gap vs optimal |
| --- | ---: | ---: | ---: | ---: |
| `greedy_edge` | 9954.06 | +31.9% | **7544.37** | **0.0% (optimal)** |
| `nn` | 8980.92 | +19.0% | 7777.33 | +3.1% |
| `christofides` | 8707.66 | +15.4% | — | — |

The standalone gap looks poor next to nearest-neighbor, yet `greedy_edge,lk`
reaches the known optimum where `nn,lk` does not — the cheap-edges-first
construction spreads the inevitable long edges more evenly, leaving LK a smoother
descent basin. This is the typical pattern: judge a constructive heuristic by the
local-search optimum it enables, not its raw tour length.

## Relationship to other solvers

- **[`christofides`](/algorithms/christofides/)** also builds a tour from scratch,
  but via MST + matching + Eulerian shortcut, with a provable 1.5× bound. Greedy
  edge has no quality bound but is simpler and faster.
- **[`savings`](/algorithms/savings/)** (Clarke-Wright) is mechanically identical
  — it shares the same `select_edges` scan primitive — differing only in its sort
  key (Clarke-Wright *savings* value, descending, instead of raw distance,
  ascending).
- **[`nn`](/algorithms/nearest-neighbor/)** walks city-to-city, committing to each
  next step immediately; greedy edge defers, which is why its standalone tour can
  be worse yet seed local search better.

## References

- Bentley, J. L. (1990) — "Experiments on Traveling Salesman Heuristics",
  *Proceedings of the first annual ACM-SIAM symposium on Discrete algorithms
  (SODA)*, 91–99. (Classic empirical study of constructive TSP heuristics —
  nearest-neighbor and the insertion family — on large geometric instances; the
  standard benchmark context for evaluating constructive methods like greedy edge.)
- Kruskal, J. B. (1956) — "On the shortest spanning subtree of a graph and the
  traveling salesman problem", *Proceedings of the American Mathematical
  Society*, 7(1), 48–50. (The union-find MST construction this solver adapts.)
- Reinelt, G. (1991) — "TSPLIB—A Traveling Salesman Problem Library", *ORSA
  Journal on Computing*, 3(4), 376–384.
- [Travelling salesman problem — Constructive heuristics (Wikipedia)](https://en.wikipedia.org/wiki/Travelling_salesman_problem#Constructive_heuristics)

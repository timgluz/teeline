---
id: "savings"
name: "Savings"
typeBadge: "Heuristic — constructive"
description: "A savings-ordered greedy-edge construction inspired by Clarke-Wright: ranks pairwise edges by the saving of linking two cities directly instead of through a centroid-nearest hub, then greedily accepts each on the same Kruskal-style scaffolding as greedy_edge (degree ≤ 2, no premature sub-cycle)."
hasExplainer: true
---

# Savings

| | |
| --- | --- |
| **Alias** | `savings`, `sav` |
| **Type** | Heuristic — constructive |
| **Complexity** | O(n² log n) edge sort + O(n² α(n)) scan |

## Description

A constructive solver that ranks every pairwise edge by the **saving** achieved by
linking two cities directly instead of routing both through a reference *hub*, then
greedily accepts each edge on the same Kruskal-style scaffolding as
[`greedy_edge`](/algorithms/greedy_edge/) (degree ≤ 2, no premature sub-cycle).

The saving for an edge `(i, j)` relative to a hub `h` is:

```text
s(i, j) = d(h, i) + d(h, j) − d(i, j)
```

A large positive saving means `i` and `j` are close to each other but far from the
hub — exactly the edges a good tour wants. Edges are scanned **savings-descending**
(so the biggest wins are grabbed first), which is the mirror of `greedy_edge`'s
distance-ascending order.

```text
1. Hub selection           the city nearest the coordinate centroid
2. Savings ranking         all n(n−1)/2 edges, descending by s(i, j)
3. Kruskal-style scan      accept edge unless:
                              · an endpoint already has degree 2, or
                              · both endpoints share a union-find component
                                AND this is not the (n−1)th accepted edge
4. Close the cycle         the n-th accepted edge rejoins the two path ends
5. Walk                    traverse the single degree-2 cycle into an ordered path
```

### Not canonical Clarke-Wright

This implementation is **inspired by** the Clarke-Wright savings heuristic, but it is
**not** canonical Clarke-Wright route merging. Canonical CW maintains a set of routes
and repeatedly merges the two routes whose endpoint junction yields the largest
saving, removing the two hub-edge stubs in the process. This solver does none of that:

- there is no hub-edge removal step;
- there is no route-endpoint merge step;
- the hub is used **only** to compute the savings sort key.

Mechanically it is a savings-ordered greedy-edge construction. The two solvers share
the exact same `select_edges` scan primitive (`src/tsp/graph.rs`); only the sort key
differs. That shared primitive is what makes the "not canonical CW" distinction
precise — see [Relationship to other solvers](#relationship-to-other-solvers).

### Why each step matters

**Step 1 (hub selection — centroid-nearest):** the hub is the city nearest the
coordinate centroid. This gives a balanced savings distribution and is
**order-independent** — unlike a fixed city-0 hub, whose ordering would shift if the
input cities were permuted. The hub is still visited like every other city; it only
biases the *ordering* of candidate merges, not the set of cities toured.

**Step 2 (savings ranking):** ranking by saving defers hard decisions the same way
`greedy_edge`'s distance sort does — cheap junctions are grabbed first, so the
expensive edges the scan is eventually forced into tend to be smaller and more evenly
distributed.

**Step 3 (accept/reject guards):** the degree ≤ 2 and no-premature-sub-cycle
constraints (tracked by a union-find) are identical to `greedy_edge`'s and are what
guarantee the accepted edges form exactly one Hamiltonian cycle at the end.

```text
procedure Savings(cities):
    h ← city_nearest_centroid(cities)
    E ← sort_all_pairs_by_savings_descending(cities, h)
    UF ← union_find(n);  degree ← [0, …, 0];  accepted ← []
    for (u, v) in E:
        if degree[u] == 2 or degree[v] == 2:
            continue
        if UF.connected(u, v) and |accepted| ≠ n − 1:
            continue
        UF.union(u, v);  degree[u]++;  degree[v]++;  accepted.push((u, v))
        if |accepted| == n:
            break
    tour ← walk_cycle_into_path(accepted)
    return tour
```

## Options

Savings is **parameter-free**. `--epochs`, `--n-nearest`, and all other
`HeuristicOptions` are accepted but ignored — the same as
[`christofides`](/algorithms/christofides/) and
[`greedy_edge`](/algorithms/greedy_edge/). Correctness relies on scanning the
*complete* pairwise edge set: restricting to a k-nearest candidate set would break
the termination guarantee.

## Usage

```bash
# standalone
teeline solve savings -i ./data/tsplib/berlin52.tsp

# short alias
teeline solve sav -i ./data/tsplib/berlin52.tsp

# as warm-start for 2-opt
teeline pipeline --steps=savings,2opt -i ./data/tsplib/berlin52.tsp

# as warm-start for LK (recommended)
teeline pipeline --steps=savings,lk -i ./data/tsplib/berlin52.tsp
```

## Benchmark

On **berlin52** (52 cities; optimal tour length **7544.37**). Savings is a
constructive heuristic, so — like `greedy_edge` — it is best judged by the
local-search optimum it enables rather than its standalone tour length:

| Pipeline | Tour length | Gap vs optimal |
| --- | ---: | ---: |
| `savings` (standalone) | 8378.97 | +11.0% |
| `savings,2opt` | 8040.28 | +6.6% |
| `savings,lk` | **7544.37** | **0.0% (optimal)** |

For comparison, `greedy_edge` on the same instance: 9954.06 standalone (+31.9%),
also reaching 7544.37 after an LK polish. The savings sort key gives a markedly
better standalone tour than raw-distance greedy edge while seeding LK to the optimum
either way.

## When to use

- You want a **fast, deterministic, parameter-free** constructive tour.
- You need a **reproducible warm-start** for local search (2-opt / LK).
- You want a stronger standalone tour than `greedy_edge` / `nn` provide, at the same
  constructive cost.

## Relationship to other solvers

- **[`greedy_edge`](/algorithms/greedy_edge/)** is the closest sibling: both reuse
  `graph::select_edges` unchanged and differ **only** in their sort key — `savings`
  ranks edges by Clarke-Wright saving (descending), `greedy_edge` ranks by raw
  distance (ascending). The savings key produces a noticeably better standalone tour.
- **[`christofides`](/algorithms/christofides/)** also builds a tour from scratch,
  but via MST + matching + Eulerian shortcut, with a provable 1.5× bound. Savings has
  no quality bound but shares its Kruskal-style scan with `greedy_edge`.
- **[`nn`](/algorithms/nearest-neighbor/)** walks city-to-city, committing to each
  next step immediately; savings (like `greedy_edge`) defers, which is why its
  standalone tour beats NN's yet seeds local search better.

## References

- Clarke, G. & Wright, J. W. (1964) — "Scheduling of Vehicles from a Central Depot to
  a Number of Delivery Points", *Operations Research*, 12(4), 568–581.
  DOI: 10.1287/opre.12.4.568 (the savings heuristic's origin; this solver is a
  savings-ordered greedy-edge variant, not canonical Clarke-Wright route merging.)
- Kruskal, J. B. (1956) — "On the shortest spanning subtree of a graph and the
  traveling salesman problem", *Proceedings of the American Mathematical Society*,
  7(1), 48–50. (The union-find MST construction this solver's scan adapts.)
- Reinelt, G. (1991) — "TSPLIB—A Traveling Salesman Problem Library", *ORSA Journal
  on Computing*, 3(4), 376–384.
- [Savings-Algorithmus (Wikipedia, German)](https://de.wikipedia.org/wiki/Savings-Algorithmus) — dedicated article on the Clarke-Wright savings heuristic; the documented savings formula and steps match this implementation. (No dedicated English page exists.)

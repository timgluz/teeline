---
id: "insertion"
name: "Insertion"
version: "1.0.0"
typeBadge: "Heuristic — constructive"
description: "Builds a tour by repeatedly inserting a city, using nearest, farthest, or cheapest insertion; farthest is the default."
hasExplainer: false
---

# Insertion

| | |
| --- | --- |
| **Aliases** | `insertion`, `ins` |
| **Type** | Heuristic — constructive |
| **Complexity** | Nearest/farthest: O(n²) time, O(n) auxiliary space; cheapest: O(n² log n) time, O(n²) auxiliary space |
| **Benchmarks** | [measured results ↓](#benchmarks) |

## Description

Insertion builds a cycle incrementally. For a city `v` and a tour edge `(a, b)`,
the cost of inserting `v` between `a` and `b` is
`d(a, v) + d(v, b) − d(a, b)`. Each insertion replaces one edge with two, until
every city is in the tour.

The strategies differ in how they choose the next city:

- **Nearest:** choose the unvisited city whose distance to the current tour is
  smallest, then insert it at the least-cost edge.
- **Farthest:** choose the unvisited city whose distance to the current tour is
  largest, then insert it at the least-cost edge. This is the default.
- **Cheapest:** choose the city and tour edge with the smallest insertion cost.

Distance to the tour is the minimum distance to any city already in it. The
seed is a farthest pair for farthest insertion and a nearest pair for the other
strategies. Ties are resolved by city IDs. The construction is deterministic.
Problems with at most two cities are handled
directly. The solver uses the problem's distance source, including symmetric
EXPLICIT edge weights; a derived display layout does not replace the matrix as
the objective.

Nearest and farthest insertion maintain each unvisited city's nearest tour
distance, taking O(n²) time and O(n) auxiliary space. Cheapest insertion uses a
heap of insertion candidates, taking O(n² log n) time and O(n²) auxiliary
space. These space bounds exclude the problem's distance backend.

## Options

| Flag | Values | Default |
| --- | --- | --- |
| `--strategy` | `nearest`, `farthest`, `cheapest` | `farthest` |

For a TOML pipeline stage, set `strategy` in its `insertion` table:

```toml
[[stage]]
solver = "insertion"

[stage.insertion]
strategy = "nearest"
```

## Usage

```bash
teeline solve insertion -i ./data/tsplib/berlin52.tsp
teeline solve insertion --strategy nearest -i ./data/tsplib/berlin52.tsp
teeline solve insertion --strategy cheapest -i ./data/tsplib/berlin52.tsp
teeline pipeline --steps=insertion,2opt -i ./data/tsplib/berlin52.tsp
```

## Benchmarks

Five release CLI runs per strategy on each fixture produced identical routes
within each strategy. The following tour costs and gaps are measured against the
TSPLIB optima; these are fixture-based measurements, not published full-tier
results.

| Instance | Optimum | Nearest | Farthest | Cheapest |
| --- | ---: | ---: | ---: | ---: |
| berlin52 | 7542 | 9043 (+19.902%) | 8118 (+7.637%) | 8975 (+19.000%) |
| a280 | 2579 | 3253 (+26.134%) | 3002 (+16.402%) | 3145 (+21.946%) |

Tier A and the preliminary tier plan three explicitly named configurations:
`nearest`, `farthest`, and `cheapest`, each selecting the matching strategy.
To run the full Tier A sweep for insertion:

```bash
cargo build --release -p teeline-cli
scripts/bench-matrix.sh bench/matrix/tier-a.toml --solver insertion
```

The tier requires its listed TSPLIB instances; see
[`docs/benchmarks/schema.md`](../benchmarks/schema.md) for the measurement and
publication contract.

## References

- Rosenkrantz, D. J., Stearns, R. E. & Lewis, P. M. (1977), "An Analysis of
  Several Heuristics for the Traveling Salesman Problem", *SIAM Journal on
  Computing*, 6(3), 563–581. <https://doi.org/10.1137/0206041>

## Changelog

| Date | Version | Change |
| --- | --- | --- |
| 2026-11-24 | 1.0.0 | Initial documentation of all three strategies, with five-run berlin52 and a280 measurements. |

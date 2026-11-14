---
id: "space_filling_curve"
name: "Space-Filling Curve"
typeBadge: "Heuristic — constructive"
description: "Projects city coordinates onto a 2^p × 2^p grid, maps each cell to a Hilbert or Morton curve index, and sorts cities by that index. O(n log n), deterministic, and a fast geometric warm start for local search."
---

# Space-Filling Curve

| | |
| --- | --- |
| **Aliases** | `space_filling_curve`, `sfc`, `hilbert` |
| **Type** | Heuristic — constructive |
| **Complexity** | O(n log n) |
| **Benchmarks** | [published results ↓](#benchmarks) |

## Description

A space-filling curve visits every cell of a square grid exactly once while
keeping consecutive cells adjacent. Project the cities onto such a grid, read off
the curve index of each city's cell, and sorting by that index gives a tour where
spatially nearby cities tend to be close in the tour.

This is the Platzman–Bartholdi construction: no pairwise distances are computed,
so the whole solver is dominated by the sort and runs in **O(n log n)**. The
standalone tour is typically 20–50% above optimal — the price of a purely
geometric order — but it is an excellent and very cheap seed for
[`2opt`](/algorithms/two-opt/), [`or_opt`](/algorithms/or-opt/), and
[`lk`](/algorithms/lin-kernighan/).

```text
1. Bounding box        min/max of x and y, computed in f64
2. Normalize           uniform: divide both axes by max(Δx, Δy)
                       stretch_to_square: divide each axis by its own span
                       scale to [0, 2^p − 1], round, clamp
3. Orient (optional)   mirror, then rotate by 0/90/180/270
4. Index               hilbert(x, y) or morton(x, y) → u64
5. Sort                (curve index, tie key, input index)
6. Emit                city order; the closing edge is the "seam"
```

### Hilbert vs Morton

**Hilbert** indexing keeps consecutive indices grid neighbours: the curve never
jumps across the grid, so adjacent tour cities are usually adjacent cells. This
is the default and the better general-purpose choice.

**Morton** (Z-order) interleaves the bits of the coordinates. It is cheaper to
compute but has long diagonal jumps where the curve crosses quadrant boundaries,
so it produces visibly worse tours. It is useful as a contrast and as a lower
bound on what the grid ordering alone can achieve.

### The seam

Every SFC tour has one edge the curve cannot make short: the **closing edge**
from the last cell back to the first. If the curve enters at one corner and exits
at another, the seam crosses the whole map. This is the main reason SFC tours are
paired with 2-opt or LK — the local search removes the seam and any other long
jumps while keeping the good local structure.

### Ties and repeated cells

Several cities can land in the same grid cell, especially at low `--order`. The
`--tie-breaker` decides their order:

- `input_order` (default) — keep the order the cities appeared in the input.
- `lexicographic` — order by the city's original `(x, y)` coordinates, which
  distinguishes cities that share a grid cell.
- `nearest_prev` — greedily pick the bucket city closest to the last visited one.
  Buckets larger than 64 cities (heavily clustered or degenerate inputs) fall back
  to input order so the solver stays O(n log n).

## Options

| Flag | Values | Default | Notes |
| --- | --- | --- | --- |
| `--curve-type` | `hilbert`, `morton` | `hilbert` | Hilbert keeps adjacent indices adjacent on the grid. |
| `--sfc-order` | `1..=16` | `16` | Grid order `p`; the grid is `2^p × 2^p`. Indices use `u64`. |
| `--aspect-scaling` | `uniform`, `stretch_to_square` | `uniform` | `uniform` preserves aspect; `stretch_to_square` fills elongated maps. |
| `--sfc-rotation` | `0`, `90`, `180`, `270` | `0` | Moves the curve's entry/exit corners and the seam. |
| `--sfc-mirror` | flag | off | Mirrors the grid before rotating. |
| `--multi-orientation[=N]` | `0..=8` (optional value) | `0` | Best-of-N orientations; `--multi-orientation` alone means 8. Still O(n log n) per orientation. |
| `--tie-breaker` | `input_order`, `lexicographic`, `nearest_prev` | `input_order` | Ordering within a grid cell; see [Ties](#ties-and-repeated-cells). |

The same options are available in a TOML pipeline stage:

```toml
[[stage]]
solver = "sfc"

[stage.sfc]
curve_type = "morton"
order = 12
aspect_scaling = "stretch_to_square"
rotation = 90
mirror = true
multi_orientation = 8
tie_breaker = "nearest_prev"
```

## Usage

```bash
# standalone (default Hilbert, order 16)
teeline solve sfc -i ./data/tsplib/berlin52.tsp

# long alias
teeline solve space_filling_curve -i ./data/tsplib/berlin52.tsp

# Morton curve with a coarser grid
teeline solve sfc --curve-type morton --sfc-order 12 -i ./data/tsplib/berlin52.tsp

# try all eight rotations/mirrors and keep the shortest
teeline solve sfc --multi-orientation -i ./data/tsplib/berlin52.tsp

# as a warm start for local search (recommended usage)
teeline pipeline --steps=sfc,2opt -i ./data/tsplib/berlin52.tsp
teeline pipeline --steps=sfc,lk -i ./data/tsplib/berlin52.tsp
```

## Benchmarks

Measured with this repository's CLI. Published optima: **berlin52 = 7542**,
**att532 = 27686**.

| Configuration | berlin52 | Gap | att532 | Gap |
| --- | ---: | ---: | ---: | ---: |
| `sfc` (default) | 10459 | +38.7% | 41528 | +50.0% |
| `sfc --multi-orientation=8` | 9095 | +20.6% | 39498 | +42.7% |
| `sfc → 2opt` | 9819 | +30.2% | — | — |
| `sfc → lk` | **7542** | **0.0%** | — | — |

The pattern is the same as other constructive heuristics: judge the standalone
gap with suspicion and the warm-start result by what local search reaches. Here
`sfc → lk` reaches the published berlin52 optimum, while `sfc` alone is ~39%
above it.

## Relationship to other solvers

- [`nn`](/algorithms/nearest-neighbor/) also constructs from geometry, but walks
  city-to-city and commits to each next city. SFC sorts globally, so it cannot
  trap itself in a far-away corner, but it also cannot adapt to local gaps.
- [`greedy_edge`](/algorithms/greedy-edge/) sorts all `n(n−1)/2` pairwise edges
  (`O(n² log n)`); SFC needs no distance matrix at all and is much cheaper on
  large coordinate sets.
- [`fourier`](/algorithms/fourier/) is another geometric constructor, but it
  learns a closed curve with gradient descent and is orders of magnitude more
  expensive. SFC is the fast baseline.
- [`2opt`](/algorithms/two-opt/), [`or_opt`](/algorithms/or-opt/), and
  [`lk`](/algorithms/lin-kernighan/) are the natural next pipeline stages.

## References

- Platzman, L. K. & Bartholdi, J. J. (1989) — "Spacefilling Curves and the
  Planar Travelling Salesman Problem", *Journal of the ACM*, 36(4), 719–737.
  (The construction this solver implements.)
- Butz, A. R. (1971) — "Alternative Algorithm for Hilbert's Space-Filling
  Curve", *IEEE Transactions on Computers*, C-20(4), 424–426.
- Lam, W. M. & Shapiro, J. M. (1994) — "An efficient method for generating
  Hilbert curves", *Proceedings of the 1994 ACM Symposium on Applied Computing*.
- [Hilbert curve (Wikipedia)](https://en.wikipedia.org/wiki/Hilbert_curve) —
  the `xy2d`/`rot` reference used to validate the indexing.
- [Z-order curve (Wikipedia)](https://en.wikipedia.org/wiki/Z-order_curve) —
  Morton indexing by bit interleaving.

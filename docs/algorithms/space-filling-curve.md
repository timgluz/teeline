---
id: "space_filling_curve"
name: "Space-Filling Curve"
typeBadge: "Heuristic — constructive"
description: "Projects city coordinates onto a 2^p × 2^p grid, maps each cell to a Hilbert or Morton curve index, and sorts cities by that index. O(n log n), deterministic, and a fast geometric warm start for local search."
hasExplainer: true
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
geometric order. It is a very cheap seed for [`2opt`](/algorithms/two-opt/),
[`or_opt`](/algorithms/or-opt/), and [`lk`](/algorithms/lin-kernighan/); whether
it beats nearest-neighbor as a seed is instance-dependent (see
[Benchmarks](#benchmarks)).

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

Several cities can land in the same grid cell, especially at low `--sfc-order`. The
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
| `--multi-orientation[=N]` | `0..=8` (optional value) | `8` | Best-of-N orientations; `--multi-orientation` alone means 8, `--multi-orientation=0` disables. Still O(n log n) per orientation. |
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

# disable the ensemble for the single-orientation tour
teeline solve sfc --multi-orientation=0 -i ./data/tsplib/berlin52.tsp

# as a warm start for local search (recommended usage)
teeline pipeline --steps=sfc,2opt -i ./data/tsplib/berlin52.tsp
teeline pipeline --steps=sfc,lk -i ./data/tsplib/berlin52.tsp
```

## Benchmarks

Measured with this repository's release binary on TSPLIB instances with
published optima. Each figure is the median of five runs; gaps are against the
published optimum. The default now runs the 8-orientation ensemble; use
`--multi-orientation=0` for the single-orientation behaviour.

### Standalone

| Instance (optimum) | `sfc` (default) | `sfc --multi-orientation=0` | `nn` | `savings` | `greedy_edge` |
| --- | ---: | ---: | ---: | ---: | ---: |
| a280 (2,579) | 3,555 (+37.8%) | 3,819 (+48.1%) | 3,157 (+22.4%) | **2,857 (+10.8%)** | 3,111 (+20.6%) |
| att532 (27,686) | 39,498 (+42.7%) | 41,528 (+50.0%) | 35,516 (+28.3%) | **30,374 (+9.7%)** | 33,093 (+19.5%) |
| dsj1000 (18,660,188) | 26,509,724 (+42.1%) | 28,219,382 (+51.2%) | 24,631,478 (+32.0%) | **20,475,028 (+9.7%)** | 21,706,244 (+16.3%) |

### Warm start

| Instance | `nn → 2opt` | `sfc → 2opt` | `nn → lk` | `sfc → lk` | `savings → lk` |
| --- | ---: | ---: | ---: | ---: | ---: |
| a280 | 10.0% | 26.1% | 3.6% | 5.0% | **3.2%** |
| att532 | 14.2% | 20.1% | 3.7% | 6.9% | **2.8%** |
| dsj1000 | 13.4% | 19.6% | 8.2% | 10.4% | **4.3%** |

On the small preliminary instances (burma14, ulysses16/22, att48, berlin52,
eil76), the median standalone gap across the solvers is: savings 1.7%, fourier
6.8%, som 13.3%, christofides 16.2%, greedy_edge 17.6%, `sfc` (default) 17.6%,
`sfc --multi-orientation=0` 32.1%, nn 21.0%. As a warm start, `sfc → lk` ties
`nn → lk` at a 1.5% median gap, while `sfc → 2opt` trails `nn → 2opt` (14.5%
vs 11.2%). Fourier's default `k_max=4` is an outlier on a280 (+103%); see the
[Fourier page](/algorithms/fourier/) for its tuned settings.

The honest conclusion: SFC is extremely fast and deterministic, and its value is
a cheap geometric seed, not a quality winner. It reaches the berlin52 optimum
with LK and is competitive with NN on some instances, but savings is the
stronger seed on a280/att532/dsj1000. `--multi-orientation=0` is 10–14 points
worse; the ensemble is the main quality lever.

**Coordinate requirement:** SFC needs coordinates. EXPLICIT instances without
`NODE_COORD_SECTION`/`DISPLAY_DATA_SECTION` (for example `gr17`) cannot be
meaningfully solved; the same limitation affects fourier, som, and LK's
candidate lists. NN, savings, greedy-edge, christofides, and 2-opt use the
distance matrix and are unaffected. Tracked in #655.

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
- Lam, W. M. & Shapiro, J. H. (1994) — "A class of fast algorithms for the
  Peano-Hilbert space-filling curve", *Proceedings of the 1994 IEEE International
  Conference on Image Processing*, 1, 638–641.
- [Hilbert curve (Wikipedia)](https://en.wikipedia.org/wiki/Hilbert_curve) —
  the `xy2d`/`rot` reference used to validate the indexing.
- [Z-order curve (Wikipedia)](https://en.wikipedia.org/wiki/Z-order_curve) —
  Morton indexing by bit interleaving.

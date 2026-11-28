---
title: "Exact Held–Karp on 1,000 Cities: How POPMUSIC Works"
socialTitle: "Exact Held–Karp on 1,000 Cities: How POPMUSIC Works"
description: "Held–Karp is exponential and capped at 24 cities. POPMUSIC runs the same exact DP on 14-city windows, stitches the results around the tour, and improves a 1,000-city instance in half a second."
pubDate: 2026-11-28
tags: ["algorithms", "popmusic", "tsp", "decomposition"]
draft: false
---

Held–Karp is the textbook exact algorithm for the Traveling Salesman Problem. It is also the textbook example of an algorithm that stops being useful at about twenty cities. So a solver that runs Held–Karp on a 1,000-city instance in half a second sounds like a mistake.

Nothing is wrong. The trick is to run the exact DP on fourteen cities at a time.

I added POPMUSIC (*Partial Optimization Metaheuristic Under Special Intensification Conditions*) to Teeline because I wanted to see whether the exact core could earn its keep as a sub-solver. It is now available as `teeline solve popmusic`. This post explains how it works and where it stops. The full reference is on [the POPMUSIC algorithm page](/algorithms/popmusic/).

## An exact solver that refuses to stay small

Bellman–Held–Karp computes the shortest tour exactly with dynamic programming. For every subset of cities and every possible last city, it stores the cheapest path that visits exactly that subset and ends at that city. Filling the table costs `O(n² · 2ⁿ)`; reading the answer costs one more pass. At 20 cities the table is already about ten million entries. Teeline caps the exact solver at 24 cities (`EXACT_MAX_CITIES`) because at 24 cities the table already needs about 1.5 GB of `f64`s, and every extra city doubles it.

The useful question is: what is the largest problem I can hand to Held–Karp without changing its shape?

A tour is a cycle. If I cut fourteen consecutive cities (`u, x₁, …, x₁₂, v`) out of it and keep `u` and `v` fixed, reordering the twelve interior cities gives another valid tour. The rest of the cycle is untouched. The subproblem becomes a shortest Hamiltonian *path* from `u` to `v` through the interior, not a shortest *cycle*. That is a fixed-endpoint variant of Held–Karp, and it is exact.

With `R = 14`, fixing the two endpoints removes them from the subset space. The DP runs over `2¹²` interior subsets and 12 possible last cities: `2¹² × 12 = 49,152` cells. The recurrence evaluates about 135,000 transitions; the `m² · 2^m` upper bound is 590,000. That is 384 KiB. A window takes about a millisecond end to end, and the DP considers every ordering of the twelve interior cities (`12!` is 479 million) without enumerating them.

The window's distances come from a local `14 × 14` matrix, 196 `f32` values, built once per window from the problem's distance source. The diagonal is zeroed, so the build makes 182 distance queries. The DP widens each edge to `f64` when it sums. The matrix fits in L1 cache; the 49,152-cell table fits comfortably in L2. The DP's inner loop reads array entries instead of issuing dynamic distance queries. On a coordinate problem the difference is academic; on an on-the-fly backend it is the difference between a window costing 182 distance queries and about 135,000 dynamic calls in the forward pass.

## The seam problem

The obvious scan starts at the beginning: windows `0..=13`, `14..=27`, `28..=41`, and so on. Each window is improved with its endpoints pinned. But the edge between city `13` and city `14` is never inside a window. A good rearrangement might need to move a city across that seam, and no amount of reordering inside either window can do it.

One scan leaves every seam frozen. POPMUSIC uses two.

The second scan starts at offset `R / 2 = 7`: `7..=20`, `21..=34`, `35..=48`, and so on. The `13/14` seam now sits in the middle of a window, where the exact DP can move cities across it. The two scans together count as one pass. Windows wrap around the end of the tour, so the last window may cover cities near the end and the beginning at once.

```text
offset 0:   [0 ......... 13] [14 ......... 27] [28 ......... 41] ...
offset 7:          [7 ......... 20] [21 ......... 34] [35 ......... 48] ...
                            ^
                            the 13/14 seam is inside this window
```

If a pass makes no splice, the solver stops. Otherwise it runs at most four passes.

## Splicing, and why strict `<` is not enough

For each window the solver computes the current open-path cost `u → interior → v`, runs the DP, and compares. If the DP's order is cheaper, the new interior order is written back into the same slots. The endpoints never move, so the tour remains a tour.

Two details matter more than they look.

First, the DP accumulates costs in `f64`, not `f32`. Teeline stores coordinates as `f32`, but `tour_length` sums distances in `f64`; the reported cost and the optimizer must agree on what "cheaper" means. The first version of the shared DP summed in `f32`, and a code review found a 5×5 matrix where it picked a tour whose exact cost is 67,108,870 over the 67,108,866 optimum. Four units worse, silently, because at edge lengths near `2²⁴ = 16,777,216`, `f32` loses unit spacing. The fix widens each edge to `f64` inside the DP, matching `tour_length`.

Second, two different interior orders can differ by one rounding unit in the last place (ulp) even when their true costs tie. A strict `<` would let the two scan offsets trade the same splice back and forth, and the "no improvement" stop would not fire on its own. An improvement must beat the current window by more than a relative `1e-6` tolerance. That is a deliberate trade-off. On a window whose current cost is around 67 million, the threshold is about 67 units, so a genuine four-unit gain is discarded. The tolerance buys stability against rounding noise and keeps the offsets from trading the same splice. The four-pass cap, not the tolerance, is what guarantees termination; the early stop is the fast path.

Here is the guard from `src/tsp/popmusic.rs`:

```rust
let Some((candidate, order)) =
    held_karp_path_with_scratch(&scratch.matrix, interior_len, &mut scratch.dp)
else {
    return false;
};

if order.iter().enumerate().all(|(i, &local)| local == i) {
    return false;
}
let epsilon = IMPROVEMENT_EPS * current.abs().max(1.0);
if candidate >= current - epsilon {
    return false;
}
```

The identity check skips a window whose optimal order is the current one. Without it, an unchanged window would count as a splice and keep the early stop from firing.

Everything is deterministic: no random number generator, windows visited in a fixed order, ties broken by first scan order. The same input produces the same tour, every time.

## It needs a warm start

A window optimizer only sees cities that are already close along the tour. If the input is the file order, each window is an arbitrary slice of an unrelated list. The DP can reorder twelve cities inside that slice, but it cannot fix the global structure. POPMUSIC is an improvement operator, not a constructor.

So a standalone run auto-expands to `pipeline(nn, popmusic)`: nearest-neighbour builds a reasonable tour, then POPMUSIC re-optimizes its windows. In a pipeline, any upstream stage can provide the seed:

```bash
# auto-expands to pipeline(nn, popmusic)
teeline solve popmusic -i ./data/tsplib/berlin52.tsp

# a different constructive start (Hilbert curve)
teeline pipeline --steps=sfc,popmusic -i ./data/tsplib/berlin52.tsp

# clean up long-range crossings a window cannot see
teeline pipeline --steps=nn,popmusic,2opt -i ./data/tsplib/berlin52.tsp
```

On `berlin52`, the nearest-neighbour tour costs 8,980; after the window passes it costs 8,098. On `a280`, 3,157 becomes 3,064. On `dsj1000`, 24,631,468 becomes 23,391,160. SFC starts worse than NN on these instances, but its windows keep nearby cities together: after POPMUSIC it reaches 8,061 on `berlin52` and 2,877 on `a280`.

## What it cannot see

The window is fourteen cities. A mistake that spans a hundred cities (a cluster visited in the wrong order, a long jump across the map) is invisible to it. The DP can reorder only cities that already share a window. On `dsj1000` (1,000 cities), the gap stays at 25.35%.

This is the opposite trade-off from 2-opt. 2-opt can reconnect any two edges in the tour, but each move replaces only two edges. POPMUSIC rewrites every edge inside fourteen cities, but cannot move a city out of the window. They fail differently, which is why chaining them can beat either alone. These are single runs of the release CLI at the current commit; every stage is deterministic.

| Instance | `nn` | `nn + popmusic` | `nn + 2opt` | `nn + popmusic + 2opt` |
| --- | ---: | ---: | ---: | ---: |
| berlin52 | 8,980 | **8,098** | 8,193 | **8,098** |
| a280 | 3,157 | 3,064 | 2,798 | **2,782** |
| dsj1000 | 24,631,468 | 23,391,160 | 21,246,572 | **20,986,392** |

On `berlin52`, POPMUSIC beats 2-opt and 2-opt cannot improve its result. On `a280` and `dsj1000`, 2-opt removes the long-range crossings POPMUSIC cannot see, and the combination beats both.

## The numbers

Tier A, five runs per instance, `meta` family budget of 300 seconds. Six representative instances from the twelve:

| Instance   | Optimum    | POPMUSIC (median) | Gap    | Median time |
| ---------- | ---------: | ----------------: | -----: | ----------: |
| burma14    | 3,323      | 3,323             | 0.00%  | 0.022 s     |
| gr17       | 2,085      | 2,085             | 0.00%  | 0.022 s     |
| berlin52   | 7,542      | 8,098             | 7.37%  | 0.036 s     |
| kroA100    | 21,282     | 23,498            | 10.41% | 0.069 s     |
| a280       | 2,579      | 3,064             | 18.81% | 0.151 s     |
| dsj1000    | 18,660,188 | 23,391,160        | 25.35% | 0.500 s     |

All 60 runs completed, none timed out, and the worst wall time was 0.51 seconds. Three release-CLI runs on `a280` and `berlin52` produced byte-identical output. The full twelve-instance table is on [the POPMUSIC algorithm page](/algorithms/popmusic/#benchmarks).

POPMUSIC is not the strongest solver in Teeline on the larger instances. Lin–Kernighan and EAX reach the optimum on `berlin52`; POPMUSIC lands at 7.37%. Its point is different: a fast, deterministic decomposition step with exactness inside each window, and it composes with the rest of the pipeline.

## Why decomposition is worth having

Teeline already had exact solvers, constructive solvers, local search, and metaheuristics. POPMUSIC adds a different family shape: a decomposition template that calls an exact sub-solver. It is the first solver that reuses the exact core rather than adding another standalone heuristic.

That reuse is the real payoff. The fixed-endpoint DP lives in one place, `src/tsp/held_karp.rs`, and both Bellman–Held–Karp and POPMUSIC call it. The brute-force oracle that checks the DP covers the core of both solvers. When the DP moved from `f32` to `f64`, one change fixed the ranking for both. The routine is still exponential. The window keeps it bounded, which is enough to make it practical.

## Try it

```bash
teeline solve popmusic -i ./data/tsplib/berlin52.tsp
teeline pipeline --steps=nn,popmusic,2opt -i ./data/tsplib/berlin52.tsp
```

The full benchmark table is at [the POPMUSIC page](/algorithms/popmusic/).

## References

- Taillard, É. D. & Helsgaun, K. (2019). "POPMUSIC for the travelling salesman problem", *European Journal of Operational Research*, 272(2), 420–429. <https://doi.org/10.1016/j.ejor.2018.06.039>
- Taillard, É. D. (2019). "An n log n Heuristic for the TSP", *MIC 2019*. [PDF](https://arodes.hes-so.ch/nanna/record/8174/files/Published%20version.pdf)
- Taillard, É. D., Alvim, A. C. F. & Voß, S. (2015). "A template for low-complexity implementation of POPMUSIC", *MIC 2015: The XI Metaheuristics International Conference*. [PDF](https://web.archive.org/web/20240415231454/http://mistic.heig-vd.ch/taillard/articles.dir/TaillardAlvimVoss2015.pdf)
- Held, M. & Karp, R. M. (1962). "A Dynamic Programming Approach to Sequencing Problems", *Journal of the Society for Industrial and Applied Mathematics*, 10(1), 196–210. <https://doi.org/10.1137/0110015>

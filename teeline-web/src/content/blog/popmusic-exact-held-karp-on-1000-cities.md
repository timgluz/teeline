---
title: "Exact Held–Karp on 1,000 Cities: How POPMUSIC Works"
socialTitle: "Exact Held–Karp on 1,000 Cities: How POPMUSIC Works"
description: "Held–Karp is exponential and capped at 24 cities. POPMUSIC runs the same exact DP on 14-city windows, stitches the results around the tour, and improves a 1,000-city instance in half a second."
pubDate: 2026-11-28
tags: ["algorithms", "popmusic", "tsp", "decomposition"]
draft: false
---

Held–Karp is the textbook exact algorithm for the Traveling Salesman Problem. It is also the textbook example of an algorithm that stops being useful at about twenty cities. So a solver that runs Held–Karp on a 1,000-city instance in half a second sounds like a mistake.

It isn't. The trick is not to run Held–Karp on 1,000 cities. It is to run it on fourteen.

The new solver is called POPMUSIC — *Partial Optimization Metaheuristic Under Special Intensification Conditions* — and it is now available as `teeline solve popmusic`. This post explains how it turns an exponential dead end into a fast, deterministic improvement operator, and where it stops working. The full reference is on [the POPMUSIC algorithm page](/algorithms/popmusic/).

## An exact solver that refuses to stay small

Bellman–Held–Karp computes the shortest tour exactly with dynamic programming. For every subset of cities and every possible last city, it stores the cheapest path that visits exactly that subset and ends at that city. Filling the table costs `O(n² · 2ⁿ)`; reading the answer costs one more pass. At 20 cities the table is already about ten million entries. Teeline caps the exact solver at 24 cities (`EXACT_MAX_CITIES`) because beyond that the table is measured in gigabytes and the run is measured in geological time.

The useful question is not "how do I make Held–Karp handle more cities?" It is "what is the largest problem I can hand to Held–Karp without changing its shape?"

A tour is a cycle. If I cut fourteen consecutive cities out of it — `u, x₁, …, x₁₂, v` — and keep `u` and `v` fixed, then reordering the twelve interior cities gives another valid tour. The rest of the cycle is untouched. The subproblem is no longer a shortest *cycle*: it is a shortest Hamiltonian *path* from `u` to `v` through the interior. That is a fixed-endpoint variant of Held–Karp, and it is exact.

With `R = 14`, the interior contains twelve cities. The DP table is `12 · 2¹² = 49,152` cells, and the recurrence evaluates about `12² · 2¹² ≈ 590,000` transitions. That is a few hundred kilobytes and a few hundred microseconds. It considers every ordering of the twelve interior cities — `12!` is 479 million — without enumerating them.

The window's distances come from a local `14 × 14` matrix, 196 `f64` values, built once per window from the problem's distance source. The matrix fits in L1 cache; the 49,152-cell table fits comfortably in L2. The DP's inner loop reads array entries instead of issuing dynamic distance queries. On a coordinate problem the difference is academic; on an on-the-fly backend it is the difference between a window costing 196 distance computations and costing 590,000.

## The seam problem

The obvious scan starts at the beginning, optimizes cities `0..13`, then `14..27`, then `28..41`, and so on. Every window is improved with its endpoints pinned. But look at the boundary between the first two windows: the edge between city `13` and city `14` is never inside a window. A good rearrangement might need to move a city across that seam, and no amount of reordering inside either window can do it.

One scan leaves every seam frozen. POPMUSIC uses two.

The second scan starts at offset `R / 2 = 7`: `7..20`, `21..34`, `35..48`, and so on. Every seam from the first scan — including `13/14` — now sits in the middle of a window, where the exact DP can move cities across it. The two scans together count as one pass. Windows wrap around the end of the tour, so the last window may cover cities near the end and the beginning at once.

```text
offset 0:   [0 ......... 13] [14 ......... 27] [28 ......... 41] ...
offset 7:          [7 ......... 20] [21 ......... 34] [35 ......... 48] ...
                    ↑
                    the 13/14 seam is interior to this window
```

If a pass makes no splice, the solver stops. Otherwise it runs at most four passes. In practice the first pass does most of the work; the later passes polish what the shifted windows exposed.

## Splicing, and why strict `<` is not enough

For each window the solver computes the current open-path cost `u → interior → v`, runs the DP, and compares. If the DP's order is cheaper, the new interior order is written back into the same slots. The endpoints never move, so the tour remains a tour.

Two details matter more than they look.

First, the DP accumulates costs in `f64`, not `f32`. Teeline stores coordinates as `f32`, but `tour_length` sums distances in `f64`; the reported cost and the optimizer must agree on what "cheaper" means. At edge lengths near `2²⁴ = 16,777,216`, `f32` loses unit spacing, and a recurrence that rounds at each addition can rank two tours differently from the `f64` sum. There is a regression test with a small matrix where the `f32` recurrence picks a tour whose exact cost is 67,108,870 while the optimum is 67,108,866 — four units worse, silently, with no benchmark median likely to notice.

Second, two different interior orders can differ by one rounding ulp even when their true costs tie. A strict `<` would let the two scan offsets trade the same splice back and forth forever, and the "no improvement" stop would never fire. An improvement must beat the current window by more than a relative `1e-6` tolerance. That is large enough to absorb addition-order noise and small enough that real gains are not discarded.

Everything is deterministic: no random number generator, windows visited in a fixed order, ties broken by first scan order. The same input produces the same tour, every time.

## It needs a warm start

A window optimizer only sees cities that are already close along the tour. If the input is the file order, each window is an arbitrary slice of an unrelated list. The DP can reorder twelve cities inside that slice, but it cannot fix the global structure. POPMUSIC is an improvement operator, not a constructor.

So a standalone run auto-expands to `pipeline(nn, popmusic)`: nearest-neighbour builds a reasonable tour, then POPMUSIC re-optimizes its windows. `--no-seed` starts from the input order instead — useful for experiments, not for quality. In a pipeline, any upstream stage can provide the seed:

```bash
# auto-expands to pipeline(nn, popmusic)
teeline solve popmusic -i ./data/tsplib/berlin52.tsp

# a stronger constructive stage
teeline pipeline --steps=sfc,popmusic -i ./data/tsplib/berlin52.tsp

# clean up long-range crossings a window cannot see
teeline pipeline --steps=nn,popmusic,2opt -i ./data/tsplib/berlin52.tsp
```

On `berlin52`, the nearest-neighbour tour costs 8,980; after the window passes it costs 8,098. On `a280`, 3,157 becomes 3,064. On `gr17`, the window is large enough relative to the tour that POPMUSIC reaches the 2,085 optimum.

## What it cannot see

The window is fourteen cities. A mistake that spans a hundred cities — a cluster visited in the wrong order, a long jump across the map — is invisible to it. The DP can reorder only cities that already share a window. On `dsj1000` (1,000 cities), the gap stays at 25.35%; on `gr17` (17 cities), the window covers nearly the whole tour and the result is optimal.

This is the opposite trade-off from 2-opt. 2-opt looks at two edges anywhere in the tour but can make only a tiny move. POPMUSIC looks at all the edges inside fourteen cities but cannot move a city out of the window. They fail differently, which is why chaining them — `nn,popmusic,2opt` — can beat either alone.

## The numbers

Tier A, five runs per instance, `meta` family budget of 300 seconds:

| Instance   | Optimum    | POPMUSIC (median) | Gap    | Median time |
| ---------- | ---------: | ----------------: | -----: | ----------: |
| burma14    | 3,323      | 3,323             | 0.00%  | 0.022 s     |
| gr17       | 2,085      | 2,085             | 0.00%  | 0.022 s     |
| berlin52   | 7,542      | 8,098             | 7.37%  | 0.036 s     |
| kroA100    | 21,282     | 23,498            | 10.41% | 0.069 s     |
| a280       | 2,579      | 3,064             | 18.81% | 0.151 s     |
| dsj1000    | 18,660,188 | 23,391,160        | 25.35% | 0.500 s     |

All 60 runs completed, none timed out, and the worst wall time was 0.51 seconds. Three release-CLI runs on `a280` and `berlin52` produced byte-identical output.

POPMUSIC is not the strongest solver in Teeline on the larger instances. Lin–Kernighan and EAX reach the optimum on `berlin52`; POPMUSIC lands at 7.37%. Its point is different: it is a fast, deterministic decomposition step with exactness inside each window, and it composes with the rest of the pipeline.

## Why decomposition is worth having

Teeline already had exact solvers, constructive solvers, local search, and metaheuristics. POPMUSIC adds a different family shape: a decomposition template that calls an exact sub-solver. It is the first solver that reuses the exact core rather than adding another standalone heuristic.

That reuse is the real payoff. The fixed-endpoint DP now lives in one place — `src/tsp/held_karp.rs` — and both Bellman–Held–Karp and POPMUSIC call it. The brute-force oracle that verifies it also verifies both solvers. When the DP moved from `f32` to `f64`, one change fixed the ranking for both. The exponential routine did not become less exponential; it became bounded, and a bounded exponential routine is a very practical thing.

If you want to try it, start with `teeline solve popmusic` on a map you know, then chain it with 2-opt and watch which mistakes each pass catches.

## References

- Taillard, É. D. & Helsgaun, K. (2019) — "POPMUSIC for the travelling salesman problem", *European Journal of Operational Research*, 272(2), 420–429. <https://doi.org/10.1016/j.ejor.2018.06.039>
- Taillard, É. D. (2019) — "An n log n Heuristic for the TSP", *MIC 2019*. [PDF](https://arodes.hes-so.ch/nanna/record/8174/files/Published%20version.pdf)
- Taillard, É. D., Alvim, A. C. F. & Voß, S. (2015) — "A template for low-complexity implementation of POPMUSIC", *MIC 2015: The XI Metaheuristics International Conference*. [PDF](https://web.archive.org/web/20240415231454/http://mistic.heig-vd.ch/taillard/articles.dir/TaillardAlvimVoss2015.pdf)
- Held, M. & Karp, R. M. (1962) — "A Dynamic Programming Approach to Sequencing Problems", *Journal of the Society for Industrial and Applied Mathematics*, 10(1), 196–210. <https://doi.org/10.1137/0110015>

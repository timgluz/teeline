---
id: "popmusic"
name: "POPMUSIC"
version: "1.0.0"
typeBadge: "Heuristic — decomposition"
description: "Decomposition template that re-optimizes short contiguous tour windows with fixed endpoints using an exact Held–Karp path DP, splicing improvements until a pass stops improving."
hasExplainer: false
---

# POPMUSIC

<!-- The #benchmarks anchor is rendered by AlgorithmBenchmarks, not by a heading in this file. -->
<!-- markdownlint-disable MD051 -->
| | |
| --- | --- |
| **Aliases** | `popmusic`, `pop-music` |
| **Type** | Heuristic — decomposition |
| **Complexity** | O(passes · n · R · 2^R) |
| **Warm start** | Consumes the previous pipeline stage's tour; falls back to `nn` |
| **Benchmarks** | [published results ↓](#benchmarks) |

## Description

POPMUSIC (*Partial Optimization Metaheuristic Under Special Intensification Conditions*)
is a decomposition template rather than a standalone neighbourhood. It keeps the tour as a
cyclic sequence, cuts short contiguous windows out of it, re-optimizes each window exactly
while pinning its two endpoint cities, splices the improved interior back, and repeats.

The window sub-solver is the same exact fixed-endpoint Held–Karp path DP that drives the
Bellman–Held–Karp solver. At the default window `R = 14`, a window has 12 interior cities,
so the DP touches at most `12 · 2^12` states — small enough to run hundreds of times, yet
exact for the window. This is the "exact core as a sub-solver" idea: the exponential
routine is bounded to a window instead of the whole instance.

Each pass uses two scans. The first starts at offset `0` and optimizes non-overlapping
windows; the second starts at `R / 2` and optimizes the overlapping halves. A pass stops
when no window improves; the solver stops after four passes or as soon as a pass makes no
splice. Windows wrap around the end of the tour, and the last window overlaps the first.

```text
procedure POPMUSIC(tour):
    R ← min(14, n)
    for pass in 1..4:
        improved ← false
        for offset in {0, R/2}:
            for start in {offset, offset+R, …} mod n:
                window ← tour[start .. start+R-1]   (wraps)
                u, v ← window.first, window.last
                candidate ← HeldKarpPath(window[1..R-2], u, v)
                if candidate strictly shortens window and order changed:
                    splice candidate back into tour
                    improved ← true
        if not improved: break
    return tour
```

An improvement is accepted only when the candidate's open-path cost is lower by more than
a relative tolerance (`1e-6`). Two different interior orders can differ by one `f32` ulp
even when their true costs tie; without the tolerance, overlapping windows would trade
those ulps back and forth and the pass would never stop.

POPMUSIC is deterministic: no random number generator is used, windows are visited in a
fixed order, and ties resolve by first scan order. The reported cost is recomputed from the
problem's distance source. Symmetric `EXPLICIT` matrices work unchanged because the DP only
reads pairwise distances; the derived display layout is never used as the objective.

Standalone runs consume the pipeline warm start when one exists and otherwise start from a
nearest-neighbour tour. The CLI auto-expands a standalone run to
`pipeline(nn, popmusic)`; `--no-seed` starts from the input city order instead.

## Options

None. `R` and the pass cap are fixed for v1 so a benchmark run is reproducible; they are not
exposed through CLI flags, TOML, the HTTP API or WASM.

## Usage

```bash
# auto-expands to pipeline(nn, popmusic)
teeline solve popmusic -i ./data/tsplib/berlin52.tsp

# explicit alias
teeline solve pop-music -i ./data/tsplib/berlin52.tsp

# skip the automatic nearest-neighbour prefix
teeline solve popmusic --no-seed -i ./data/tsplib/berlin52.tsp

# use it after a stronger constructive stage
teeline pipeline --steps=sfc,popmusic -i ./data/tsplib/berlin52.tsp

# compose with a local search afterwards
teeline pipeline --steps=nn,popmusic,2opt -i ./data/tsplib/berlin52.tsp
```

## References

- Taillard, É. D. & Helsgaun, K. (2019) — "POPMUSIC for the travelling salesman problem", *European Journal of Operational Research*, 272(2), 420–429. <https://doi.org/10.1016/j.ejor.2018.06.039>
- Taillard, É. D. (2019) — "An n log n Heuristic for the TSP", *MIC 2019*. [PDF](https://arodes.hes-so.ch/nanna/record/8174/files/Published%20version.pdf)
- Taillard, É. D., Alvim, A. C. S. & Voß, S. (2015) — "A template for low-complexity implementation of POPMUSIC", *MIC 2015*. [PDF](http://mistic.heig-vd.ch/taillard/articles.dir/TaillardAlvimVoss2015.pdf)
- Held, M. & Karp, R. M. (1962) — "A Dynamic Programming Approach to Sequencing Problems", *Journal of the Society for Industrial and Applied Mathematics*, 10(1), 196–210 (the fixed-endpoint path DP).

## Changelog

| Date | Version | Change |
| --- | --- | --- |
| 2026-11-28 | 1.0.0 | Initial POPMUSIC decomposition solver: two-offset cyclic window scan, exact fixed-endpoint window DP, deterministic output, `nn` auto-seed and pipeline warm start. |

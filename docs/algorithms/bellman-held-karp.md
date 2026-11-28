---
id: "bhk"
name: "Bellman–Held–Karp"
version: "1.1.0"
typeBadge: "Exact"
description: "Dynamic programming algorithm that solves TSP exactly. It builds up solutions for subsets of cities, combining the optimal sub-tour results to find the globally optimal tour. Returns the provably shortest Hamiltonian circuit."
hasExplainer: true
---

# Bellman–Held–Karp

<!-- The #benchmarks anchor is rendered by AlgorithmBenchmarks, not by a heading in this file. -->
<!-- markdownlint-disable MD051 -->
| | |
| --- | --- |
| **Alias** | `bhk`, `bellman_karp` |
| **Type** | Exact |
| **Complexity** | O(2ⁿ · n²) time, O(2ⁿ · n) space |
| **Benchmarks** | [published results ↓](#benchmarks) |

## Description

Dynamic programming algorithm that solves TSP exactly. It builds up solutions for subsets of cities, combining the optimal sub-tour results to find the globally optimal tour. Returns the provably shortest Hamiltonian circuit.

**Do not use on more than ~20 cities** — memory and runtime grow exponentially with city count.
The DP table is stored in `f64` so its cost comparisons use the same accumulation as the
reported tour length; at the dispatcher's 24-city cap that is about 1.5 GB, roughly double a
32-bit table.

```text
procedure BellmanHeldKarp(cities):
    n ← |cities|
    dp[S][i] ← min cost to visit exactly the cities in S, ending at i
    dp[{0}][0] ← 0;  all other dp entries ← ∞
    for each subset S of size 2..n (in increasing order):
        for each i in S, i ≠ 0:
            for each j in S \ {i}:
                dp[S][i] ← min(dp[S][i], dp[S\{i}][j] + dist(j, i))
    return min over i≠0 of dp[{0..n-1}][i] + dist(i, 0)
```

## Usage

```bash
teeline solve bhk -i ./data/discopt/tsp_5_1.tsp
teeline solve bellman_karp -i ./data/discopt/tsp_5_1.tsp
```

## References

- [Bellman-Held-Karp walkthrough (YouTube)](https://youtu.be/D8aHqaFa8GE)
- *Algorithms Illuminated, Part 4* — Tim Roughgarden
- Held, M. & Karp, R. M. (1962) — "A Dynamic Programming Approach to Sequencing Problems", *Journal of the Society for Industrial and Applied Mathematics*, 10(1), 196–210

## Changelog

| Date | Version | Change |
| --- | --- | --- |
| 2026-11-28 | 1.1.0 | Moved the DP onto the shared Held–Karp path primitive also used by POPMUSIC. The DP table is now `f64` (about 1.5 GB at the 24-city cap) so its comparisons match the reported `f64` tour length. Tours are reported in forward DP order, equal-cost ties may resolve to a different optimal cycle, and the `--verbose` DP-table dump was removed. The optimal cost is unchanged. |

# Bellman–Held–Karp

| | |
| --- | --- |
| **Alias** | `bhk`, `bellman_karp` |
| **Type** | Exact |
| **Complexity** | O(2ⁿ · n²) time, O(2ⁿ · n) space |

## Description

Dynamic programming algorithm that solves TSP exactly. It builds up solutions for subsets of cities, combining the optimal sub-tour results to find the globally optimal tour. Returns the provably shortest Hamiltonian circuit.

**Do not use on more than ~20 cities** — memory and runtime grow exponentially with city count.

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
teeline solve bellman_karp -i ./data/discopt/tsp_5_1.tsp --verbose
```

## References

- [Bellman-Held-Karp walkthrough (YouTube)](https://youtu.be/D8aHqaFa8GE)
- *Algorithms Illuminated, Part 4* — Tim Roughgarden
- Held, M. & Karp, R. M. (1962) — "A Dynamic Programming Approach to Sequencing Problems", *Journal of the Society for Industrial and Applied Mathematics*, 10(1), 196–210

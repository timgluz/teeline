---
name: "Bellman-Held-Karp"
solver_id: "bhk"
title: "Bellman–Held–Karp — Teeline"
description: "Dynamic programming exact TSP solver. Builds optimal sub-tour solutions for all city subsets. Do not use on more than ~20 cities — memory and runtime grow exponentially."
type_badge: "Exact"
---
# Bellman–Held–Karp

| | |
|---|---|
| **Alias** | `bhk`, `bellman_karp` |
| **Type** | Exact |
| **Complexity** | O(2ⁿ · n²) time, O(2ⁿ · n) space |

## Description

Dynamic programming algorithm that solves TSP exactly. It builds up solutions for subsets of cities, combining the optimal sub-tour results to find the globally optimal tour. Returns the provably shortest Hamiltonian circuit.

**Do not use on more than ~20 cities** — memory and runtime grow exponentially with city count.

## Usage

```bash
teeline solve bhk -i ./data/discopt/tsp_5_1.tsp
teeline solve bellman_karp -i ./data/discopt/tsp_5_1.tsp --verbose
```

## References

- [Bellman-Held-Karp walkthrough (YouTube)](https://youtu.be/D8aHqaFa8GE)
- *Algorithms Illuminated, Part 4* — Tim Roughgarden
- Held, M. & Karp, R. M. (1962) — "A Dynamic Programming Approach to Sequencing Problems", *Journal of the Society for Industrial and Applied Mathematics*, 10(1), 196–210

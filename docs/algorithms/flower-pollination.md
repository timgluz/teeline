---
id: "fpa"
name: "Flower Pollination Algorithm"
version: "2.0.0"
typeBadge: "Heuristic — nature-inspired metaheuristic"
description: "Nature-inspired metaheuristic modelling pollination: flowers exchange order-crossover segments (global Lévy-scaled toward the best, local ε-scaled toward a neighbour), explore with 2-opt reversals, and finish with a 2-opt polish."
hasExplainer: true
---

# Flower Pollination Algorithm

<!-- The #benchmarks anchor is rendered by AlgorithmBenchmarks, not by a heading in this file. -->
<!-- markdownlint-disable MD051 -->
| | |
| --- | --- |
| **Alias** | `fpa`, `flower_pollination` |
| **Type** | Heuristic — nature-inspired metaheuristic |
| **Complexity** | O(epochs · pop · n) + 2-opt polish |
| **Auto-seeds from** | `shuffle` (flower 0); the rest of the population is NN-seeded internally |
| **Benchmarks** | [published results ↓](#benchmarks) |

## Description

Nature-inspired metaheuristic modelling the pollination process of flowering plants. Maintains a population of *flowers* (candidate tours). Each epoch, each flower applies either:

- **Global pollination** (probability `switch_prob`): a Lévy-flight-scaled order crossover toward the global best tour — the permutation analogue of `x + γ·L·(g* − x)`. The copied segment is `γ·|L|` of the tour, clamped.
- **Local pollination** (probability `1 − switch_prob`): a small ε-scaled order crossover toward a random neighbour, followed by a 2-opt reversal — the permutation analogue of `x + ε·(x_j − x_k)`.

The switch probability controls the balance between exploitation (global) and exploration (local). The best tour is finally polished with 2-opt.

**TSP-specific adaptations** (deviations from Yang 2012; this is a memetic adaptation, not a published algorithm):

| Adaptation | Reason |
| --- | --- |
| Global pollination → Lévy-scaled order crossover toward gbest | A rank-swap prefix breaks four edges at once and is blind to rotation; copying a segment keeps the target's internal edges |
| `γ = 0.1` on the Lévy step | The raw Lévy median saturates the segment cap, turning global moves into fixed half-best copies |
| Local pollination → ε-scaled order crossover toward a neighbour + 2-opt reversal | The `x_j − x_k` difference has no clean permutation form; borrowing a small segment from a neighbour is the edge-preserving analogue |
| ε capped at 0.3 | Uniform ε can copy the whole neighbour |
| Half the population seeded from nearest-neighbour tours | A constructive start is worth many epochs on TSP |
| Final 2-opt polish | The population explores; the polish exploits the best tour found |
| `switch_prob` floored at 0.8 when `mutation_probability < 0.01` | Prevents degeneration to 99.9 % local-only search under default CLI options |

Auto-expands to `pipeline(shuffle, fpa)`.

```text
procedure FlowerPollination(cities, n_flowers, p, epochs):
    flowers ← half_nn_seeded(half_random(n_flowers, cities))
    gbest ← best_tour(flowers)
    for epoch in 1..epochs:
        for each flower i:
            if random() < p:
                L ← |levy_flight()|
                candidate ← order_crossover(flowers[i], gbest, γ·L)
            else:
                j ← random neighbour of i
                ε ← random()
                candidate ← order_crossover(flowers[i], flowers[j], ε·0.3)
                candidate ← random_2opt_reversal(candidate)
            if length(candidate) < length(flowers[i]):
                flowers[i] ← candidate
        gbest ← best_tour(flowers)
    return 2opt_polish(gbest)
```

## Options

| Flag | Description | Default |
| ------ | ------------- | --------- |
| `--epochs` | Maximum iterations | 10 000 |
| `--n_nearest` | Number of flowers (floored at 25); also the candidate width for the final 2-opt polish (floored at 16) | 25 |
| `--mutation_probability` | Switch probability (global vs local pollination) | 0.8 |

## Usage

```bash
teeline solve fpa -i ./data/tsplib/berlin52.tsp
teeline solve flower_pollination -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve fpa -i ./data/tsplib/berlin52.tsp --n_nearest=50 --mutation_probability=0.8
```

## References

- Yang, X.-S. (2012) — "Flower Pollination Algorithm for Global Optimization", in *Unconventional Computation and Natural Computation* (UCNC 2012), LNCS 7445, pp. 240–249, Springer
- Davis, L. (1985) — "Applying Adaptive Algorithms to Epistatic Domains", *Proc. 9th International Joint Conference on Artificial Intelligence (IJCAI)*, pp. 162–163. [ijcai.org](https://ijcai.org/proceedings/1985/29)
- Croes, G. A. (1958) — "A Method for Solving Traveling-Salesman Problems", *Operations Research*, **6**(6), 791–812. DOI: 10.1287/opre.6.6.791
- [Flower pollination algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Flower_pollination_algorithm) — general overview

## Changelog

| Version | Date | Change |
| --- | --- | --- |
| 2.0.0 | 22&nbsp;Nov&nbsp;2026 | Replaced the swap-prefix moves with a Lévy-scaled order crossover toward gbest, an ε-scaled crossover toward a neighbour plus a 2-opt reversal, half-NN seeding, and a final 2-opt polish. Tier A median gap: **253.94% → 5.31%**; `dsj1000`: **2654.97% → 9.25%**. |

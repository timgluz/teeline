---
id: "pso"
name: "Particle Swarm Optimisation"
version: "2.0.0"
typeBadge: "Heuristic — swarm metaheuristic"
description: "Swarm metaheuristic where particles exchange order-crossover segments, explore with 2-opt reversals, and finish with a 2-opt polish."
hasExplainer: true
---

# Particle Swarm Optimisation

<!-- The #benchmarks anchor is rendered by AlgorithmBenchmarks, not by a heading in this file. -->
<!-- markdownlint-disable MD051 -->
| | |
| --- | --- |
| **Alias** | `pso`, `particle_swarm` |
| **Type** | Heuristic — swarm metaheuristic |
| **Complexity** | O(epochs · swarm · n) + 2-opt polish |
| **Auto-seeds from** | `shuffle` (particle 0); the rest of the swarm is NN-seeded internally |
| **Benchmarks** | [published results ↓](#benchmarks) |

## Description

Swarm metaheuristic where each particle is a candidate tour. A move copies a contiguous segment of a target tour — an order crossover — and fills the remaining positions in the particle's own order. That keeps the target's edges inside the copied segment instead of breaking them with rank transpositions. The cognitive and social moves are applied in sequence, each recomputed from the tour the previous one left behind, and a 2-opt reversal perturbation supplies exploration. The best tour the swarm finds is finally polished with 2-opt.

**TSP-specific adaptations** (deviations from Clerc 2004):

| Adaptation | Reason |
| --- | --- |
| Order-crossover move toward `pbest`/`gbest` | A rank swap can break four edges at once and is blind to tour rotation; copying a segment preserves the target's internal edges |
| 2-opt reversal perturbation, W: 0.9 → 0.4 | Explores the edge neighbourhood the tour is scored on, rather than a position-based swap distance |
| Final 2-opt polish | The swarm explores; the polish exploits the best tour it found |
| Half the swarm seeded from nearest-neighbour tours | Gives the swarm a constructive head start; the other half stays random so the population keeps its diversity |

Particle 0 keeps the pipeline seed (a shuffle when `pso` is expanded automatically); the rest of the swarm is initialised internally.

Auto-expands to `pipeline(shuffle, pso)`.

```text
procedure ParticleSwarm(cities, n_particles, epochs):
    particles ← half_nn_seeded(half_random(n_particles, cities))
    gbest ← best_tour(particles)
    for epoch in 1..epochs:
        ω ← inertia_decay(epoch)
        for each particle p:
            p.position ← order_crossover(p.position, p.best, c1 × r1)
            p.position ← order_crossover(p.position, gbest, c2 × r2)
            p.position ← random_2opt_reversals(p.position, ω)
            if length(p.position) < length(p.best):
                p.best ← p.position
        gbest ← best_tour(particles)
    return 2opt_polish(gbest)
```

## Options

| Flag          | Description                         | Default |
|---------------|-------------------------------------|---------|
| `--epochs`    | Maximum iterations                  | 10 000  |
| `--n_nearest` | Number of particles (floored at 30); also the candidate width for the final 2-opt polish (floored at 16) | 30      |

## Usage

```bash
teeline solve pso -i ./data/tsplib/berlin52.tsp
teeline solve particle_swarm -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve pso -i ./data/tsplib/berlin52.tsp --n_nearest=50
```

## References

- Kennedy, J. & Eberhart, R. (1995) — "Particle Swarm Optimization", *Proc. IEEE ICNN'95*, Vol. 4, pp. 1942–1948. DOI: 10.1109/ICNN.1995.488968
- Clerc, M. (2004) — "Discrete Particle Swarm Optimization, illustrated by the Traveling Salesman Problem", in *New Optimization Techniques in Engineering*, Springer, pp. 219–239. DOI: 10.1007/978-3-540-39930-8_8
- Davis, L. (1985) — "Applying Adaptive Algorithms to Epistatic Domains", *Proc. 9th International Joint Conference on Artificial Intelligence (IJCAI)*, pp. 162–163. [ijcai.org](https://ijcai.org/proceedings/1985/29)
- Croes, G. A. (1958) — "A Method for Solving Traveling-Salesman Problems", *Operations Research*, **6**(6), 791–812. DOI: 10.1287/opre.6.6.791
- Li, X., Tian, P., Hua, J. & Zhong, N. (2006) — "A Hybrid Discrete Particle Swarm Optimization for the Traveling Salesman Problem", *Simulated Evolution and Learning (SEAL 2006)*, LNCS 4247, 181–188. DOI: 10.1007/11903697_24
- [Particle swarm optimisation (Wikipedia)](https://en.wikipedia.org/wiki/Particle_swarm_optimization) — general overview

## Changelog

| Version | Date | Change |
| --- | --- | --- |
| 2.0.0 | 22&nbsp;Nov&nbsp;2026 | Replaced the swap-list velocity with edge-preserving order-crossover moves, a 2-opt reversal perturbation, and a final 2-opt polish. Tier A median gap: **150.35% → 6.88%**; `dsj1000`: **2340.27% → 11.17%**. |

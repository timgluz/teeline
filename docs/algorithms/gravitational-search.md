---
id: "gsa"
name: "Gravitational Search Algorithm"
version: "2.0.0"
typeBadge: "Heuristic — swarm metaheuristic"
description: "Physics-inspired swarm metaheuristic where each agent is a candidate tour with a mass proportional to its fitness. Agents are pulled toward a mass-weighted Kbest attractor by an order crossover, explore with 2-opt reversals, and finish with a 2-opt polish."
hasExplainer: true
---

# Gravitational Search Algorithm

<!-- The #benchmarks anchor is rendered by AlgorithmBenchmarks, not by a heading in this file. -->
<!-- markdownlint-disable MD051 -->
| | |
| --- | --- |
| **Alias** | `gsa`, `gravitational_search` |
| **Type** | Heuristic — swarm metaheuristic |
| **Complexity** | O(epochs · pop · n) + 2-opt polish |
| **Auto-seeds from** | `shuffle` (agent 0); the rest of the population is NN-seeded internally |
| **Benchmarks** | [published results ↓](#benchmarks) |

## Description

Physics-inspired swarm metaheuristic where each agent is a candidate tour with a mass proportional to its fitness. Heavier agents (shorter tours) attract lighter ones. Each epoch, an agent samples one attractor from the current Kbest set, weighted by mass, and moves toward it with an order crossover. Kbest shrinks linearly from N to 1 and the gravitational constant decays, shifting the algorithm from broad exploration to convergence. A 2-opt reversal supplies local exploration, and the best tour is finally polished with 2-opt.

**TSP-specific adaptations** (deviations from Rashedi et al. 2009; this is a memetic adaptation, not a published algorithm):

| Adaptation | Reason |
| --- | --- |
| Force → mass-weighted order crossover toward one sampled Kbest agent | A summed swap-list force breaks up to four edges per swap and is blind to rotation; sampling one mass-weighted attractor gives the same expected pull without stale rank lists |
| Kbest shrinks linearly N → 1 | Rashedi's exploration-to-exploitation schedule; the fixed top-half was a v1 simplification |
| Greedy accept the combined move | GSA has no per-agent memory; always-accepting an unconditional reversal drifts the whole swarm to a noise floor and freezes gbest at the best seed |
| 2-opt reversal after the attraction | Explores the edge neighbourhood the tour is scored on, and keeps a move alive after the population collapses |
| Half the population seeded from nearest-neighbour tours | A constructive start is worth many epochs on TSP |
| Final 2-opt polish | The swarm explores; the polish exploits the best tour found |

Auto-expands to `pipeline(shuffle, gsa)`.

```text
procedure GravitationalSearch(cities, n_agents, epochs):
    agents ← half_nn_seeded(half_random(n_agents, cities))
    gbest ← best_tour(agents)
    for epoch in 1..epochs:
        G ← G0 · exp(-α · epoch / epochs)
        masses ← spread_based_mass(agents)
        kbest ← top_mass(masses, kbest_size(epoch))
        for each agent i:
            j ← sample(kbest, weights=masses, exclude=i)
            candidate ← order_crossover(agents[i], agents[j], G · masses[j])
            candidate ← random_2opt_reversal(candidate)
            if length(candidate) < length(agents[i]):
                agents[i] ← candidate
        gbest ← best_tour(agents)
    return 2opt_polish(gbest)
```

## Options

| Flag          | Description                      | Default |
|---------------|----------------------------------|---------|
| `--epochs`    | Maximum iterations               | 10 000  |
| `--n_nearest` | Number of agents (floored at 25); also the candidate width for the final 2-opt polish (floored at 16) | 25      |

## Usage

```bash
teeline solve gsa -i ./data/tsplib/berlin52.tsp
teeline solve gravitational_search -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve gsa -i ./data/tsplib/berlin52.tsp --n_nearest=50
```

## References

- Rashedi, E., Nezamabadi-pour, H. & Saryazdi, S. (2009) — *GSA: A Gravitational Search Algorithm*. Information Sciences, 179(13), 2232–2248.
- Davis, L. (1985) — "Applying Adaptive Algorithms to Epistatic Domains", *Proc. 9th International Joint Conference on Artificial Intelligence (IJCAI)*, pp. 162–163. [ijcai.org](https://ijcai.org/proceedings/1985/29)
- Croes, G. A. (1958) — "A Method for Solving Traveling-Salesman Problems", *Operations Research*, **6**(6), 791–812. DOI: 10.1287/opre.6.6.791
- [Gravitational search algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Gravitational_search_algorithm) — general overview

## Changelog

| Version | Date | Change |
| --- | --- | --- |
| 2.0.0 | 22&nbsp;Nov&nbsp;2026 | Replaced the summed swap-list force with a mass-weighted order crossover toward a sampled Kbest attractor, added greedy acceptance, a 2-opt reversal, a Kbest schedule shrinking N → 1, half-NN seeding, and a final 2-opt polish. Tier A median gap: **190.52% → 2.59%**; `dsj1000`: **2503.87% → 10.13%**. |

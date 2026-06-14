---
name: "Particle Swarm"
solver_id: "pso"
title: "Particle Swarm Optimisation — Teeline"
description: "Swarm metaheuristic where each particle is a candidate tour with a velocity (ordered swap list) pulled toward personal and global bests. Velocity-capped with linearly decaying inertia."
type_badge: "Heuristic · swarm metaheuristic"
---
# Particle Swarm Optimisation

| | |
|---|---|
| **Alias** | `pso`, `particle_swarm` |
| **Type** | Heuristic — swarm metaheuristic |
| **Auto-seeds from** | `shuffle` (random swarm) |

## Description

Swarm metaheuristic where each particle is a candidate tour with a velocity — an ordered list of position swaps. Each iteration, the velocity is updated to pull the particle toward both its personal best tour and the global best tour found by any particle. The new position (tour) is obtained by applying the velocity swaps.

**TSP-specific adaptations** (deviations from Clerc 2004):

| Adaptation | Reason |
|---|---|
| Velocity cap at `⌈0.35 · n⌉` swaps | Without a cap, velocity grows to ~5n swaps per epoch, scrambling tours into noise |
| Linear inertia decay W: 0.9 → 0.4 | High inertia early for broad exploration; decays like a cooling schedule for late fine-tuning |
| All particles initialised randomly | Seeding via the pipeline; PSO itself stays a pure algorithm |

Auto-expands to `pipeline(shuffle, pso)`.

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--epochs` | Maximum iterations | 10 000 |
| `--n_nearest` | Number of particles (floored at 30) | 30 |

## Usage

```bash
teeline solve pso -i ./data/tsplib/berlin52.tsp
teeline solve particle_swarm -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve pso -i ./data/tsplib/berlin52.tsp --n_nearest=50
```

## References

- [Particle swarm optimisation (Wikipedia)](https://en.wikipedia.org/wiki/Particle_swarm_optimization)
- Kennedy, J. & Eberhart, R. (1995) — "Particle Swarm Optimization", *Proc. IEEE ICNN'95*, Vol. 4, pp. 1942–1948. DOI: 10.1109/ICNN.1995.488968
- Clerc, M. (2004) — "Discrete Particle Swarm Optimization, illustrated by the Traveling Salesman Problem", in *New Optimization Techniques in Engineering*, Springer, pp. 219–239

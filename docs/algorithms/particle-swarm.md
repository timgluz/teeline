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
- Kennedy & Eberhart (1995) — *Particle Swarm Optimization*
- Clerc, M. (2004) — *Discrete Particle Swarm Optimization, illustrated by the Traveling Salesman Problem*

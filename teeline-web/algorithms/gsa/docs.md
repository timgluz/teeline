---
name: "Gravitational Search"
solver_id: "gsa"
title: "Gravitational Search Algorithm — Teeline"
description: "Physics-inspired swarm metaheuristic where agents attract each other proportional to mass (fitness). Gravitational constant decays over time, shifting from exploration to convergence."
type_badge: "Heuristic · swarm metaheuristic"
---
# Gravitational Search Algorithm

| | |
|---|---|
| **Alias** | `gsa`, `gravitational_search` |
| **Type** | Heuristic — swarm metaheuristic |
| **Auto-seeds from** | `shuffle` (random swarm) |

## Description

Physics-inspired swarm metaheuristic where each agent is a candidate tour with a mass proportional to its fitness and a velocity (an ordered list of position swaps). Heavier agents (shorter tours) attract lighter ones; each epoch the gravitational constant decays, shifting the algorithm from broad exploration to convergence.

**TSP-specific adaptations** (deviations from Rashedi et al. 2009):

| Adaptation | Reason |
|---|---|
| Discrete velocity as swap list | TSP has no continuous position space; swap sequences from `swap_sequence(i,j)` approximate the continuous update |
| Spread-based mass: `m_i = (worst − cost_i) / Σ(worst − cost_j)` | Directly from Rashedi 2009; maps fitness to mass with guaranteed [0,1] range |
| Uniform-mass fallback when all costs equal | Prevents NaN when the population converges to identical tour costs |
| Fixed inertia `W = 0.0` | Empirically: carrying stale swap indices across epochs added noise faster than gravity overcame it; inertia disabled after tuning on berlin52 |
| Fixed Kbest = ⌈N/2⌉ | v1 simplification; v2 follow-up will decay K linearly from N → 1 (full Rashedi fidelity) |
| Velocity cap at `⌈0.35 · n⌉` swaps | Prevents tour scrambling; same cap as PSO |
| PSO-style always-accept position | Simplifies update loop; gbest tracked separately |

Auto-expands to `pipeline(shuffle, gsa)`.

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--epochs` | Maximum iterations | 10 000 |
| `--n_nearest` | Number of agents (floored at 25) | 25 |

## Usage

```bash
teeline solve gsa -i ./data/tsplib/berlin52.tsp
teeline solve gravitational_search -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve gsa -i ./data/tsplib/berlin52.tsp --n_nearest=50
```

## References

- Rashedi, E., Nezamabadi-pour, H. & Saryazdi, S. (2009) — *GSA: A Gravitational Search Algorithm*. Information Sciences, 179(13), 2232–2248.
- [Gravitational search algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Gravitational_search_algorithm)

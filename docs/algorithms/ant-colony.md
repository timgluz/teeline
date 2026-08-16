---
id: "aco"
name: "Ant Colony Optimization"
typeBadge: "Heuristic — nature-inspired metaheuristic"
description: "Nature-inspired metaheuristic (Ant System, Dorigo 1996) that models pheromone-trail foraging in ant colonies. Maintains a shared pheromone matrix across a colony of ants. Each epoch:"
hasExplainer: true
---

# Ant Colony Optimization

| | |
| --- | --- |
| **Alias** | `aco`, `ant_colony` |
| **Type** | Heuristic — nature-inspired metaheuristic |
| **Complexity** | O(epochs · ants · n²) |
| **Auto-seeds from** | `shuffle` (random ants); a warm-start tour also bootstraps the initial pheromone level (see below) |

## Description

Nature-inspired metaheuristic (Ant System, Dorigo 1996) that models pheromone-trail foraging in ant colonies. A shared *pheromone matrix* persists across epochs — unlike Cuckoo Search's or Flower Pollination's mutated population arrays, ants themselves carry no memory between epochs. Each epoch:

1. Every ant independently constructs a full tour from a random start city, choosing each next city probabilistically, weighted by `pheromone(i,j)^alpha * (1/distance(i,j))^beta` among unvisited cities.
2. After all ants finish, pheromone **evaporates** uniformly (`*= 1 - evaporation_rate`), then **every** ant deposits `1/tour_cost` along the edges of its own tour (not just the iteration-best — that would be Elitist AS or Ant Colony System, different variants).

Implements classic Ant System: fully probabilistic transition rule, single global pheromone update, no candidate-list restriction (transition probability considered over all unvisited cities each step, matching the textbook algorithm rather than a k-nearest-restricted variant).

**TSP-specific adaptations** (deviations from Dorigo 1996):

| Adaptation | Reason |
| --- | --- |
| τ0 bootstraps from a warm-start tour when available (`τ0 = num_ants / tour_length`), else a flat constant with a logged warning | Scales initial pheromone to the same order of magnitude as real deposits (`Δτ = 1/L`); a flat constant at TSPLIB-scale tour costs would be swamped by or swamp real deposits for many epochs |
| Pheromone floor after evaporation (`tau_min = tau0 * 1e-4`) | Without a floor, an un-deposited edge decays to exact `f32` `0.0` within a few hundred epochs, making it permanently unreachable regardless of heuristic distance — an MMAS-influenced one-line fix |
| Warm-start biases epoch-1 via one extra deposit pass along its edges | Since ants carry no memory between epochs, a passed-in `init_tour` would otherwise only affect the `epochs: 0` passthrough case, not actually shape search |
| Graceful degradation to eta-only (then first-candidate) selection if pheromone^alpha * eta^beta underflows/overflows to a non-finite sum | Avoids panics or NaN costs on pathological inputs (e.g. very high `beta` with coincident cities) without discarding distance information entirely |

Auto-expands to `pipeline(shuffle, aco)`.

```text
procedure AntColony(cities, num_ants, alpha, beta, evaporation_rate, epochs):
    pheromone ← initialize(tau0)
    eta ← precompute(1 / distance)^beta
    best ← best_tour_so_far
    for epoch in 1..epochs:
        for each ant:
            tour ← construct_tour(pheromone, eta, alpha)
            if length(tour) < length(best):
                best ← tour
        pheromone ← evaporate(pheromone, evaporation_rate)
        for each ant:
            deposit(pheromone, ant.tour, 1 / length(ant.tour))
    return best
```

## Options

| Flag | Description | Default |
| ------ | ------------- | --------- |
| `--epochs` | Maximum iterations | 150 |
| `--alpha` | Pheromone influence on transition probability | 1.0 |
| `--beta` | Heuristic (1/distance) influence on transition probability (validated to `[0, 6]`) | 2.0 |
| `--evaporation-rate` | Fraction of pheromone lost per epoch, in `(0, 1)` | 0.5 |
| `--num-ants` | Colony size — tours constructed per epoch | 25 |

## Usage

```bash
teeline solve aco -i ./data/tsplib/berlin52.tsp
teeline solve ant_colony -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve aco -i ./data/tsplib/berlin52.tsp --num-ants=40 --beta=3.0 --evaporation-rate=0.3
```

## References

- Dorigo, M., Maniezzo, V. & Colorni, A. (1996) — "Ant System: Optimization by a Colony of Cooperating Agents", *IEEE Transactions on Systems, Man, and Cybernetics, Part B: Cybernetics*, 26(1), pp. 29–41
- Dorigo, M. & Stützle, T. (2004) — *Ant Colony Optimization*, MIT Press, ISBN 0-262-04219-3
- Stützle, T. & Hoos, H.H. (2000) — "MAX–MIN Ant System", *Future Generation Computer Systems*, 16(8), pp. 889–914 (source of the pheromone-floor technique referenced in the adaptations table above)
- [Ant colony optimization algorithms (Wikipedia)](https://en.wikipedia.org/wiki/Ant_colony_optimization_algorithms)

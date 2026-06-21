# Genetic Algorithm

| | |
| --- | --- |
| **Alias** | `ga`, `genetic_algorithm` |
| **Type** | Heuristic — evolutionary metaheuristic |
| **Auto-seeds from** | `shuffle` (random population) |

## Description

Evolutionary metaheuristic that maintains a population of candidate tours. Each generation applies:

1. **Selection** — fitter tours (shorter distance) are more likely to reproduce.
2. **Crossover** — two parent tours are combined to produce offspring that inherit segments from each parent.
3. **Mutation** — random swap applied with probability `mutation_probability` to maintain diversity.
4. **Elitism** — the top `n_elite` individuals are passed unchanged to the next generation.

The population is seeded with a mix of NN-derived tours and random mutations (RSM mutants) for diversity. Auto-expands to `pipeline(shuffle, ga)`.

```text
procedure GeneticAlgorithm(cities, pop_size, epochs):
    population ← initialize_population(pop_size, cities)
    best ← fittest(population)
    for epoch in 1..epochs:
        parents ← tournament_selection(population)
        offspring ← order_crossover(parents)
        offspring ← mutate_each(offspring)
        population ← select_survivors(population ∪ offspring)
        if fittest(population) < best:
            best ← fittest(population)
    return best
```

## Options

| Flag | Description | Default |
| ------ | ------------- | --------- |
| `--epochs` | Maximum generations | 10 000 |
| `--mutation_probability` | Probability of random swap per child | 0.001 |
| `--n_elite` | Individuals carried unchanged to next generation | 3 |

## Usage

```bash
teeline solve ga -i ./data/tsplib/berlin52.tsp
teeline solve ga -i ./data/tsplib/berlin52.tsp --verbose
teeline solve ga -i ./data/tsplib/berlin52.tsp --epochs=500 --mutation_probability=0.2
teeline solve ga -i ./data/tsplib/berlin52.tsp --n_elite=7
```

## References

- *AIMA*, Section 4.1.4 — Genetic Algorithms
- [Genetic algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Genetic_algorithm)

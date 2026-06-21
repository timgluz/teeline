# Stochastic Hill Climbing

| | |
| --- | --- |
| **Alias** | `stochastic_hill` |
| **Type** | Heuristic — local search |
| **Auto-seeds from** | `shuffle` (random tour) |

## Description

Iterative improvement with random restarts. Each step applies a random perturbation to the current tour; the new tour is accepted if it is better. When no improvement is found for `platoo_epochs` consecutive steps, the solver restarts from a fresh random tour. The best tour seen across all restarts is returned.

Auto-expands to `pipeline(shuffle, stochastic_hill)`.

## Options

| Flag              | Description                              | Default |
|-------------------|------------------------------------------|---------|
| `--epochs`        | Maximum iterations total (0 = unlimited) | 0       |
| `--platoo_epochs` | Steps without improvement before restart | —       |

## Usage

```bash
teeline solve stochastic_hill -i ./data/tsplib/berlin52.tsp
teeline solve stochastic_hill -i ./data/tsplib/berlin52.tsp --epochs=1000
teeline solve stochastic_hill -i ./data/tsplib/berlin52.tsp --platoo_epochs=50
```

## References

- *AIMA*, Chapter 4.1 — Local Search and Optimization Problems
- [Hill climbing (Wikipedia)](https://en.wikipedia.org/wiki/Hill_climbing)

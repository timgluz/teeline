# Simulated Annealing

| | |
|---|---|
| **Alias** | `sa`, `simulated_annealing` |
| **Type** | Heuristic — local search (stochastic) |
| **Auto-seeds from** | `shuffle` (random tour) |

## Description

Probabilistic local search inspired by the annealing process in metallurgy. Each iteration proposes a random tour modification. Improvements are always accepted; worsenings are accepted with probability exp(−Δ/T), where T is the current "temperature". Temperature decreases each step according to the cooling rate, so the algorithm transitions from broad exploration at high temperature to fine-tuned exploitation at low temperature.

Auto-expands to `pipeline(shuffle, sa)`. The temperature schedule is calibrated for cold starts — seeding SA from a greedy NN tour constrains early exploration and typically worsens the final result. For a warm-started SA run use the `classic` preset (`nn → 2opt → sa`): the 2-opt stage removes edge crossings before SA fine-tunes the result.

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--max_temperature` | Starting temperature | 1000.0 |
| `--min_temperature` | Stopping temperature | 0.001 |
| `--cooling_rate` | Fractional temperature drop per step | — |
| `--epochs` | Maximum iterations | — |

## Usage

```bash
# auto-expands to pipeline(shuffle, sa)
teeline solve sa -i ./data/tsplib/berlin52.tsp
teeline solve sa -i ./data/tsplib/berlin52.tsp --verbose

# run SA from input city order (no shuffle)
teeline solve sa --no-seed -i ./data/tsplib/berlin52.tsp

# warm-start from a 2-opt-refined tour
teeline solve classic -i ./data/tsplib/berlin52.tsp

# custom temperature schedule
teeline solve sa -i ./data/tsplib/berlin52.tsp --cooling_rate=0.003 --max_temperature=500.0
```

## References

- *AIMA*, Section 4.1.2 — Simulated Annealing
- [Simulated annealing (Wikipedia)](https://en.wikipedia.org/wiki/Simulated_annealing)
- Kirkpatrick, S., Gelatt, C. D. & Vecchi, M. P. (1983) — "Optimization by Simulated Annealing", *Science*, 220(4598), 671–680. DOI: 10.1126/science.220.4598.671

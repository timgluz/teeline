# Flower Pollination Algorithm

| | |
|---|---|
| **Alias** | `fpa`, `flower_pollination` |
| **Type** | Heuristic — nature-inspired metaheuristic |
| **Auto-seeds from** | `shuffle` (random flowers) |

## Description

Nature-inspired metaheuristic modelling the pollination process of flowering plants. Maintains a population of *flowers* (candidate tours). Each epoch, each flower applies either:

- **Global pollination** (probability `switch_prob`): a Lévy-flight-scaled move toward the global best tour — analogous to `x + γ·L·(g* − x)` in the continuous version.
- **Local pollination** (probability `1 − switch_prob`): an ε-scaled displacement combining two randomly chosen flowers — analogous to `x + ε·(x_j − x_k)`.

The switch probability controls the balance between exploitation (global) and exploration (local).

**TSP-specific adaptations** (deviations from Yang 2012):

| Adaptation | Reason |
|---|---|
| Global pollination → Lévy-scaled prefix of swap sequence toward gbest | Permutation analogue of the continuous update; preserves tour validity |
| Local pollination → ε-scaled prefix of swap diff between two random flowers | Permutation analogue of local cross-pollination |
| `switch_prob` floored at 0.8 when `mutation_probability < 0.01` | Prevents degeneration to 99.9 % local-only search under default CLI options |

Auto-expands to `pipeline(shuffle, fpa)`.

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--epochs` | Maximum iterations | 10 000 |
| `--n_nearest` | Number of flowers (floored at 25) | 25 |
| `--mutation_probability` | Switch probability (global vs local pollination) | 0.8 |

## Usage

```bash
teeline solve fpa -i ./data/tsplib/berlin52.tsp
teeline solve flower_pollination -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve fpa -i ./data/tsplib/berlin52.tsp --n_nearest=50 --mutation_probability=0.8
```

## References

- Yang, X.-S. (2012) — "Flower Pollination Algorithm for Global Optimization", in *Unconventional Computation and Natural Computation* (UCNC 2012), LNCS 7445, pp. 240–249, Springer
- [Flower pollination algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Flower_pollination_algorithm)

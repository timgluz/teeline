# Cuckoo Search

| | |
|---|---|
| **Alias** | `cs`, `cuckoo_search` |
| **Type** | Heuristic — nature-inspired metaheuristic |
| **Auto-seeds from** | `shuffle` (random nests) |

## Description

Nature-inspired metaheuristic that models brood parasitism in cuckoos. Maintains a population of *nests* (candidate tours). Each epoch:

1. A new cuckoo tour is generated from a randomly selected nest via a **Lévy flight** step — a sequence of random 2-opt reversals whose count is drawn from a power-law Lévy distribution.
2. The cuckoo tour replaces a random nest if it is better.
3. Each nest is independently **abandoned** with probability `pa` and re-seeded randomly (Bernoulli nest abandonment) to maintain population diversity.

**TSP-specific adaptations** (deviations from Yang & Deb 2009):

| Adaptation | Reason |
|---|---|
| Lévy flight → k random 2-opt reversals | Maps continuous step magnitude to a discrete tour perturbation; preserves permutation validity |
| k capped at n/2 | Prevents full-tour scrambles from very large Lévy draws |
| β=1.5 fixed; σ_u≈0.6966 precomputed | Standard Lévy exponent (Mantegna 1994); constant avoids repeated gamma evaluation |
| Per-nest Bernoulli abandonment | Closer to the original paper than deterministic worst-k; avoids discarding more information per epoch than Lévy moves can recover |

Auto-expands to `pipeline(shuffle, cs)`.

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--epochs` | Maximum iterations | 10 000 |
| `--n_nearest` | Number of nests (floored at 25) | 25 |
| `--mutation_probability` | Per-nest abandonment probability `pa` | 0.001 |

## Usage

```bash
teeline solve cs -i ./data/tsplib/berlin52.tsp
teeline solve cuckoo_search -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve cs -i ./data/tsplib/berlin52.tsp --n_nearest=40 --mutation_probability=0.25
```

## References

- Yang, X.-S. & Deb, S. (2009) — "Cuckoo Search via Lévy Flights", *Proc. World Congress on Nature & Biologically Inspired Computing (NaBIC 2009)*, IEEE, pp. 210–214
- [Cuckoo search (Wikipedia)](https://en.wikipedia.org/wiki/Cuckoo_search)

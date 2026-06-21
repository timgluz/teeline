# Kohonen Self-Organizing Map

| | |
| --- | --- |
| **Alias** | `som`, `kohonen` |
| **Type** | Heuristic — constructive |
| **Complexity** | O(epochs · N · n) per run |

## Description

Uses a **1-D ring of neurons** — a neural network — that learns a topology-preserving mapping
from 2-D city space onto a cyclic ordering. Each neuron holds a position in the same plane as
the cities. The ring is initialised as a small circle near the centroid and trained so that it
wraps around all the cities; the ordering of neurons on the ring then gives the tour.

### Training

City coordinates are first normalised to the unit box `[0, 1]²` for numerical stability.
`N = n_cities × neuron_multiplier` neurons are placed uniformly on a small circle (radius 0.1)
centred at the normalised centroid.

Each epoch:

1. Anneal learning rate and neighbourhood radius:

   ```text
   η  = learning_rate × exp(−t / epochs)
   σ  = max(radius_fraction × N × exp(−t / epochs), 1.0)
   ```

2. Pick a random city `c` (with replacement).
3. Find the **Best Matching Unit** (BMU) — the neuron nearest to `c`; ties broken by lowest index.
4. Pull every neuron toward `c` weighted by a Gaussian over ring distance:

   ```text
   h(i) = exp(−ring_dist(i, bmu)² / 2σ²)
   neurons[i] += η · h(i) · (c − neurons[i])
   ```

   Neurons with `h < 0.001` are skipped for performance.

### Tour extraction

After training, each city is assigned to its closest neuron (BMU). Cities are sorted by their
neuron's ring index. Collisions (two cities mapping to the same neuron) are resolved by
distance to the neuron, with city-array index as the final tie-breaker, guaranteeing a valid
Hamiltonian tour regardless of convergence quality.

## Options

| Field | Default | Range | Description |
| ------- | --------- | ------- | ------------- |
| `epochs` | 100 000 | ≥ 1 | Number of training iterations |
| `learning_rate` | 0.8 | (0, 1] | Initial learning rate η₀ |
| `radius_fraction` | 0.1 | (0, 1] | Initial neighbourhood radius as fraction of neuron count |
| `neuron_multiplier` | 8 | ≥ 1 | Neuron count = n_cities × multiplier; higher reduces dead neurons |

## Usage

```bash
# standalone
teeline solve som -i ./data/tsplib/berlin52.tsp

# recommended: pipe into local search
teeline pipeline --steps=som,2opt -i ./data/tsplib/berlin52.tsp
teeline pipeline --steps=som,sa   -i ./data/tsplib/berlin52.tsp

# custom training schedule
teeline solve som --epochs=200000 --learning_rate=0.9 -i ./data/tsplib/berlin52.tsp
```

## Notes

- **Coordinate normalisation**: city coordinates are normalised to `[0, 1]²` internally.
  The tour is decoded by neuron ring index, not neuron position, so no denormalisation is needed.
- **`init_tour` is ignored**: as a constructive solver, SOM builds a tour from scratch and does
  not benefit from a seed tour; the `--init-tour` pipeline option has no effect.
- **Dead neurons**: neurons that never win the BMU for any city leave gaps in coverage. The
  default `neuron_multiplier = 8` (more neurons than cities) mitigates this. Increase it on
  highly clustered instances.
- **Premature freezing**: if `radius_fraction` is too small or `epochs` too few, the ring
  freezes before reaching all cities. Increase `epochs` or `radius_fraction` if the gap is
  unusually large.
- **Quality**: on berlin52, typical gap is ~5–15% standalone. Piping into 2-opt routinely
  reduces this to ~3–7%.

## References

- Kohonen, T. (1982) — "Self-organized formation of topologically correct feature maps",
  *Biological Cybernetics* **43**(1), 59–69.
- Angéniol, B., de la Croix Vaubois, G., & Le Texier, J.-Y. (1988) — "Self-organizing feature
  maps and the travelling salesman problem", *Neural Networks* **1**(4), 289–293.

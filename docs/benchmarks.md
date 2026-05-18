# Algorithm Benchmarks — berlin52

All solvers measured against the **berlin52** TSPLIB instance (52 cities, EUC\_2D).
The known optimal tour cost is **7 544.37** (from `berlin52.opt.tour`).

## Environment

| Item | Value |
|------|-------|
| CPU | AMD Ryzen 7 PRO 4750U (16 threads) |
| RAM | 32 GB |
| OS | Linux |
| Binary | `target/release/bin` (release build, `cargo build --release`) |
| Teeline version | 0.6.1 |
| Hard timeout | 3 minutes per run |
| Benchmark date | 2026-05-15 |

Each stochastic solver was run once. Results will vary across runs — treat gap values as
representative, not as a guarantee.

---

## Results

| Algorithm | Configuration | Tour cost | Gap from optimal | Wall time | CPU | Peak RSS |
|-----------|--------------|----------:|:----------------:|----------:|:---:|--------:|
| **Nearest Neighbour** | default | 8 980.92 | +19.0 % | 0.01 s | 50 % | 7.6 MB |
| **2-opt** | default | 9 368.32 | +24.2 % | 0.01 s | 63 % | 7.4 MB |
| **3-opt** | default | 7 742.65 | +2.6 % | 0.3 s | 55 % | 7.4 MB |
| **Stochastic Hill** | `--epochs=10000` | 8 385.20 | +11.2 % | 0.02 s | 88 % | 7.6 MB |
| **Stochastic Hill** | `--epochs=100000` | 9 255.82 | +22.7 % | 0.18 s | 98 % | 7.4 MB |
| **Simulated Annealing** | default | 8 059.29 | +6.8 % | 0.34 s | 99 % | 8.1 MB |
| **Simulated Annealing** | `--epochs=100000` | 8 275.12 | +9.7 % | 0.31 s | 99 % | 8.0 MB |
| **Tabu Search** | `--epochs=1000` | 9 337.22 | +23.8 % | 0.04 s | 95 % | 7.6 MB |
| **Tabu Search** | `--epochs=10000` | 9 270.00 | +22.9 % | 0.47 s | 99 % | 7.6 MB |
| **Genetic Algorithm** | `--epochs=500` | 13 294.30 | +76.2 % | 0.08 s | 98 % | 7.6 MB |
| **Genetic Algorithm** | `--epochs=10000` (default) | 8 172.11 | +8.3 % | 1.63 s | 100 % | 7.7 MB |
| **Particle Swarm (PSO)** | `--epochs=10000` (default) | 8 874.46 | +17.6 % | 0.84 s | 100 % | 7.9 MB |
| **Particle Swarm (PSO)** | `--epochs=10000 --n_nearest=50` | 8 663.69 | +14.8 % | 1.42 s | 100 % | 8.1 MB |
| **Cuckoo Search** | default (`--epochs=10000 --n_nearest=25`) | 7 877.84 | +4.4 % | 0.72 s | 100 % | 7.9 MB |
| **Flower Pollination (FPA)** | default (`--epochs=10000 --n_nearest=25`) | 8 867.93 | +17.5 % | 0.53 s | 100 % | 7.2 MB |
| **Flower Pollination (FPA)** | `--epochs=10000 --n_nearest=50` | 8 950.21 | +18.6 % | 1.13 s | 99 % | 7.2 MB |

*Wall time* = elapsed wall-clock time. *CPU* = percentage of one core used (>100% would indicate parallelism). *Peak RSS* = maximum resident set size reported by GNU `time -v`.

---

## How to reproduce

```bash
cargo build --release

# Replace <solver> and flags with any row from the table above
./target/release/bin <solver> \
    -i data/tsplib/berlin52.tsp \
    --optimal-tour data/tsplib/berlin52.opt.tour

# With GNU time for resource usage:
/usr/bin/time -v ./target/release/bin nn -i data/tsplib/berlin52.tsp
```

---

## Observations

### Quality vs. speed trade-off

```
Gap from optimal
  0%  ─────────────────────────────── optimal (7 544.37)
  3%  3-opt (0.3 s)              ← best overall
  4%  CS (0.72 s)
  7%  SA (0.34 s)
  8%  GA/10k (1.63 s)
 11%  Stochastic Hill/10k (0.02 s)  ← best value for time
 15%  PSO/50p (1.42 s)
 17%  PSO/default (0.84 s)
 18%  FPA/default (0.53 s)
 19%  FPA/50 flowers (1.13 s)
 19%  NN (0.01 s)
 23%  Tabu/10k (0.47 s)
 24%  2-opt (0.01 s)
 77%  GA/500 epochs
```

**3-opt** is the strongest solver on this instance: deterministic 2.6 % gap in ~0.3 s. It seeds
from a nearest-neighbor tour and applies best-improvement-per-pass (scan all C(n,3) triples, take
the globally best improving move, restart). The result is reproducible across runs because the NN
seed and the greedy improvement strategy are both deterministic.

**Cuckoo Search** is the best stochastic option (~4–7 % across runs) at 0.72 s. It seeds nest 0
with a greedy NN tour for a strong starting neighbourhood, then runs one Lévy-flight 2-opt
perturbation per nest per epoch. Quality degrades significantly with high `pa` (≥ 0.05) because
random re-seedings overwhelm the search; the default `pa`=0.01 is near-optimal for this instance.

**Simulated Annealing** reaches ~7 % in under half a second using its default temperature
schedule. It is the strongest single-run solver for time-budgets under 0.5 s.

**Genetic Algorithm** matches SA quality (~8 %) but needs the full 10 000 generations (~1.6 s)
to get there. At 500 epochs it is the worst performer — GA needs population diversity to build
good crossover material, which takes many generations.

**Stochastic Hill** at 10 000 epochs is the best "instant" solver: 11 % gap in 20 ms. Oddly,
more epochs hurts here (22 % at 100 000) because restarts can scatter away from a good local
optimum already found.

**Nearest Neighbour** and **2-opt** complete in ≤ 10 ms and are useful as fast constructors
whose output can seed another solver. All stochastic solvers are expected to beat NN quality (+19 %)
as a sanity check.

**PSO** sits in the middle of the pack. More particles (`--n_nearest=50`) improve quality at
the cost of proportionally more wall time. Default of 30 particles is a reasonable starting
point.

**Flower Pollination (FPA)** lands mid-table (~17–19 % gap), broadly level with PSO. With the
default 25 flowers it converges in under 0.6 s; scaling to 50 flowers roughly doubles wall time
without improving quality on this instance. The greedy acceptance rule (update only if
`new_cost < current`) means the population converges very quickly — tracing shows only 1–2
global-best improvements across 10 000 epochs. Adding a Metropolis acceptance or elitism would
likely push quality closer to SA/CS.

**Tabu Search** underperforms relative to its wall time budget; the current implementation
does not improve much beyond 1 000 epochs on this instance.

### Memory

All solvers stay well under 10 MB peak RSS on a 52-city instance. Memory scales with n²
(the distance matrix) so expect roughly 4× growth going from 52 to 104 cities.

### Exact solvers

`bellman_karp` and `branch_bound` find the provably optimal tour but have factorial/exponential
complexity. They are not included here because berlin52 (52 cities) is far beyond their practical
limit (~20 cities). See the [README](../README.md) for details.

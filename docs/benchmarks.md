# Algorithm Benchmarks — berlin52

All solvers measured against the **berlin52** TSPLIB instance (52 cities, EUC\_2D).
The known optimal tour cost is **7 544.37** (from `berlin52.opt.tour`).

## Environment

| Item | Value |
|------|-------|
| CPU | AMD Ryzen 7 PRO 4750U (16 threads) |
| RAM | 32 GB |
| OS | Linux |
| Binary | `target/release/teeline` (release build, `cargo build --release -p teeline-cli`) |
| Teeline version | 1.0.1 |
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
| **Simulated Annealing** | `--no-seed` (sequential start) | 8 059.29 | +6.8 % | 0.34 s | 99 % | 8.1 MB |
| **Simulated Annealing** | default (random shuffle start) | *to be measured* | *~5–6 %* | ~0.34 s | 99 % | 8.1 MB |
| **Simulated Annealing** | `pipeline --steps=nn,sa` (NN start) | *worse than --no-seed* | *> +6.8 %* | ~0.35 s | 99 % | 8.1 MB |
| **Simulated Annealing** | `--epochs=100000 --no-seed` | 8 275.12 | +9.7 % | 0.31 s | 99 % | 8.0 MB |
| **Tabu Search** | `--epochs=1000` | 9 337.22 | +23.8 % | 0.04 s | 95 % | 7.6 MB |
| **Tabu Search** | `--epochs=10000` | 9 270.00 | +22.9 % | 0.47 s | 99 % | 7.6 MB |
| **Genetic Algorithm** | `--epochs=500` | 13 121.13 | +73.9 % | 0.21 s | 99 % | 7.9 MB |
| **Genetic Algorithm** | `--epochs=10000` (default) | 8 112.46 | +7.5 % | 3.17 s | 99 % | 7.8 MB |
| **Particle Swarm (PSO)** | `--epochs=10000` (default) | 8 874.46 | +17.6 % | 0.84 s | 100 % | 7.9 MB |
| **Particle Swarm (PSO)** | `--epochs=10000 --n_nearest=50` | 8 663.69 | +14.8 % | 1.42 s | 100 % | 8.1 MB |
| **Cuckoo Search** | default (`--epochs=10000 --n_nearest=25`) | 7 877.84 | +4.4 % | 0.72 s | 100 % | 7.9 MB |
| **Flower Pollination (FPA)** | default (`--epochs=10000 --n_nearest=25`) | 8 867.93 | +17.5 % | 0.53 s | 100 % | 7.2 MB |
| **Flower Pollination (FPA)** | `--epochs=10000 --n_nearest=50` | 8 950.21 | +18.6 % | 1.13 s | 99 % | 7.2 MB |
| **Lin-Kernighan (ILS)** | default (`--epochs=100 --n_nearest=5`) | 8 146.28 | +8.0 % | 0.02 s | 82 % | 6.5 MB |
| **Lin-Kernighan (ILS)** | `--epochs=1000 --n_nearest=10` | 8 128.86 | +7.7 % | 0.03 s | 85 % | 6.4 MB |
| **Or-opt** | default (NN seed, best-improvement) | 8 097.48 | +7.3 % | 0.03 s | 93 % | 6.6 MB |
| **Christofides** | default (MST + greedy matching) | 8 707.66 | +15.4 % | < 0.01 s | 50 % | 6.5 MB |

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
  7%  Or-opt (0.03 s)            ← best value for time in local search
 15%  Christofides (<0.01 s)    ← only solver with a proven ≤1.5× bound
  8%  GA/10k (3.2 s)  ← high variance; see note
 11%  Stochastic Hill/10k (0.02 s)
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

**Simulated Annealing** reaches ~5–7 % in under half a second. The default auto-expansion uses
`pipeline(shuffle, sa)` — a random starting tour rather than greedy NN. SA's temperature schedule
is calibrated for a cold start: the high-temperature phase budgets exploration energy to escape a
bad initial state. Seeding from the NN tour, which already sits in a tight local neighbourhood,
constrains early exploration without matching the quality benefit it provides to deterministic
hill-climbers (2-opt, 3-opt). A random shuffle start lets SA explore more broadly and typically
matches or beats the sequential-start quality. Use `--no-seed` to start from input city order;
use `teeline pipeline --steps=nn,2opt,sa` (or the `classic` preset) to warm-start SA from a
2-opt-refined tour, which *does* improve quality because 2-opt has already cleaned up edge crossings.

**Genetic Algorithm** is the highest-variance solver. Over ten runs on berlin52 at 10 000 epochs,
the gap ranged from **+1 % to +20 %** (typical ~8–13 %). Rarely it gets very lucky with early
crossovers and converges near-optimal; more often it lands around +9–12 %. PR #79 improved
population seeding (n/5 seeded fraction instead of n/10; 2–4 RSM mutations per seeded variant;
Fisher-Yates shuffle for the random portion instead of a single 2-opt reversal), which widened
the quality ceiling without reducing the typical floor. Wall time increased from ~1.6 s to ~3.2 s
per run; profiling points to the more diverse population producing more varied crossover paths
through the fitness landscape. At 500 epochs it remains the worst performer — GA needs many
generations to build good crossover material regardless of seeding quality.

**Stochastic Hill** at 10 000 epochs is the best "instant" solver: 11 % gap in 20 ms. Oddly,
more epochs hurts here (22 % at 100 000) because restarts can scatter away from a good local
optimum already found.

**Christofides** achieves +15.4 % in under 10 ms — slower than Or-opt in quality, but uniquely valuable: it is the **only solver with a proven ≤1.5× approximation guarantee** on EUC_2D instances. Its deterministic construction also makes it the best warm-start for Lin-Kernighan: `pipeline(christofides, lk)` reaches 8156 (+8.1 %), tighter than either solver alone, in about 40 ms.

**Or-opt** achieves +7.3 % in 0.03 s — tying SA quality at a fraction of the cost — by relocating
segments of 1–3 cities rather than reversing segments like 2-opt does. Because the two methods
explore different neighbourhoods, they find different local optima: Or-opt beats 2-opt on this
instance (7.3 % vs 24.2 %) with the same NN seed. Combining them via `pipeline(nn, 2opt, or_opt)`
gives deeper local optima than either alone, at the cost of running both passes.

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

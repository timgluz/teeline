# Teeline

Teeline is a solver for the symmetric Traveling Salesman Problem, written in Rust.

> *The Traveling Salesman Problem (TSP) is the search for a minimum cost Hamiltonian circuit connecting a set of locations.* — [source](http://www.optimization-online.org/DB_FILE/2017/12/6370.pdf)

It is a work in progress. It already implements all algorithms typically covered by a CS algorithms course. More advanced algorithms will be added once the code structure and interfaces have stabilised.

## Backstory

It all started from the ["In Pursuit of the Traveling Salesman"](https://www.amazon.de/Pursuit-Traveling-Salesman-Mathematics-Computation-ebook/dp/B0073X0IR2/) book — a fantastic read that covers the history of the problem and the big ideas behind the Concorde solver. I was genuinely surprised that Linear Programming works so well here and can provide exact solutions for very large instances.

After finishing the book I took the [Discrete Optimization](https://coursera.org/share/1428f00fd18abc041afcf9105c02365b) course on Coursera to learn more about the theory behind Concorde. One of the assignments asked for a solver that could handle more than 10,000 cities, which pushed me to experiment with different heuristics — and gave me a great opportunity to learn Rust.

---

## Getting Started

### Prerequisites

- **Rust toolchain** — install via [rustup](https://www.rust-lang.org/tools/install):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- Rust 1.80 or later is required (uses `std::sync::LazyLock`).

### Build

```bash
# debug build (faster compile, slower runtime)
cargo build

# optimised release build (recommended for real use)
cargo build --release
```

### Quick test

```bash
# run the test suite
cargo test

# check the CLI help
./target/release/bin --help
./target/release/bin solve --help
./target/release/bin convert --help
```

### Install locally (optional)

Copy or symlink the binary so you can call it as `teeline`:

```bash
cp ./target/release/bin ~/bin/teeline   # or any directory on your PATH
```

All examples below assume this step has been done. If you skipped it, replace `teeline` with `./target/release/bin`.

---

## Preparing Data

Teeline reads a subset of the TSPLIB format — cities must be given as 2D Euclidean coordinates in either a `NODE_COORD_SECTION` or `DISPLAY_DATA_SECTION`.

### Downloading TSPLIB benchmark instances

1. Download the archive from the [TSPLIB symmetric TSP page](https://comopt.ifi.uni-heidelberg.de/software/TSPLIB95/tsp.html):

```bash
curl -O -L https://comopt.ifi.uni-heidelberg.de/software/TSPLIB95/tsp/ALL_tsp.tar.gz
```

2. Unpack into the `data/` folder:

```bash
mkdir -p data/tsplib
tar -xzf ALL_tsp.tar.gz -C data/tsplib
```

3. The archive contains individually gzipped files — decompress them all in one go:

```bash
gunzip data/tsplib/*.gz
```

### Converting your own coordinates

Teeline includes a native `convert` subcommand that converts DiscOpt-format coordinate files (first line is the city count and is ignored; remaining lines are `x y` float pairs) to TSPLIB EUC_2D format:

```bash
# single file → produces data/discopt/tsp_51_1.tsp
teeline convert -i ./data/raw/tsp_51_1 -o ./data/discopt/

# whole directory at once
teeline convert -i ./data/raw/ -o ./data/discopt/
```

---

## Run your first solve

Teeline reads city data from a file or stdin and prints the tour cost followed by the ordered city IDs.

```bash
# from a file
teeline solve nn -i ./data/tsplib/berlin52.tsp

# from stdin
cat ./data/tsplib/berlin52.tsp | teeline solve nn

# with the visualization window
teeline solve nn -i ./data/tsplib/berlin52.tsp --gui
```

Output format:

```
10628.46302 0
1 49 32 45 19 41 8 9 10 43 33 51 11 52 6 22 ...
```

First line: `<tour_cost> <optimised_flag>`. Second line: space-separated city IDs in visit order.

---

## Algorithms

### Exact algorithms

Exact algorithms always find the optimal solution but have exponential or factorial time complexity. **Do not use them on more than ~20 cities.**

#### Bellman–Held–Karp (`bellman_karp`, `bhk`)

Dynamic programming algorithm that solves TSP in O(2ⁿ · n²) time.

```bash
teeline solve bhk -i ./data/discopt/tsp_5_1.tsp
teeline solve bellman_karp -i ./data/discopt/tsp_5_1.tsp --verbose
```

Resources:
- [Bellman-Held-Karp walkthrough (YouTube)](https://youtu.be/D8aHqaFa8GE)
- *Algorithms Illuminated, Part 4* — Tim Roughgarden

#### Branch and Bound (`branch_bound`)

Systematic enumeration of candidate solutions; prunes branches that cannot improve on the best solution found so far.

```bash
teeline solve branch_bound -i ./data/discopt/tsp_5_1.tsp
teeline solve branch_bound -i ./data/discopt/tsp_5_1.tsp --verbose
```

Resources:
- [EECS 281: Backtracking and Branch & Bound (YouTube)](https://www.youtube.com/watch?v=hNs7G1b2iFY&t=5480s)
- [GeeksForGeeks article](https://www.geeksforgeeks.org/traveling-salesman-problem-using-branch-and-bound-2/)

---

### Approximate algorithms

Approximate (heuristic) algorithms trade optimality guarantees for speed, making them practical for large instances.

#### Nearest Neighbor (`nearest_neighbor`, `nn`)

Greedy construction: from the current city, always move to the closest unvisited city. Uses a KD-tree for fast lookups.

```bash
teeline solve nn -i ./data/tsplib/berlin52.tsp
teeline solve nn -i ./data/tsplib/berlin52.tsp --verbose
```

Resources:
- *Algorithms and Data Structures in Action* — Marcello La Rocca
- [Nearest neighbour algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Nearest_neighbour_algorithm)

#### 2-opt (`two_opt`, `2opt`)

Local search: repeatedly reverse sub-segments of the tour to remove crossings until no improving swap exists.

```bash
teeline solve 2opt -i ./data/tsplib/berlin52.tsp
teeline solve two_opt -i ./data/tsplib/berlin52.tsp --verbose
```

Resources:
- [Section 20.4: The 2-OPT Heuristic (YouTube)](https://youtu.be/dYEWqrp-mho)
- [2-opt (Wikipedia)](https://en.wikipedia.org/wiki/2-opt)

#### Stochastic Hill Climbing (`stochastic_hill`)

Iterative improvement with random restarts to escape plateaus and local optima.

Options:
| Flag | Description | Default |
|---|---|---|
| `--epochs` | Maximum iterations (0 = unlimited) | 0 |
| `--platoo_epochs` | Steps without improvement before restart | — |

```bash
teeline solve stochastic_hill -i ./data/tsplib/berlin52.tsp
teeline solve stochastic_hill -i ./data/tsplib/berlin52.tsp --epochs=1000
teeline solve stochastic_hill -i ./data/tsplib/berlin52.tsp --platoo_epochs=50
```

Resources:
- *AIMA*, Chapter 4.1 — Local Search and Optimization Problems
- [Hill climbing (Wikipedia)](https://en.wikipedia.org/wiki/Hill_climbing)

#### Simulated Annealing (`simulated_annealing`, `sa`)

Probabilistic local search that accepts worsening moves with a probability that decreases as the "temperature" cools, allowing escape from local optima.

Options:
| Flag | Description | Default |
|---|---|---|
| `--max_temperature` | Starting temperature | 1000.0 |
| `--min_temperature` | Stopping temperature | 0.001 |
| `--cooling_rate` | Fractional temperature drop per step | — |
| `--epochs` | Maximum iterations | — |

```bash
teeline solve sa -i ./data/tsplib/berlin52.tsp
teeline solve sa -i ./data/tsplib/berlin52.tsp --verbose
teeline solve sa -i ./data/tsplib/berlin52.tsp --cooling_rate=0.003 --max_temperature=500.0
```

Resources:
- *AIMA*, Section 4.1.2 — Simulated Annealing
- [Simulated annealing (Wikipedia)](https://en.wikipedia.org/wiki/Simulated_annealing)

#### Tabu Search (`tabu_search`)

Local search that maintains a short-term memory (the *tabu list*) of recently visited solutions to avoid cycling, and accepts worsening moves when no improvement is available.

Options:
| Flag | Description | Default |
|---|---|---|
| `--epochs` | Maximum iterations | — |

```bash
teeline solve tabu_search -i ./data/tsplib/berlin52.tsp
teeline solve tabu_search -i ./data/tsplib/berlin52.tsp --epochs=500
```

Resources:
- [Tabu search (Wikipedia)](https://en.wikipedia.org/wiki/Tabu_search)
- *Heuristic Search*, Chapter 14.4

#### Genetic Algorithm (`genetic_algorithm`, `ga`)

Evolutionary metaheuristic: maintains a population of candidate tours and iteratively applies selection, crossover, and mutation to evolve better solutions.

Options:
| Flag | Description | Default |
|---|---|---|
| `--epochs` | Maximum generations | 10 000 |
| `--mutation_probability` | Probability of random swap on a child | 0.001 |
| `--n_elite` | Individuals passed unchanged to next generation | 3 |

```bash
teeline solve ga -i ./data/tsplib/berlin52.tsp
teeline solve ga -i ./data/tsplib/berlin52.tsp --verbose
teeline solve ga -i ./data/tsplib/berlin52.tsp --epochs=500 --mutation_probability=0.2
teeline solve ga -i ./data/tsplib/berlin52.tsp --n_elite=7
```

Resources:
- [Genetic algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Genetic_algorithm)
- *AIMA*, Section 4.1.4 — Genetic Algorithms

#### Particle Swarm Optimisation (`particle_swarm`, `pso`)

Swarm metaheuristic: each particle is a candidate tour that moves through the search space pulled toward its own personal best and the global best tour found so far. The velocity is an ordered list of position swaps rather than a continuous vector.

**TSP-specific adaptations** (deviations from the textbook Clerc 2004 algorithm):

| Adaptation | Why |
|---|---|
| Velocity cap at `⌈0.35 · n⌉` swaps | Without a cap, steady-state velocity grows to ~5 n swaps, scrambling the tour into noise |
| Linear inertia decay W: 0.9 → 0.4 | High inertia early for broad exploration; decays like a cooling schedule so late epochs fine-tune around the best region found |
| Particle 0 seeded with a greedy NN tour | Gives the swarm a good starting neighbourhood; remaining particles are random for diversity |

Options:
| Flag | Description | Default |
|---|---|---|
| `--epochs` | Maximum iterations | 10 000 |
| `--n_nearest` | Number of particles (floored at 30) | 30 |

```bash
teeline solve pso -i ./data/tsplib/berlin52.tsp
teeline solve particle_swarm -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve pso -i ./data/tsplib/berlin52.tsp --n_nearest=50
```

Resources:
- [Particle swarm optimisation (Wikipedia)](https://en.wikipedia.org/wiki/Particle_swarm_optimization)
- Kennedy & Eberhart (1995) — *Particle Swarm Optimization*
- Clerc (2004) — *Discrete Particle Swarm Optimization, illustrated by the Traveling Salesman Problem*

#### Cuckoo Search (`cuckoo_search`, `cs`)

Nature-inspired metaheuristic: maintains a population of nests (candidate tours). Each epoch it generates a new cuckoo tour via a Lévy-flight step (a sequence of random 2-opt reversals whose count is drawn from a power-law Lévy distribution), then replaces a random worse nest with the cuckoo if it's better. Each nest is independently abandoned with probability `pa` and re-seeded randomly each epoch to maintain diversity.

**TSP-specific adaptations** (deviations from Yang & Deb 2009):

| Adaptation | Why |
|---|---|
| Lévy flight → k random 2-opt reversals | Maps continuous Lévy step magnitude to a discrete tour perturbation; preserves permutation validity |
| k capped at n/2 | Prevents full-tour scrambles from very large Lévy draws |
| β=1.5 fixed; σ_u≈0.6966 precomputed | Standard Lévy exponent (Mantegna 1994); constant avoids repeated gamma evaluation |
| Per-nest Bernoulli abandonment | Closer to the original paper than deterministic worst-k; avoids discarding more information per epoch than Lévy moves can recover |

Options:
| Flag | Description | Default |
|---|---|---|
| `--epochs` | Maximum iterations | 10 000 |
| `--n_nearest` | Number of nests (floored at 25) | 25 |
| `--mutation_probability` | Per-nest abandonment probability `pa` | 0.001 |

```bash
teeline solve cs -i ./data/tsplib/berlin52.tsp
teeline solve cuckoo_search -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve cs -i ./data/tsplib/berlin52.tsp --n_nearest=40 --mutation_probability=0.25
```

Resources:
- Yang & Deb (2009) — *Cuckoo Search via Lévy Flights*
- [Cuckoo search (Wikipedia)](https://en.wikipedia.org/wiki/Cuckoo_search)

#### Flower Pollination Algorithm (`flower_pollination`, `fpa`)

Nature-inspired metaheuristic: each flower is a candidate tour. Each epoch it applies either *global pollination* (Lévy-flight-scaled movement toward the global best tour) or *local pollination* (ε-scaled displacement from two randomly chosen flowers). The switch probability controls the balance between exploitation and exploration.

**TSP-specific adaptations** (deviations from Yang 2012):

| Adaptation | Why |
|---|---|
| Global pollination → Lévy-scaled prefix of the swap sequence toward gbest | Permutation analogue of `x + γ·L·(g* − x)`; preserves tour validity |
| Local pollination → ε-scaled prefix of the swap diff between two random flowers | Permutation analogue of `x + ε·(x_j − x_k)` |
| switch_prob floored at 0.8 when `mutation_probability < 0.01` | Prevents degeneration to 99.9 % local-only search under default CLI options |

Options:
| Flag | Description | Default |
|---|---|---|
| `--epochs` | Maximum iterations | 10 000 |
| `--n_nearest` | Number of flowers (floored at 25) | 25 |
| `--mutation_probability` | Switch probability (global vs local pollination) | 0.8 |

```bash
teeline solve fpa -i ./data/tsplib/berlin52.tsp
teeline solve flower_pollination -i ./data/tsplib/berlin52.tsp --epochs=500
teeline solve fpa -i ./data/tsplib/berlin52.tsp --n_nearest=50 --mutation_probability=0.8
```

Resources:
- Yang (2012) — *Flower Pollination Algorithm for Global Optimization*
- [Flower pollination algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Flower_pollination_algorithm)

---

## Visualising Results

While solving, Teeline runs headless by default. Pass `--gui` to open a window that shows the current best route updating in real time.

### Comparing against a known-optimal tour

Pass `--optimal-tour <FILE>` with a TSPLIB `.opt.tour` file to overlay the optimal route on the visualisation and print a gap comparison to stderr after solving.

```bash
teeline solve ga -i data/tsplib/berlin52.tsp \
    --optimal-tour data/tsplib/berlin52.opt.tour
```

Example output (stderr):

```
--- Comparison ---
Optimal  : 7544.36572  (from BERLIN52.OPT.TOUR)
Solver   : 7953.25830
Gap      : +5.42 %
```

The visualisation window uses colour to show how the solver's best tour compares to the optimal:

| Colour | Meaning |
|--------|---------|
| Very dark green (thick) | Edge appears in **both** the optimal tour and the solver's best — correctly found |
| Light gray | Edge is in the optimal tour but **missed** by the solver |
| Dark green | Solver's best route |
| Blue | Current route being explored |

Optimal tour files for the TSPLIB benchmark instances are included in `data/tsplib/` (e.g. `berlin52.opt.tour`).

You can also upload your input file and the solution output to the [Discrete Optimization visualiser](https://discreteoptimization.github.io/vis/tsp/) to inspect tours interactively.

---

## Benchmarks

See [docs/benchmarks.md](docs/benchmarks.md) for a full comparison of all solvers on the berlin52 instance (52 cities, known optimal 7 544.37), including tour quality, wall time, CPU usage, and peak memory — measured with the release binary under a 3-minute timeout.

Quick summary:

| Algorithm | Gap | Wall time |
|-----------|:---:|----------:|
| Cuckoo Search (default) | +4.4 % | 0.72 s |
| Simulated Annealing (default) | +6.8 % | 0.34 s |
| Genetic Algorithm (10 000 ep) | +8.3 % | 1.63 s |
| Stochastic Hill (10 000 ep) | +11.2 % | 0.02 s |
| PSO (50 particles, 10 000 ep) | +14.8 % | 1.42 s |
| Flower Pollination (default) | +17.5 % | 0.53 s |
| Nearest Neighbour | +19.0 % | 0.01 s |

---

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow.

In short:

1. Open a GitHub issue to discuss the change before writing code.
2. Implement the feature or fix on a dedicated branch.
3. Add tests — unit tests go inline with the source file; integration tests go in `tests/`.
4. Open a pull request and wait for a code review.

When adding a new solver, follow the pattern of the existing ones: a `solve(cities: &[KDPoint], options: &SolverOptions) -> Solution` function in its own file under `src/tsp/`, registered in the `Solvers` enum in `src/tsp/mod.rs` and the `solve` match arm in `src/main.rs`.

---

## Contributors

- **[Timo Sulg](https://github.com/timgluz)** — author and maintainer
- **[equalis3r](https://github.com/equalis3r)** — Bellman–Held–Karp fix and test (PR #25)

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

1. Go to the [TSPLIB symmetric TSP page](https://comopt.ifi.uni-heidelberg.de/software/TSPLIB95/tsp.html).
2. Download the archive(s) you want (`ALL_tsp.tar.gz` for everything, or individual `.tsp.gz` files).
3. Unpack into the `data/` folder:

```bash
mkdir -p data/tsplib
tar -xzf ALL_tsp.tar.gz -C data/tsplib        # full archive
gunzip -c berlin52.tsp.gz > data/tsplib/berlin52.tsp  # single file
```

### Converting your own coordinates

To convert a plain list of coordinates to TSPLIB format use the included helper:

```bash
python3 convert2tsplib.py
```

---

## Run your first solve

Teeline reads city data from a file or stdin and prints the tour cost followed by the ordered city IDs.

```bash
# from a file
teeline nn -i ./data/tsplib/berlin52.tsp

# from stdin
cat ./data/tsplib/berlin52.tsp | teeline nn

# headless / CI — skip the visualisation window
teeline nn -i ./data/tsplib/berlin52.tsp --disable_progress
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
teeline bhk -i ./data/discopt/tsp_5_1.tsp
teeline bellman_karp -i ./data/discopt/tsp_5_1.tsp --verbose
```

Resources:
- [Bellman-Held-Karp walkthrough (YouTube)](https://youtu.be/D8aHqaFa8GE)
- *Algorithms Illuminated, Part 4* — Tim Roughgarden

#### Branch and Bound (`branch_bound`)

Systematic enumeration of candidate solutions; prunes branches that cannot improve on the best solution found so far.

```bash
teeline branch_bound -i ./data/discopt/tsp_5_1.tsp
teeline branch_bound -i ./data/discopt/tsp_5_1.tsp --verbose
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
teeline nn -i ./data/tsplib/berlin52.tsp
teeline nn -i ./data/tsplib/berlin52.tsp --verbose
```

Resources:
- *Algorithms and Data Structures in Action* — Marcello La Rocca
- [Nearest neighbour algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Nearest_neighbour_algorithm)

#### 2-opt (`two_opt`, `2opt`)

Local search: repeatedly reverse sub-segments of the tour to remove crossings until no improving swap exists.

```bash
teeline 2opt -i ./data/tsplib/berlin52.tsp
teeline two_opt -i ./data/tsplib/berlin52.tsp --verbose
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
teeline stochastic_hill -i ./data/tsplib/berlin52.tsp
teeline stochastic_hill -i ./data/tsplib/berlin52.tsp --epochs=1000
teeline stochastic_hill -i ./data/tsplib/berlin52.tsp --platoo_epochs=50
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
teeline sa -i ./data/tsplib/berlin52.tsp
teeline sa -i ./data/tsplib/berlin52.tsp --verbose
teeline sa -i ./data/tsplib/berlin52.tsp --cooling_rate=0.003 --max_temperature=500.0
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
teeline tabu_search -i ./data/tsplib/berlin52.tsp
teeline tabu_search -i ./data/tsplib/berlin52.tsp --epochs=500
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
teeline ga -i ./data/tsplib/berlin52.tsp
teeline ga -i ./data/tsplib/berlin52.tsp --verbose
teeline ga -i ./data/tsplib/berlin52.tsp --epochs=500 --mutation_probability=0.2
teeline ga -i ./data/tsplib/berlin52.tsp --n_elite=7
```

Resources:
- [Genetic algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Genetic_algorithm)
- *AIMA*, Section 4.1.4 — Genetic Algorithms

---

## Visualising Results

While solving, Teeline opens a Piston window that shows the current best route updating in real time. Pass `--disable_progress` to suppress it (required in headless / CI environments).

You can also upload your input file and the solution output to the [Discrete Optimization visualiser](https://discreteoptimization.github.io/vis/tsp/) to inspect tours interactively.

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

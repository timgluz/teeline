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

The workspace has two crates: `teeline` (pure solver library) and `teeline-cli` (CLI binary). To get the runnable binary, build `teeline-cli`:

```bash
# debug build — fast compile, slower runtime
cargo build -p teeline-cli

# optimised release build — recommended for real use
cargo build -p teeline-cli --release

# solver library only (useful for embedding)
cargo build -p teeline
```

With [go-task](https://taskfile.dev/) installed you can use the shorter aliases:

```bash
task build          # debug binary (teeline-cli)
task build:release  # release binary
```

### Quick test

```bash
# run the unit and integration test suite
cargo test -p teeline -p teeline-cli

# run end-to-end CLI tests (requires the debug binary to be built first)
./tests/bats/bin/bats tests/e2e/

# check the CLI help
./target/release/teeline --help
./target/release/teeline solve --help
./target/release/teeline convert --help
./target/release/teeline solvers --help
```

### Install from a GitHub Release (no Rust required)

Pre-built binaries for Linux (x86\_64/aarch64), macOS (x86\_64/aarch64), and Windows (x86\_64) are published on the [Releases page](https://github.com/timgluz/teeline/releases).

A shell installer is also available:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/timgluz/teeline/releases/latest/download/teeline-cli-installer.sh | sh
```

Each release also includes `teeline-solver.wasm` — the WebAssembly component built with `cargo-component`.

### Install locally from source (optional)

```bash
cp ./target/release/teeline ~/bin/teeline   # or any directory on your PATH
```

All examples below assume `teeline` is on your PATH.

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
```

Output format:

```
10628.46302 0
1 49 32 45 19 41 8 9 10 43 33 51 11 52 6 22 ...
```

First line: `<tour_cost> <optimised_flag>`. Second line: space-separated city IDs in visit order.

---

## Listing available solvers

The `solvers` subcommand prints the solver catalogue so you never have to look up alias names manually.

```bash
# human-readable table: full name, short alias, type
teeline solvers

# one alias per line — suitable for shell scripting
teeline solvers --short

# heuristic solvers only (excludes exact algorithms)
teeline solvers --heuristic --short

# exact solvers only
teeline solvers --exact
```

Example output:

```
NAME                   ALIAS    TYPE
bellman_karp           bhk      exact
branch_bound           —        exact
nearest_neighbor       nn       heuristic
two_opt                2opt     heuristic
three_opt              3opt     heuristic
simulated_annealing    sa       heuristic
genetic_algorithm      ga       heuristic
tabu_search            tabu     heuristic
particle_swarm         pso      heuristic
cuckoo_search          cs       heuristic
flower_pollination     fpa      heuristic
stochastic_hill        —        heuristic
random_shuffle         shuffle  utility
```

The `--short` form is useful for driving scripts or benchmarks:

```bash
for solver in $(teeline solvers --heuristic --short); do
    teeline solve $solver -i data/tsplib/berlin52.tsp 2>/dev/null | head -1
done
```

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
# auto-expands to pipeline(shuffle, sa) — random start for broad exploration
teeline solve sa -i ./data/tsplib/berlin52.tsp
teeline solve sa -i ./data/tsplib/berlin52.tsp --verbose

# run SA from input city order (no random shuffle)
teeline solve sa --no-seed -i ./data/tsplib/berlin52.tsp

# warm-start from a 2-opt-refined tour (best quality — use the 'classic' preset)
teeline solve classic -i ./data/tsplib/berlin52.tsp

teeline solve sa -i ./data/tsplib/berlin52.tsp --cooling_rate=0.003 --max_temperature=500.0
```

> **Why random and not NN?** SA's temperature schedule is calibrated for cold starts — the
> high-temperature phase is designed to escape a *bad* initial tour via broad exploration.
> A greedy NN tour already sits in a tight local neighbourhood; seeding SA from there
> constrains early exploration and typically *worsens* the final result compared to a random
> start. Deterministic local-search solvers (2-opt, 3-opt, tabu) are the opposite: they are
> monotone hill-climbers, so a better start strictly means a better end — they get NN seeding.
> If you want SA with a warm start, use the `classic` preset (`nn → 2opt → sa`): the 2-opt
> stage first removes edge crossings, handing SA a cleaner tour that it can fine-tune.

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
| All particles initialised randomly | Seeding happens externally via the pipeline (see [Pipeline](#pipeline)); PSO itself stays a pure algorithm |

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
| All nests initialised randomly | Seeding happens externally via the pipeline (see [Pipeline](#pipeline)); CS itself stays a pure algorithm |

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
| All flowers initialised randomly | Seeding happens externally via the pipeline (see [Pipeline](#pipeline)); FPA itself stays a pure algorithm |

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

## Pipeline

Local search algorithms (2-opt, 3-opt, SA, hill climbing, tabu, GA, PSO, CS, FPA) improve an existing tour; they do not construct one from scratch. Starting from a random or sequential tour wastes the early epochs escaping a bad initial state. **Warm-starting from a greedy Nearest Neighbour tour** gives the solver a much better region to refine, typically reducing the optimality gap by several percentage points at no extra tuning cost.

Teeline makes this composable through the pipeline mechanism: solvers are chained in sequence, each stage receiving the best tour from the previous stage as its starting point.

### Auto-expansion

Auto-expansion strategy depends on the solver type:

| Solver type | Auto-expands to | Why |
|---|---|---|
| **Deterministic local search** (2opt, 3opt, tabu) | `pipeline(nn, solver)` | Monotone hill-climbers: better start = better end |
| **Stochastic** (sa, stochastic_hill, ga, pso, cs, fpa) | `pipeline(shuffle, solver)` | Temperature/diversity schedules are calibrated for cold starts; NN start constrains early exploration |
| **Constructive** (nn, bhk, branch_bound) | no expansion | They build a tour from scratch |

```bash
# sa auto-expands to pipeline(shuffle, sa)
teeline solve sa -i ./data/tsplib/berlin52.tsp
teeline pipeline --steps=shuffle,sa -i ./data/tsplib/berlin52.tsp  # equivalent

# 2opt auto-expands to pipeline(nn, 2opt)
teeline solve 2opt -i ./data/tsplib/berlin52.tsp
teeline pipeline --steps=nn,2opt -i ./data/tsplib/berlin52.tsp  # equivalent
```

Pass `--no-seed` to disable auto-expansion and run the solver from input city order:

```bash
teeline solve sa --no-seed -i ./data/tsplib/berlin52.tsp
```

### Named presets

Three presets bundle commonly useful chains:

| Preset | Expands to | Character |
|--------|-----------|-----------|
| `fast` | nn → 2-opt | Deterministic, sub-second, good quality |
| `classic` | nn → 2-opt → SA | Balanced quality and speed |
| `thorough` | nn → 3-opt → SA | Best quality, slower |

```bash
teeline solve fast      -i ./data/tsplib/berlin52.tsp
teeline solve classic   -i ./data/tsplib/berlin52.tsp
teeline solve thorough  -i ./data/tsplib/berlin52.tsp
```

### Custom pipelines

The `pipeline` subcommand accepts any comma-separated sequence of solver names:

```bash
# nn → 2-opt → tabu search
teeline pipeline --steps=nn,2opt,tabu_search -i ./data/tsplib/berlin52.tsp

# pass tuning flags — they apply to all stages that use them
teeline pipeline --steps=nn,sa --epochs=5000 --cooling_rate=0.005 \
    -i ./data/tsplib/berlin52.tsp
```

Constructive solvers (`nn`, `bhk`, `branch_bound`) ignore `initial_tour` and are best placed first. A warning is printed if `nn` appears at a non-first position, as it would discard the warm-start seed.

---

## WebAssembly

All solvers are also available as a [WebAssembly Component Model](https://component-model.bytecodealliance.org/) library — no HTTP server required. The component exposes a single typed `solve` function callable from Go, Python, JavaScript, Rust, and any WIT-compatible runtime including [Spin](https://spinframework.io/).

```bash
# build the component
cargo component build --manifest-path teeline-wasm/Cargo.toml --release

# call it from Node.js (after jco transpile)
import { solve } from './teeline-wasm/js-bindings/teeline_wasm.js';
const result = solve('sa', cities, options);
```

See **[docs/wasm.md](docs/wasm.md)** for the full interface reference, build instructions, and working examples in JavaScript, Python, Go, and Rust.

---

## Comparing against a known-optimal tour

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

Optimal tour files for the TSPLIB benchmark instances are included in `data/tsplib/` (e.g. `berlin52.opt.tour`).

You can also upload your input file and the solution output to the [Discrete Optimization visualiser](https://discreteoptimization.github.io/vis/tsp/) to inspect tours interactively.

---

## Benchmarks

See [docs/benchmarks.md](docs/benchmarks.md) for a full comparison of all solvers on the berlin52 instance (52 cities, known optimal 7 544.37), including tour quality, wall time, CPU usage, and peak memory — measured with the release binary under a 3-minute timeout.

Quick summary (pipeline presets first, then standalone `--no-seed` baselines):

| Algorithm | Gap | Wall time |
|-----------|:---:|----------:|
| `classic` preset (nn→2opt→sa) | +4.8 % | 0.36 s |
| `fast` preset (nn→2opt) | +11.2 % | < 0.01 s |
| — | — | — |
| Cuckoo Search (default, shuffle start) | +4.4 % | 0.72 s |
| Simulated Annealing (default, shuffle start) | ~5–6 % | 0.34 s |
| Genetic Algorithm (default, 10 000 ep) | +8.3 % | 1.63 s |
| Stochastic Hill (default, 10 000 ep) | +11.2 % | 0.02 s |
| PSO (default, 50 particles, 10 000 ep) | +14.8 % | 1.42 s |
| Flower Pollination (default) | +17.5 % | 0.53 s |
| Nearest Neighbour | +19.0 % | 0.01 s |

> **Note:** Stochastic solvers (sa, ga, pso, cs, fpa, stochastic_hill) auto-expand to `pipeline(shuffle, solver)`. Use `--no-seed` to skip the shuffle and run from input city order. Use `teeline solve classic` for the best SA result via `nn → 2opt → sa`.

---

## Development

Common development tasks are standardised in [`Taskfile.yml`](Taskfile.yml). Install [go-task](https://taskfile.dev/installation/):

```bash
brew install go-task                        # macOS
go install github.com/go-task/task/v3/cmd/task@latest  # any platform with Go
```

See all available tasks:

```bash
task --list
```

Key workflows:

```bash
task build               # compile debug binary
task test                # run unit and integration tests
task test:e2e            # run BATS end-to-end CLI tests
task test:all            # unit + e2e
task lint                # clippy (warnings are errors)
task fmt                 # auto-format with rustfmt
task check               # build + test + lint + fmt:check (mirrors CI)

task run -- solve nn -i tests/fixtures/berlin52.tsp   # run solver via cargo run
task bench:berlin52      # compare all approximate solvers on berlin52 (release build)
task build:wasm          # build the WebAssembly component
```

### Releasing

Releases are automated via [cargo-dist](https://axodotdev.github.io/cargo-dist/). To cut a new release:

```bash
# 1. Bump version in teeline-cli/Cargo.toml, commit and push
# 2. Tag the commit
git tag v0.2.0
git push origin v0.2.0
```

The `release.yml` workflow triggers, builds native binaries for all five platforms plus the WASM component, and publishes everything to a GitHub Release. You can preview what will be built with:

```bash
dist plan
```

---

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) for the workflow.

In short:

1. Open a GitHub issue to discuss the change before writing code.
2. Implement the feature or fix on a dedicated branch.
3. Add tests — unit tests go inline with the source file; library integration tests go in `tests/`; CLI end-to-end tests go in `tests/e2e/` as BATS scripts.
4. Open a pull request and wait for a code review.

When adding a new solver, follow the pattern of the existing ones: a `solve(cities: &[KDPoint], distances: &DistanceMatrix, options: &SolverOptions) -> Solution` function in its own file under `src/tsp/`, then register it in four places in `src/tsp/mod.rs`: the `Solvers` enum, the `FromStr` impl, the `tsp::solve` dispatch match arm, and `Solvers::all_meta()` (the catalogue used by the `solvers` subcommand). The WASM surface picks it up automatically.

---

## Contributors

- **[Timo Sulg](https://github.com/timgluz)** — author and maintainer
- **[equalis3r](https://github.com/equalis3r)** — Bellman–Held–Karp fix and test (PR #25)

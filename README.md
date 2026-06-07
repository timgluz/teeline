# Teeline

Teeline is a solver for the symmetric Traveling Salesman Problem, written in Rust.

> *The Traveling Salesman Problem (TSP) is the search for a minimum cost Hamiltonian circuit connecting a set of locations.* — [source](http://www.optimization-online.org/DB_FILE/2017/12/6370.pdf)

It is a work in progress. It already implements all algorithms typically covered by a CS algorithms course. More advanced algorithms will be added once the code structure and interfaces have stabilised.

## Subprojects

| Subproject | Description |
|---|---|
| [teeline-cli](teeline-cli/README.md) | Command-line solver — reads TSPLIB files, prints the best tour found |
| [teeline-qt](teeline-qt/README.md) | Qt 6 desktop GUI with live solver visualization and a pipeline builder |
| [teeline-wasm](teeline-wasm/README.md) | WebAssembly Component Model build — callable from JS, Python, Go, and Rust |
| [teeline-web](teeline-web/README.md) | Browser-based solver at [tspsolver.com](https://tspsolver.com) — upload a `.tsp` file, configure a solver, and download the optimised tour |

The `teeline` crate at the root is the pure solver library shared by all three.

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

| Algorithm | Alias | Type | Docs |
|-----------|-------|------|------|
| Bellman–Held–Karp | `bhk` | exact | [→](docs/algorithms/bellman-held-karp.md) |
| Branch and Bound | `branch_bound` | exact | [→](docs/algorithms/branch-bound.md) |
| Nearest Neighbor | `nn` | heuristic — constructive | [→](docs/algorithms/nearest-neighbor.md) |
| 2-opt | `2opt` | heuristic — local search | [→](docs/algorithms/two-opt.md) |
| 3-opt | `3opt` | heuristic — local search | [→](docs/algorithms/three-opt.md) |
| Stochastic Hill Climbing | `stochastic_hill` | heuristic — local search | [→](docs/algorithms/stochastic-hill.md) |
| Simulated Annealing | `sa` | heuristic — local search | [→](docs/algorithms/simulated-annealing.md) |
| Tabu Search | `tabu_search` | heuristic — local search | [→](docs/algorithms/tabu-search.md) |
| Genetic Algorithm | `ga` | heuristic — evolutionary | [→](docs/algorithms/genetic-algorithm.md) |
| Particle Swarm | `pso` | heuristic — swarm | [→](docs/algorithms/particle-swarm.md) |
| Cuckoo Search | `cs` | heuristic — nature-inspired | [→](docs/algorithms/cuckoo-search.md) |
| Flower Pollination | `fpa` | heuristic — nature-inspired | [→](docs/algorithms/flower-pollination.md) |

Exact algorithms find the provably optimal tour but have exponential complexity — do not use on more than ~20 cities. See [docs/benchmarks.md](docs/benchmarks.md) for a quality and speed comparison of all heuristics.


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

The component can also be loaded as a native MCP tool inside Claude via [Wassette](https://microsoft.github.io/wassette/latest/) — paste a `.tsp` file, ask Claude to list algorithms, pick one, and get a tour back without leaving the chat. See **[docs/wassette.md](docs/wassette.md)** for setup.

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

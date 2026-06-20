# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Teeline is a Traveling Salesman Problem (TSP) solver written in Rust. It reads city data in TSPLIB format (from stdin or file) and outputs the tour cost and ordered city IDs. A Piston-based progress window renders the route visually in a separate thread while solving.

## Commands

```bash
# Build (CLI binary is in teeline-cli workspace member)
cargo build -p teeline-cli           # debug
cargo build -p teeline-cli --release # optimized

# Or use Taskfile tasks
task build          # debug
task build:release  # optimized

# Run all tests
cargo test

# Run a specific test
cargo test test_coords_from_text_only_ints

# Run (debug binary is target/debug/teeline, release is target/release/teeline)
cat ./data/tsplib/berlin52.tsp | ./target/debug/teeline solve nn
./target/debug/teeline solve lk -i ./data/tsplib/berlin52.tsp
./target/debug/teeline --help
./target/debug/teeline solve --help
./target/debug/teeline convert --help

# Convert DiscOpt files to TSPLIB format
./target/debug/teeline convert -i ./data/raw/tsp_51_1 -o ./data/discopt/
./target/debug/teeline convert -i ./data/raw/ -o ./data/discopt/
```

## Architecture

**Entry point:** `src/main.rs` — parses CLI args with `clap`, reads TSPLIB data, then spawns two threads: one for the Piston progress/visualization window (`src/tsp/progress.rs`) and one for the solver. Output is written to stdout as `<total_distance> <optimized_flag>\n<city_id> <city_id> ...`.

**Library root:** `src/lib.rs` exposes `pub mod tsp` and a private `mod test`.

**Core types (all in `src/tsp/mod.rs`):**

- `Solvers` enum — lists every algorithm with string aliases (e.g. `"nn"` → `NearestNeighbor`, `"2opt"` → `TwoOpt`)
- `SolverOptions` — shared config struct passed to every solver (epochs, temperatures, mutation rate, etc.)
- `Solution` — holds the tour route (`Vec<usize>` of city IDs) and total distance
- `NearestResult` / `NearestResultItem` — returned by both KD-tree and distance matrix nearest-neighbor lookups

**Spatial data structures:**

- `src/tsp/kdtree.rs` — KD-tree for fast nearest-neighbor queries; `KDPoint` is the shared city/point type used everywhere
- `src/tsp/distance_matrix.rs` — brute-force O(n²) alternative; both expose the same `nearest(&pt, n)` interface

**Input:** `src/tsp/tsplib.rs` — state-machine parser for TSPLIB format; accepts `NODE_COORD_SECTION` and `DISPLAY_DATA_SECTION`; reads from file or stdin.

**Solvers** (each in its own file under `src/tsp/`):

| File | Algorithm | Alias |
| --- | --- | --- |
| `bellman_karp.rs` | Bellman–Held–Karp (exact, exponential) | `bhk` |
| `branch_bound.rs` | Branch and bound (exact) | — |
| `nearest_neighbor.rs` | Greedy nearest neighbor via KD-tree | `nn` |
| `two_opt.rs` | 2-opt local search | `2opt` |
| `stochastic_hill.rs` | Hill climbing with random restarts | — |
| `simulated_annealing.rs` | Simulated annealing | `sa` |
| `tabu_search.rs` | Tabu search | — |
| `genetic_algorithm.rs` | Genetic algorithm | `ga` |
| `particle_swarm.rs` | Discrete PSO (velocity-capped, linearly decaying inertia, NN-seeded) | `pso` |
| `cuckoo_search.rs` | Cuckoo Search via Lévy flights (k random 2-opt reversals, Bernoulli nest abandonment) | `cs` |
| `flower_pollination.rs` | Flower Pollination Algorithm (global Lévy-flight toward gbest; local ε-scaled cross-pollination) | `fpa` |
| `lin_kernighan.rs` | Lin-Kernighan style ILS: candidate-list 2-opt + double-bridge kicks | `lk` |
| `or_opt.rs` | Or-opt local search: relocates segments of 1–3 cities (best-improvement) | `or_opt` |
| `christofides.rs` | Christofides ≤1.5× approximation: MST + greedy matching + Eulerian shortcut | `christofides` |
| `gravitational_search.rs` | Gravitational Search Algorithm (Rashedi 2009): mass-weighted swap-velocity swarm (educational) | `gsa` |
| `fourier.rs` | Fourier-basis constructive solver: closed-curve gradient descent + argsort decode | `fourier` |

**Tests:**

- Unit tests live inline in each source file (`#[cfg(test)]`)
- Integration tests in `tests/` use the public library crate
- Test helpers (approximate float comparison, etc.) in `src/test/helpers.rs`

## Important Notes

- The CLI binary is produced by the `teeline-cli` workspace member (`cargo build -p teeline-cli`). The binary is named `teeline` (at `target/debug/teeline` and `target/release/teeline`). Any older `target/release/bin` is a stale artifact from before the workspace split — ignore it.
- `bellman_karp` and `branch_bound` are exact algorithms with factorial/exponential complexity — don't use them on datasets larger than ~30 cities.
- TSPLIB parsing normalizes all keys to uppercase and lowercases metadata values; city coordinates are stored as `f32`.
- `teeline convert` converts DiscOpt coordinate files (first line ignored, remaining lines are `x y` pairs) to TSPLIB EUC_2D format. It replaces the old `convert2tsplib.py` script.
- `download_data.sh` fetches benchmark datasets.

## teeline-web WASM Development Gotcha

`teeline-web/src/teeline-wasm.d.ts` is an **ambient module declaration** that TypeScript uses as the sole source of truth for `'teeline-wasm'` imports — it completely shadows `node_modules/teeline-wasm/teeline_wasm.d.ts`. **Any new WIT export must be added to this ambient file**, or tsc will fail with "Module 'teeline-wasm' has no exported member '...'".

After adding a new WIT export:

1. Build WASM: `cargo component build --manifest-path teeline-wasm/Cargo.toml --target wasm32-wasip2`
2. Run jco: `npx jco transpile ...` (see Taskfile)
3. **Update `teeline-web/src/teeline-wasm.d.ts`** with the new export's TypeScript type

## GitKB

This project uses GitKB for knowledge management.

@.kb/AGENTS.md

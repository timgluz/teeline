# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Teeline is a Traveling Salesman Problem (TSP) solver written in Rust. It reads city data in TSPLIB format (from stdin or file) and outputs the tour cost and ordered city IDs. A Piston-based progress window renders the route visually in a separate thread while solving.

## Commands

```bash
# Build
cargo build           # debug
cargo build --release # optimized

# Run all tests
cargo test

# Run a specific test
cargo test test_coords_from_text_only_ints

# Run (debug binary is named 'bin')
cat ./data/tsplib/berlin52.tsp | ./target/debug/bin nn
./target/debug/bin -i ./data/tsplib/berlin52.tsp two_opt
./target/debug/bin --help
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
|---|---|---|
| `bellman_karp.rs` | Bellman–Held–Karp (exact, exponential) | `bhk` |
| `branch_bound.rs` | Branch and bound (exact) | — |
| `nearest_neighbor.rs` | Greedy nearest neighbor via KD-tree | `nn` |
| `two_opt.rs` | 2-opt local search | `2opt` |
| `stochastic_hill.rs` | Hill climbing with random restarts | — |
| `simulated_annealing.rs` | Simulated annealing | `sa` |
| `tabu_search.rs` | Tabu search | — |
| `genetic_algorithm.rs` | Genetic algorithm | `ga` |

**Tests:**
- Unit tests live inline in each source file (`#[cfg(test)]`)
- Integration tests in `tests/` use the public library crate
- Test helpers (approximate float comparison, etc.) in `src/test/helpers.rs`

## Important Notes

- The binary is named `bin` (not `teeline`) — set by `[[bin]] name = "bin"` in `Cargo.toml`.
- `bellman_karp` and `branch_bound` are exact algorithms with factorial/exponential complexity — don't use them on datasets larger than ~30 cities.
- The visualization window is **off by default**. Pass `--gui` to open it. In headless/CI environments never pass `--gui`; the CI workflow (`cargo test`) avoids running the binary directly.
- TSPLIB parsing normalizes all keys to uppercase and lowercases metadata values; city coordinates are stored as `f32`.
- `convert2tsplib.py` converts raw coordinate lists to TSPLIB format; `download_data.sh` fetches benchmark datasets.

## GitKB

This project uses GitKB for knowledge management.

@.kb/AGENTS.md

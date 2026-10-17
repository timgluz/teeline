---
id: 019e2683-385c-7580-a0e5-45bb169cbea2
slug: context/immutable/architecture
title: "Architecture"
type: architecture
status: active
priority: medium
---

## Component Overview

```
src/
├── main.rs               # CLI (clap), reads input, spawns threads, dispatches solver
├── lib.rs                # pub mod tsp
└── tsp/
    ├── mod.rs            # Solvers enum, SolverOptions, Solution, CityTable, total_distance
    ├── tsplib.rs         # TSPLIB format parser (state machine, stdin or file)
    ├── kdtree.rs         # KD-tree; KDPoint is the shared city/point type
    ├── distance_matrix.rs# Brute-force O(n²) nearest-neighbour alternative
    ├── route.rs          # Route utility helpers
    ├── progress.rs       # Piston visualisation window (runs on main thread)
    ├── nearest_neighbor.rs
    ├── two_opt.rs
    ├── bellman_karp.rs   # Exact; practical limit ~20 cities
    ├── branch_bound.rs   # Exact; practical limit ~20 cities
    ├── simulated_annealing.rs
    ├── tabu_search.rs
    ├── stochastic_hill.rs
    └── genetic_algorithm.rs
tests/                    # Integration tests (use teeline crate)
data/tsplib/              # TSPLIB benchmark instances + .opt.tour files
```

## Threading Model

- **Main thread**: runs the Piston progress window (required by Piston's event loop)
- **Solver thread**: spawned via `thread::spawn`; communicates back via `mpsc` channel
- Channel: `Sender<ProgressMessage>` passed into solver; window reads via `try_recv` each frame

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `KDPoint` | `kdtree.rs` | City with `id: usize`, `x: f32`, `y: f32` |
| `Solution` | `mod.rs` | Tour route + total distance |
| `SolverOptions` | `mod.rs` | Shared config (epochs, temperatures, etc.) |
| `Solvers` | `mod.rs` | Enum of all available algorithms |
| `CityTable` | `mod.rs` | `HashMap<usize, KDPoint>` for O(1) city lookup |

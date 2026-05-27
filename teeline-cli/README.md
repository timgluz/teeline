# teeline-cli

Command-line interface for the Teeline TSP solver. Reads city data in TSPLIB format (from a file or stdin) and prints the best tour found.

## Build

```bash
cargo build -p teeline-cli          # debug
cargo build -p teeline-cli --release  # optimised
```

## Usage

```bash
# solve from a file
teeline solve nn -i data/tsplib/berlin52.tsp

# chain solvers in a pipeline
teeline solve classic -i data/tsplib/berlin52.tsp

# compare against a known-optimal tour
teeline solve ga -i data/tsplib/berlin52.tsp \
    --optimal-tour data/tsplib/berlin52.opt.tour

# list available solvers
teeline solvers
teeline solvers --heuristic --short
```

Full documentation is in the [root README](../README.md).

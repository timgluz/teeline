# teeline-qt

Qt 6 / QML desktop GUI for the Teeline TSP solver. Provides live visualization of the solver's progress, a solver picker with configuration panels, and a multi-stage pipeline builder.

## Requirements

- Qt 6.7 or later
- Rust toolchain (the solver engine is compiled into the binary via `qtbridge`)

## Build

```bash
cargo build -p teeline-qt          # debug
cargo build -p teeline-qt --release  # optimised
```

Or open `teeline-qt/` in Qt Creator and build from there.

## Features

- **Welcome screen** — drag-and-drop or browse for a `.tsp` file
- **Solver picker** — lists all available solvers grouped by category with descriptions and complexity
- **Config panel** — tuning parameters (epochs, temperatures, mutation probability, …) for solvers that support them
- **Visualization** — live canvas that animates the solver's improving tour
- **Pipeline builder** — chain multiple solvers; each stage warm-starts from the previous result

# teeline-qt

Qt 6 / QML desktop GUI for the Teeline TSP solver. Provides live visualization of the solver's progress, a solver picker with configuration panels, and a multi-stage pipeline builder.

## Requirements

- Qt 6.7 or later
- Rust toolchain (the solver engine is compiled into the binary via `qtbridge`)

## Build

```bash
# from the teeline-qt/ directory
cargo build           # debug
cargo build --release # optimised

# from the repo root (uses the manifest path)
cargo build --manifest-path teeline-qt/Cargo.toml
```

Or open `teeline-qt/` in Qt Creator and build from there.

## Tasks

A [`Taskfile.yml`](Taskfile.yml) is included with common workflows. Install [go-task](https://taskfile.dev/installation/), then from `teeline-qt/`:

```bash
task build            # compile debug binary
task run              # build and launch the app
task run:berlin52     # launch pre-loaded with berlin52.tsp
task run:release      # build release binary and launch
task check            # type-check without producing a binary
task lint             # clippy (warnings are errors)
task fmt              # auto-format with rustfmt
task ci               # check + lint + fmt:check (mirrors CI)
task clean            # remove build artefacts
```

From the **repo root** the same tasks are available under the `qt:` namespace:

```bash
task qt:build
task qt:run
task qt:run:berlin52
```

## Features

- **Welcome screen** — drag-and-drop or browse for a `.tsp` file
- **Solver picker** — lists all available solvers grouped by category with descriptions and complexity
- **Config panel** — tuning parameters (epochs, temperatures, mutation probability, …) for solvers that support them
- **Visualization** — live canvas that animates the solver's improving tour
- **Pipeline builder** — chain multiple solvers; each stage warm-starts from the previous result

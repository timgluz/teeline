# ADR-008: Qt 6/QML desktop GUI in `teeline-qt`

## Status

Accepted. Recorded 2026-11-14 to supersede ADR-004 and ADR-006 (the eframe/egui era).

## Context

The desktop GUI needs a richer surface than the eframe/egui progress window provided:
a solver picker with per-solver configuration panels, a multi-stage pipeline builder, and
live progress rendering. ADR-004 replaced piston with eframe, and ADR-006 extracted the
eframe code into `teeline-gui`; both kept the GUI as a Rust workspace member with a
`ProgressPlot` library surface.

Qt 6 offers a mature declarative UI (QML), a native desktop look on Linux, macOS and
Windows, and a clear separation between the UI and the Rust solver engine. The GUI is
platform-heavy (Qt runtime, `qtbridge` bindings) and must not leak into the pure solver
library.

## Decision

The desktop GUI is `teeline-qt`, a Qt 6/QML application built with
[`qtbridge`](https://github.com/qt/qtbridge-rust) and living in its own Cargo workspace
(`teeline-qt/`), not as a member of the root workspace.

- The `teeline` crate remains a pure solver library: no GUI toolkit, no eframe/egui,
  no Qt dependency.
- Solvers report progress by sending `ProgressMessage` values over an `mpsc` channel.
  `teeline-qt` owns the Qt event loop, drains the channel on a worker thread, and
  forwards updates to QML.
- `teeline-qt` requires Qt 6.7 or later at build time and is not a standalone
  distributable (`dist = false`). Build it from `teeline-qt/` with
  `cargo build --manifest-path teeline-qt/Cargo.toml` or `task qt:build`.

## Alternatives considered

| Alternative | Rejected because |
| --- | --- |
| Keep eframe/egui (`teeline-gui`) | Limited widget and declarative UI surface for configuration and pipeline building; no QML |
| Browser-only UI (`teeline-web`) | No native desktop app; requires a browser and a WASM round-trip |
| CLI only | No live visualisation or interactive pipeline builder |

## Consequences

- ADR-004 and ADR-006 are superseded; their `teeline-gui` crate no longer exists.
- The core library stays lean (no GUI dependency), which keeps WASM and server builds fast.
- Building or running the desktop app now requires the Qt 6 runtime; CI has dedicated Qt
  workflows (`build-qt.yml`, `build-qt-ci.yml`).
- `ProgressMessage` remains in the library because it is the typed contract between every
  solver and every consumer; moving it into the GUI would invert the dependency.

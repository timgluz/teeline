---
id: 019e5e85-ac0c-7b93-89f6-be7621c2175b
slug: decisions/adr-006-teeline-gui-extraction
title: "ADR-006: Extract eframe GUI into teeline-gui workspace member"
type: brief
status: active
tags: [adr, architecture, gui, workspace]
---

# ADR-006: Extract eframe GUI into teeline-gui workspace member

## Status

Accepted (implemented in feat/91-teeline-gui)

## Context

The `teeline` library previously embedded the eframe/egui progress visualisation behind a
`gui` Cargo feature:

- `default = ["gui"]` pulled in eframe for all consumers by default
- `teeline-wasm` had to use `default-features = false` to opt out
- The `[[bin]]` entry had `required-features = ["gui"]` — the binary could not be built
  without the GUI dependency at all
- `#[cfg(feature = "gui")]` guards spread across `src/tsp/mod.rs`
- Future richer visualisations would further bloat the core library

## Decision

Move all GUI code to a new `teeline-gui` workspace member:

- `src/tsp/progress_eframe.rs` → `teeline-gui/src/plot.rs`
- `src/main.rs` → `teeline-gui/src/main.rs`
- `teeline-gui` has `[lib]` (exposes `ProgressPlot`) + `[[bin]] name = "bin"` (the CLI)
- `teeline` becomes a pure solver library with no features and no eframe dependency
- `teeline-wasm`'s `default-features = false` becomes a no-op (harmless, left as-is)

## Why ProgressMessage stays in teeline

`ProgressMessage` is the typed `mpsc` channel contract that every solver sends to
(`Option<mpsc::Sender<ProgressMessage>>`). Moving it to `teeline-gui` would create a
circular dependency: `teeline` would need to import from `teeline-gui` to send messages,
while `teeline-gui` imports from `teeline` to run the solvers. The enum has zero GUI
dependencies and belongs in the library.

## Why teeline-gui is both [lib] and [[bin]]

The `[lib]` surface (`pub use plot::ProgressPlot`) lets future GUI experiments — a web
canvas renderer, a 3D view, a remote progress dashboard — depend on `teeline-gui` as a
library and reuse the channel-receiver pattern without touching the solvers. The `[[bin]]`
is the current CLI entry point, unchanged in behaviour.

## Trade-off vs feature-flag approach

| Aspect | Feature flag (old) | Separate crate (new) |
|--------|-------------------|---------------------|
| Consumer opt-out | `default-features = false` | depend on `teeline` directly |
| GUI always available | No (needs feature) | Yes (in teeline-gui) |
| Coupling | GUI code in core library | Clean separation |
| Complexity | One Cargo.toml | Two Cargo.toml files |

The feature-flag approach is simpler for small projects but couples the GUI to the library
and creates an awkward binary that requires its own feature to build. As the solver library
grows (WASM, server-side usage), the separation pays off.

## Consequences

- `cargo build -p teeline` compiles in seconds with no eframe/egui dependency
- `cargo build -p teeline-gui` brings in eframe and produces the CLI binary
- CI explicitly targets both crates with `-p teeline -p teeline-gui`
- BATS e2e tests are unaffected — binary still lands at `target/debug/bin`

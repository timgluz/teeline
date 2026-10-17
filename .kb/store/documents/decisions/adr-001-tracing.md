---
id: 019e272b-7c4f-7e62-96bb-99c0d8880b8a
slug: decisions/adr-001-tracing
title: "ADR-001: Structured logging via tracing"
type: brief
status: active
tags: [logging, tracing, architecture]
---

## Context

Teeline previously had no logging — only scattered `if options.verbose { println!() }` guards writing to stdout. Stdout is reserved for solution output (designed for shell piping), so mixing log output with solution data breaks downstream consumers.

## Decision

Add structured, levelled logging via the `tracing` crate (0.1) with `tracing-subscriber` (0.3) using an `EnvFilter`-based subscriber that writes to **stderr**.

- `--verbose` flag sets the default filter to `teeline=debug`
- Default run uses `teeline=info`  
- `RUST_LOG` environment variable overrides both

## Alternatives Considered

- **`log` + `env_logger`**: No spans, no structured fields — adequate for simple cases but doesn't support the call-graph tracing that `tracing` enables for future use.
- **`slog`**: More complex API requiring explicit loggers to be threaded through call sites; no macro-based API.

## Implementation

All `if options.verbose { println!() }` blocks replaced with:
- `tracing::info!` — solver start/end, improvements found
- `tracing::debug!` — per-iteration details (tick, swap, generation)
- `tracing::warn!` — plateau restarts in stochastic hill climbing

An `info_span!("solver", algorithm)` wraps the solver thread so all solver logs carry the algorithm name as structured context.

## Consequences

- Stdout stays clean; solution output is pipeable
- `RUST_LOG=teeline=debug ./bin nn -i file.tsp` shows all debug logs on stderr
- `./bin nn -i file.tsp 2>/dev/null` suppresses all logs
- `show_table()` in `bellman_karp.rs` remains behind `options.verbose` (prints a formatted ASCII table; out of scope for structured conversion)

## Files Changed

- `Cargo.toml` — added `tracing` and `tracing-subscriber` dependencies
- `src/main.rs` — subscriber init, replaced verbose prints, added solver span
- `src/tsp/simulated_annealing.rs`, `tabu_search.rs`, `stochastic_hill.rs`, `genetic_algorithm.rs`, `nearest_neighbor.rs`, `two_opt.rs`, `bellman_karp.rs`, `branch_bound.rs` — replaced verbose blocks with tracing calls

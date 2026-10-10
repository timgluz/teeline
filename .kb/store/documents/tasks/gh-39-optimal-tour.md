---
id: 019e2c4c-ce2d-7533-a9b2-a559a5be3922
slug: tasks/gh-39-optimal-tour
title: "Add --optimal-tour flag with gap comparison and visualization overlay (GH #39)"
type: task
status: completed
priority: medium
tags: [feature, cli, visualization, tsplib]
---

## Overview

Add a `--optimal-tour <FILE>` CLI flag so users can compare solver output against a known-optimal TSPLIB tour file. The feature has two outputs:

1. **stderr comparison block** — prints Optimal / Solver / Gap % after the solver finishes
2. **Visualization overlay** — renders the optimal route as a lime-green background layer in the egui window, with a color legend

## Goals

- Parse `.opt.tour` files (TSPLIB TOUR format) into an `OptTour` struct
- Compute optimal cost via the existing `DistanceMatrix::tour_length`
- Print gap comparison to stderr so stdout format is unchanged for pipeline/grader use
- Overlay optimal route in lime green on the egui visualization; add a color legend

## Acceptance Criteria

- [ ] `src/tsp/opt_tour.rs` created with `OptTour` struct and state-machine parser
- [ ] Parser handles KEY-VALUE header, TOUR_SECTION, multiple IDs per line, -1 terminator
- [ ] Parser validates TYPE=TOUR and dimension == route.len()
- [ ] `--optimal-tour <FILE>` flag added to CLI (clap)
- [ ] Gap comparison block printed to stderr; stdout unchanged without the flag
- [ ] Dimension mismatch produces a clear warning and skips comparison/overlay
- [ ] Optimal route displayed in lime green (50, 205, 50) in the visualization window
- [ ] Color legend rendered in bottom-left of window, hidden when no optimal tour loaded
- [ ] 4 unit tests pass in `opt_tour.rs`
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo test` passes

## Context

- `.opt.tour` files already exist in `data/tsplib/` for 32 benchmark instances
- `DistanceMatrix::tour_length(&[usize]) -> f32` is the cost function to use
- `DistanceMatrix::len()` returns distance-pair count, NOT city count — use `tsp_data.len()` instead
- Existing colors: dark forest green (34,139,34) = solver best; lime (50,205,50) = optimal overlay
- `ProgressPlot::new()` must be called **before** `send_progress()` — it initialises the global mpsc channel via `init_channels()`

## Implementation Plan

See `docs/superpowers/plans/2026-05-15-optimal-tour-comparison.md` for the full step-by-step plan.

## Progress Log

### 2026-05-15
- GH issue #39 reviewed; plan written covering 4 tasks (parser, visualization, CLI wiring, smoke test)
- Opus code review identified 3 blockers (distances.size→len, Route::new slice syntax, channel init ordering) and 6 warnings; all fixed in plan
- KB task created; feature branch `feat/39-optimal-tour` opened
- Implemented all 4 tasks; 4 extra issues fixed during execution (dead_code on parse_from_str, DistanceMatrix::len() count mismatch, duplicate error on bad path, Clippy map_flatten)
- PR #59 opened: https://github.com/timgluz/teeline/pull/59
- Visually verified in egui window: lime-green optimal overlay + dark-green solver best clearly distinct; legend renders correctly

## Completion Evidence

- Commits: 152054a, 997f875, ff01ce0, f2005a5
- `cargo test`: 111 passed, 0 failed
- `cargo clippy -- -D warnings`: clean
- PR #59 open on `feat/39-optimal-tour`
- Screenshot confirmed: both tours visible, legend shows all 4 colors

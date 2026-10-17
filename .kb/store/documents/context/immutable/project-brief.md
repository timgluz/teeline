---
id: 019e2683-3831-7cb0-95d3-6d480b6ff5be
slug: context/immutable/project-brief
title: "Project Brief"
type: brief
status: active
priority: medium
---

## Vision

Teeline is a **learning-oriented TSP solver** written in Rust. The goal is to implement a wide range of TSP algorithms correctly and idiomatically — not to compete with production solvers, but to understand the algorithms deeply through implementation.

## Core Purpose

- Implement classic and modern TSP algorithms in idiomatic Rust
- Correctness and code clarity matter more than raw performance
- Serve as a reference codebase for learning algorithm design in Rust

## Key Constraints

- Input: TSPLIB format (EUC_2D), from stdin or file
- Output: tour cost + ordered city IDs to stdout
- All algorithms slot into the existing `Solvers` enum dispatch pattern
- No `unsafe` code unless absolutely necessary
- Binary named `bin` (set in `Cargo.toml [[bin]]`)

## Foundational Decisions

- Pure Rust — no Python bridge, no JVM
- Piston visualisation window runs in a separate thread while solving
- KD-tree for spatial nearest-neighbour queries (already implemented)
- TSPLIB benchmark data in `data/tsplib/` for regression testing
- Concorde source (cloned at `../concorde`) used as algorithmic reference only — not linked
- License: academic-use algorithms referenced but not copied verbatim

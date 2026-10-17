---
id: 019e2683-3887-71e1-b766-b2f0d7df0b53
slug: context/extensible/product
title: "Product Context"
type: context
status: active
priority: medium
---

## Problem Being Solved

TSP is a canonical NP-hard combinatorial optimisation problem. Teeline exists to learn how different algorithmic approaches (exact, heuristic, metaheuristic, approximation) trade off solution quality against runtime — by implementing them from scratch in Rust.

## Primary User

Solo developer (Timo) using this as a learning project. The "user" is also the implementer; there are no external users or production deployments.

## Product Principles

- **Correctness first** — a slow correct solver beats a fast wrong one
- **Idiomatic Rust** — follow Rust conventions; Clippy warnings are errors in CI
- **Complete coverage** — aim to implement one representative algorithm from each major TSP algorithm family
- **Benchmark against reality** — TSPLIB known optima are the ground truth

## Algorithm Families Targeted

| Family | Status |
|--------|--------|
| Exact (exponential) | ✅ BHK, Branch-Bound |
| Exact (SMT reference) | 🔲 Z3 (#46) |
| Construction heuristic | ✅ Nearest Neighbour |
| Approximation (guaranteed bound) | 🔲 Christofides (#49) |
| Local search | ✅ 2-opt; 🔲 3-opt (#47), Or-opt (#48) |
| Advanced local search | 🔲 Lin-Kernighan (#45) |
| Metaheuristics | ✅ SA, Tabu, GA, Stochastic Hill |

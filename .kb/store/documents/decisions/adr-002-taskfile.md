---
id: 019e5e6b-6d53-7f42-8c59-d83e6001c699
slug: decisions/adr-002-taskfile
title: "ADR-002: Taskfile as task runner"
type: brief
status: active
tags: [adr, tooling, dx]
---

# ADR-002: Taskfile as task runner

## Status

Accepted

## Context

Common development commands (`cargo build`, `cargo test`, `cargo clippy`, BATS e2e tests,
WASM component build) were scattered across README, CLAUDE.md, and CI workflow files. There
was no single entry point for local development workflows, making onboarding slower and
leaving room for commands to drift out of sync across those documents.

## Decision

Add `Taskfile.yml` at the project root using [Taskfile](https://taskfile.dev/) (go-task)
as the task runner.

**Why Taskfile over alternatives:**

| Alternative | Why not chosen |
|-------------|----------------|
| **Makefile** | Tab-indentation requirement causes silent errors; `make` semantics are file-timestamp-based, not task-based; not natively cross-platform |
| **`just`** | Simpler syntax but fewer features; no dependency declarations between tasks; less mature ecosystem |
| **`cargo-make`** | Rust-native, but TOML verbosity is high; introduces another Cargo dependency; heavier install |
| **Taskfile** | YAML syntax; explicit `deps:` between tasks; cross-platform; self-documenting via `task --list`; already installed at `/home/timgluz/bin/task` |

## Tasks included

- `build`, `build:release`, `build:wasm` — compile artefacts
- `test`, `test:verbose`, `test:e2e`, `test:all` — unit, integration, and BATS e2e tests
- `lint`, `lint:fix` — Clippy
- `fmt`, `fmt:check` — rustfmt
- `check` — full CI-equivalent suite (build + test + lint + fmt:check)
- `run` — `cargo run` passthrough
- `bench:berlin52` — run all approximate solvers on berlin52 for quick quality comparison
- `clean` — remove build artefacts

## Consequences

- Developers must install `go-task` (`brew install go-task` / `go install ...@latest`).
- `task --list` is now the canonical way to discover available workflows.
- `task check` mirrors the CI suite and should be run before pushing.
- `bench:berlin52` task fixes a regression in the original issue proposal: the
  `--disable_progress` flag no longer exists; stderr is redirected; solver list updated
  to include `3opt pso cs fpa`; input path updated to `tests/fixtures/berlin52.tsp`.

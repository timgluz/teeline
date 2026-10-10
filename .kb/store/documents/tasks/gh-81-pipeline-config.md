---
id: 019e5085-8117-7f52-b375-3e528ff93520
slug: tasks/gh-81-pipeline-config
title: "feat: per-stage option overrides via TOML config file for pipeline"
type: task
status: completed
priority: high
tags: [feature, pipeline, config]
---

## Overview

Issue #81. The pipeline solver chains stages with a single shared `SolverOptions` — no way to tune individual stages. SA after 2-opt should use a lower `max_temperature` since 2-opt already cleaned up edge crossings, but the same options struct is passed to every stage. This feature adds a TOML config file so users can specify per-stage option overrides with a `[global]` baseline.

## Goals

- Add `PipelineStage { solver, options }` struct to pipeline.rs
- `pipeline::solve()` accepts `&[PipelineStage]` instead of `&[Solvers]`; `progress_tx` injected at runtime
- `OptionsProvider` trait unifies CLI args and TOML tables (`provide(&self, base) -> Result<SolverOptions, String>`)
- `src/config.rs` (private, binary-only): `TomlTableProvider`, `load_pipeline_config()`, `apply_global()`, validation
- `--config <PATH>` flag on `pipeline` subcommand only; `solve` unchanged; `--config` and `--steps` mutually exclusive
- `configs/classic-tuned.toml` and `configs/thorough-tuned.toml` example configs
- Integration tests (shell-out) covering happy path, mutual exclusion, missing file, validation errors, solve rejection

## Design Decisions

- **No auto-discovery** — config only loaded when `--config` is explicitly passed; avoids stale-file confusion
- **Priority order**: stage fields > CLI flags > `[global]` section > `SolverOptions::default()`
- **Provider fold**: `[TomlGlobalProvider, CliArgsProvider].try_fold(default, provide)` — CLI wins because it runs last
- **`progress_tx` and `initial_tour`** are runtime-injected in `pipeline::solve()`, not in TOML
- **Irrelevant stage fields** (e.g. `max_temperature` on `nn`) → collected as soft warnings, emitted after tracing init
- `SolverOptions` not touched with serde derives (contains `mpsc::Sender`)

## Acceptance Criteria

- [x] `serde` + `toml` added to Cargo.toml
- [x] `PipelineStage` struct added to `src/tsp/pipeline.rs` with `#[derive(Clone, Debug)]`
- [x] `pipeline::solve()` updated: takes `&[PipelineStage]` + `progress_tx`, injects both per stage
- [x] `OptionsProvider` trait in `src/config.rs`; `TomlTableProvider` implements it
- [x] `CliArgsProvider` in `src/main.rs` implements `OptionsProvider`; replaces `solver_options_from_args` free fn
- [x] `load_pipeline_config(source, global_base)` returns `(Vec<PipelineStage>, Vec<String>)` — stages + warnings
- [x] `apply_global(source, base)` convenience fn for `[global]` pre-application
- [x] Validation: unknown keys, empty stages, cooling_rate bounds, temperature ordering, mutation_probability range
- [x] Irrelevant-field warnings collected and returned (not logged inside config.rs)
- [x] `--config` flag on `pipeline_cmd` only; `--steps` and `--config` mutually exclusive
- [x] `run_pipeline()` handles both paths; warnings emitted after tracing subscriber init
- [x] `configs/classic-tuned.toml` and `configs/thorough-tuned.toml` created with comments
- [x] 8 integration tests pass; 263 total tests pass; clippy clean

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Added `serde`, `toml` |
| `src/tsp/pipeline.rs` | `PipelineStage`, updated `solve()`, updated tests |
| `src/config.rs` | New — `OptionsProvider`, `TomlTableProvider`, `load_pipeline_config`, `apply_global`, 15 unit tests |
| `src/main.rs` | `CliArgsProvider`, `--config` flag, `run_pipeline()`, `run_as_pipeline_stages()` refactor |
| `configs/classic-tuned.toml` | New example config |
| `configs/thorough-tuned.toml` | New example config |
| `tests/pipeline_config_test.rs` | New — 8 binary integration tests |

## Progress Log

### 2026-05-22
- Planned in conversation (issue opened by timgluz)
- Opus code review applied: provider trait, priority chain fix, progress_tx type, warning collection
- Implementation complete on branch `feat/81-pipeline-config`
- 263 tests pass, clippy clean, manual smoke-test verified

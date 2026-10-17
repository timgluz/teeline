---
id: 019fbea3-7db3-72c3-838e-fbf13faf09a3
slug: tasks/gh372-lk-som-pipeline-toml
title: "GH #372: [stage.lk]/[stage.som] pipeline TOML support"
type: task
status: completed
priority: medium
tags: [solver, lin-kernighan, som, pipeline-config, 371/PR]
---

## Overview

Follow-up from #371/PR #373 (merged `1a72647`). `LKOptions::from_toml`/`SOMOptions::from_toml`
are fully implemented but `TomlTableProvider::provide` in `src/config.rs` doesn't
recognize `"lk"`/`"som"` as valid `[[stage]]` sub-table keys — mirrors the exact gap
`[stage.fourier]` closed in #371. Both solvers already have CLI flags; this is
TOML-config-only.

GitHub issue: https://github.com/timgluz/teeline/issues/372

## Goals

- `[stage.lk]`/`[stage.som]` pipeline TOML tables parse and apply
- `[stage.heuristic]` under lk/som stages becomes a hard error (currently silent no-op),
  matching fourier's treatment — verified zero blast radius (no existing fixture/config
  uses lk/som TOML stages)
- Fix real pre-existing doc bugs found while restructuring `lin-kernighan.md`'s Options
  table (wrong defaults, stale "unused" claim, stale benchmark numbers) — see plan doc

## Acceptance Criteria

- [x] `TomlTableProvider::provide` gets `"lk"`/`"som"` arms (src/config.rs)
- [x] `load_pipeline_config` belongs-check array gets lk/som tuples (ordered after
      fourier, before heuristic) + heuristic-exclusion updated
- [x] Unit tests: lk/som sub-table works, heuristic-exclusion errors, wrong-solver
      rejection, validate()-propagation (max_depth=0, learning_rate=1.5 errors) — 8 new
      tests, all passing
- [x] Integration tests: pipeline_config_test.rs (lk max_depth, som epochs applied),
      new fixtures pipeline_lk.toml/pipeline_som.toml (som epochs=500, not default 100k),
      pipeline.bats e2e cases (2 new, 13/13 total passing)
- [x] docs/algorithms/lin-kernighan.md: restructured Options table with CORRECT verified
      defaults (epochs=100, platoo_epochs=10, n_nearest=5, max_depth=5 — NOT the
      previously-wrong 10000/500/3), fixed stale max_depth "unused" claim, fixed stale
      benchmark (0.0% gap not ~8-9%), --max_depth->--max-depth typo fix, [stage.lk]
      TOML example added
- [x] docs/algorithms/som.md: CLI-flag column added to existing (already-correct) table,
      [stage.som] TOML example added
- [x] `cargo build/test --workspace` (302 lib tests + all integration suites),
      `cargo fmt -- --check`, `cargo clippy --workspace -- -D warnings` all pass
- [x] Manual verification: [stage.lk]/[stage.som] overrides apply and pipeline resolves
      without "unknown field" error; [stage.heuristic] under lk now hard-errors
      ("`[stage.heuristic]` is not valid for this solver") confirming the belongs-check
      exclusion took effect

## Context

- Plan reviewed independently by an Opus pass before implementation — found 4 real
  factual defects in the docs section (wrong LK defaults, stale max_depth claim,
  inverted epochs=0 sentinel claim, fabricated public ADR citation), all corrected in
  the final plan. Rust-side config.rs plan was confirmed correct as originally written.
- Full plan: /home/timgluz/.claude/plans/please-take-a-look-fluttering-brook.md
- Relevant code: `src/config.rs` (`TomlTableProvider`, `load_pipeline_config`),
  `src/tsp/mod.rs` (`LKOptions`, `SOMOptions`, `HeuristicOptions::validate`)
- Parent: [[tasks/gh371-fourier-cli-flags]] (established the exact pattern being mirrored)
- ADR reference (internal only, not cited in public docs): GitKB
  `.kb/workspaces/main/decisions/adr-005-validation-approach.md`

## Progress Log

### 2026-08-01

Branch `feat/gh372-lk-som-pipeline-toml` created off up-to-date master (`1a72647`).
Plan mode research + design brainstorm (1 round of AskUserQuestion) + Opus review pass
completed. Starting implementation.

### 2026-08-01 (later)

`src/config.rs` changes done (TomlTableProvider lk/som arms, belongs-check array,
heuristic exclusion), 8 new unit tests all passing. Fixtures `pipeline_lk.toml`/
`pipeline_som.toml` + 2 integration tests in `pipeline_config_test.rs` + 2 bats e2e
cases, all passing. Docs restructured: `lin-kernighan.md` (corrected wrong defaults,
fixed stale max_depth/benchmark claims, added TOML example) and `som.md` (CLI-flag
column, TOML example). `cargo fmt`/`test --workspace` clean (302 lib tests + all
integration suites).

While answering a user question about whether LK uses a KD-tree for its candidate
list (it doesn't — confirmed brute-force `build_candidates()` in
`src/tsp/lin_kernighan.rs:12-32`), filed follow-up
[issue #374](https://github.com/timgluz/teeline/issues/374) for a KD-tree migration,
explicitly scoped to require measured wall-clock improvement (not just complexity
analysis) before shipping, per #370's own KD-tree lesson. Also surfaced a likely-stale
`CLAUDE.md` claim that `nearest_neighbor.rs` uses a KD-tree (it uses `DistanceMatrix`)
— not filed as a separate issue yet.

### 2026-08-01 (committed)

All acceptance criteria met. Committed as `c9c75dd` on `feat/gh372-lk-som-pipeline-toml`
(off master `1a72647`). Pre-commit hooks (cargo fmt, clippy, markdownlint, taplo) all
passed after one markdownlint autofix (Benchmark table alignment style in
lin-kernighan.md — switched to the repo's loose/unaligned table-dash convention already
used in docs/benchmarks.md). Not yet pushed or opened as a PR.

## Completion Evidence

- Commit: `c9c75dd` on branch `feat/gh372-lk-som-pipeline-toml`
- Test results: `cargo test --workspace` all green (302 lib tests + all integration/e2e
  suites), `cargo clippy --workspace -- -D warnings` clean, `cargo fmt -- --check` clean,
  `./tests/bats/bin/bats tests/e2e/pipeline.bats` 13/13 passing
- Manual verification: `pipeline --config` with `[stage.lk]`/`[stage.som]` overrides
  resolves and runs to completion; `[stage.heuristic]` under an lk stage now errors
  cleanly with `config: stage 0 (lk): \`[stage.heuristic]\` is not valid for this solver`
- Follow-up filed: [issue #374](https://github.com/timgluz/teeline/issues/374) (LK
  KD-tree candidate-list migration, discovered mid-task)

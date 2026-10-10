---
id: 019fbe77-e1b2-7611-a40a-3b8abccc499f
slug: tasks/gh371-fourier-cli-flags
title: "GH #371: Fourier k_max/m CLI + pipeline TOML flags"
type: task
status: completed
priority: medium
tags: [solver, fourier, cli, pipeline-config]
---

## Overview

Expose `FourierOptions.k_max`/`.m` (already fully implemented — defaults, `validate()`,
`from_toml()`) as CLI flags (`--k-max`/`--m`) and pipeline TOML `[stage.fourier]`
overrides. Currently only reachable via the REST API's `configs.fourier`. Root cause:
`FourierOptions::from_cli()` only reads `epochs`, and `CliArgsProvider::provide` has no
`Solvers::Fourier` match arm at all (falls to generic heuristic fallback, so CLI Fourier
runs always use defaults regardless of flags). Separately, `TomlTableProvider` doesn't
recognize `fourier` as a valid `[[stage]]` sub-table key.

GitHub issue: https://github.com/timgluz/teeline/issues/371

## Goals

- `--k-max`/`--m` CLI flags on `solve fourier` / `pipeline --steps=...,fourier`
- `[stage.fourier]` pipeline TOML support
- Document the quality/wall-time tradeoff (k_max is the dominant quality lever per the
  scaling experiments, not m) in CLI help text and `docs/algorithms/fourier.md`
- No auto-scaling default, no new wall-time guard — manual flags + documentation only
  (design decisions made with user before implementation, see plan doc)

## Acceptance Criteria

- [x] `tuning_args()` in `teeline-cli/src/main.rs` gets `k_max`/`m` args (long:
      `--k-max`/`--m`)
- [x] `CliArgsProvider::provide` gets a `Solvers::Fourier` arm
- [x] `FourierOptions::from_cli` parses `k_max`/`m` (mirrors `LKOptions::from_cli`)
- [x] `TomlTableProvider::provide` gets a `"fourier"` arm; belongs-check array + heuristic
      exclusion list updated in `load_pipeline_config`
- [x] `docs/algorithms/fourier.md` gets tuning guidance + `--k-max`/`[stage.fourier]`
      examples. `docs/benchmarks.md` gets a cross-reference note instead of a full a280
      row — that doc is strictly scoped to berlin52 under one fixed methodology (release
      build, `GNU time -v` CPU/RSS), and the a280 numbers weren't measured under it, so a
      row with fabricated/missing columns would have been misleading.
- [x] Unit tests: `FourierOptions::from_cli` (happy path, bad int, epochs=0 error),
      `CliArgsProvider`-level test for the new match arm, `TomlTableProvider` fourier
      sub-table test, belongs-check exclusion test (plus a symmetrical
      fourier-sub-table-on-nn-errors test not originally listed)
- [x] Integration tests: `tests/pipeline_config_test.rs` fourier stage test,
      `tests/e2e/pipeline.bats` fourier config fixture
- [x] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` pass
- [x] Manual verification: a280 gap improves with `--k-max 32` (+103.38% → +24.57%,
      matching the KB experiment), `--epochs 0` now errors (confirmed intended change),
      `--usage-spec` still renders, bad-integer flags error cleanly, `--k-max` on a
      non-fourier solver is silently ignored as expected

## Context

- Plan reviewed independently by an Opus pass before implementation (verdict: approve
  with minor fixes — all incorporated into acceptance criteria above)
- Design decisions (manual flags not auto-scaling, no wall-time guard, TOML in scope,
  `--k-max`/`--m` kebab-case naming) made via AskUserQuestion with the user before
  planning
- Relevant code: `src/tsp/mod.rs` (`FourierOptions`), `teeline-cli/src/main.rs`
  (`CliArgsProvider`, `tuning_args`), `src/config.rs` (`TomlTableProvider`,
  `load_pipeline_config`)
- Parent: [[tasks/fourier-scaling-optimizer-experiments]] (experiment data this is
  based on)
- Follow-up filed: https://github.com/timgluz/teeline/issues/372 (lk/som pipeline TOML
  support, same gap, out of scope here)
- Discovered but out of scope: `teeline-api`'s `make_app_options()` never calls
  `.validate()` on any solver config (API-wide gap, not fourier-specific) — worth its
  own follow-up issue, not filed yet

## Progress Log

### 2026-08-01

Branch `feat/gh371-fourier-cli-flags` created off master. Plan mode research + design
brainstorm completed (2 rounds of AskUserQuestion with user, 1 Opus review pass). Filed
follow-up issue #372 for lk/som pipeline TOML. Starting implementation.

### 2026-08-01 (later)

Implementation complete, all acceptance criteria met. Changes: `src/tsp/mod.rs`
(`FourierOptions::from_cli` parses k_max/m + 3 new tests), `teeline-cli/src/main.rs`
(`--k-max`/`--m` args, `Solvers::Fourier` match arm, 1 new test), `src/config.rs`
(`"fourier"` TomlTableProvider arm, belongs-check + heuristic-exclusion updates, 3 new
tests), `tests/pipeline_config_test.rs` + `tests/e2e/pipeline.bats` + new fixture
`tests/fixtures/pipeline_fourier.toml` (integration coverage), `docs/algorithms/fourier.md`
(tuning-guidance section, options table CLI-flag column, usage examples),
`docs/benchmarks.md` (cross-reference note).

`cargo test --workspace` (294+ lib tests plus all integration/e2e suites) and
`cargo clippy --workspace -- -D warnings` both clean. Manual CLI verification confirmed
gap improvement (+103.38% → +24.57% on a280 with `--k-max 32`, matching prior KB
experiment data), clean error messages for bad integers and `--epochs 0`, correct no-op
on non-fourier solvers, and unbroken `--usage-spec` generation.

Committed as `bb49f48` on `feat/gh371-fourier-cli-flags` (pre-commit hooks — cargo fmt,
clippy, markdownlint, taplo — all passed after one taplo autofix on the new TOML
fixture). Not yet pushed or opened as a PR.

### 2026-08-01 (merged)

PR #373 merged by the user into `master` as squash commit `1a72647`. Local `master`
fast-forwarded, tree clean. Merged local branch `feat/gh371-fourier-cli-flags` deleted
(content confirmed present on `origin/feat/gh371-fourier-cli-flags` and in the squash
commit before deletion). GH #371 fully resolved end-to-end.

### 2026-08-01 (code review)

`/code-review high --fix` found and fixed 2 documentation-accuracy bugs (no
functional/correctness issues in the Rust code): a self-contradicting sentence in
`docs/algorithms/fourier.md` (said only `k_max`/`m` have CLI flags, right below a table
listing `--epochs` too) and a ~10x wall-time mismatch between the `--k-max` help text
(~45s/~0.7s) and the docs table (4.25s/0.19s) for the identical a280 `k_max=32`
scenario. Both corrected to match the docs table's verified numbers. Rebuilt, re-linted
(clean), committed as `eb13c4a`.

## Completion Evidence

- Commits: `bb49f48` (implementation), `eb13c4a` (code-review doc fixes) on branch
  `feat/gh371-fourier-cli-flags`, squash-merged to `master` as `1a72647` via
  [PR #373](https://github.com/timgluz/teeline/pull/373) (closes #371)
- Test results: `cargo test --workspace` all green, `cargo clippy --workspace -- -D
  warnings` clean, `./tests/bats/bin/bats tests/e2e/pipeline.bats` 11/11 passing
- Manual verification: a280 `--k-max 32` gap +24.57% (KB experiment predicted +24.6%);
  `--k-max abc`/`--m abc` → clean parse errors; `--epochs 0` on fourier → clean
  validation error; `solve nn --k-max 32` → silently ignored as expected; `--usage-spec`
  renders both new flags correctly

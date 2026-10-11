---
id: 01a129e9-f1ca-7460-820b-95cd828a489e
slug: tasks/ci-cli-e2e-bats-workflow
title: "ci(bench): dedicated GitHub Action for the CLI e2e (BATS) suite"
type: task
status: draft
priority: medium
tags: [ci, bats, testing, tooling]
---

## Overview

The BATS suite (`tests/e2e/`, 85 cases as of writing) does run in CI, but only as a step inside
the **Rust** workflow (`.github/workflows/rust.yml`, step `Run e2e tests (BATS)` →
`./tests/bats/bin/bats tests/e2e/`). That entangles a shell/CLI suite with a Rust workflow and
gives it the Rust workflow's trigger — which is both too broad and too narrow:

- `pull_request: branches: [master]` means a PR whose base is a **feature branch gets no run at
  all**. Verified: PR #570 (stacked on another feature branch) has no `Rust` run for its head
  commit, while PR #569 (base `master`) ran the BATS step and passed.
- Conversely, a change to `scripts/` currently triggers the full Rust job — build, `cargo test`,
  BATS, clippy, and a ~2.5 minute coverage run — when only the shell suite is relevant.

The goal is a dedicated, cheap CLI e2e workflow so script and test changes are verified without
pulling in the Rust pipeline, and so the suite is not tied to a PR's base branch.

## What already exists — do not redo these

Recorded so this task is not spent rediscovering them:

- **BATS is already installed as git submodules**, exactly as the upstream tutorial's
  "Quick installation" prescribes (`https://github.com/bats-core/bats-core.git` →
  `tests/bats`, plus `bats-support` and `bats-assert` under `tests/test_helper/`). Pinned at
  bats-core v1.13.0-33, bats-assert v2.2.4-2, bats-support v0.3.0-37.
  Tutorial "Quick installation": https://bats-core.readthedocs.io/en/stable/tutorial.html
- **A Taskfile target already runs them**: `test:e2e` in the root `Taskfile.yml`, with
  `deps: [build]` because most e2e files need the debug binary.
- Every workflow that needs the submodules already passes `submodules: true` / `recursive`.

## Goals

- A dedicated workflow for the CLI e2e suite, independent of the Rust workflow
- It runs for changes that can affect the CLI or its tests, and not for unrelated ones
- The suite runs exactly once per PR — not once here and once in `rust.yml`
- A documented way to run the same suite locally through Task

## Acceptance Criteria

- [ ] A workflow (for example `.github/workflows/cli-e2e.yml`) runs `tests/e2e/` on
      `pull_request` and on `push` to `master`
- [ ] It checks out submodules, or BATS is not present and every test errors
- [ ] It builds the debug binary first — `solve`, `solvers`, `convert` and `pipeline` need
      `target/debug/teeline`; only `publish-benchmarks.bats` runs without it
- [ ] `paths` covers what can affect the suite: `scripts/**`, `tests/**`, `bench/**`,
      `teeline-cli/**`, `src/**` (the CLI links the library), `Taskfile.yml`, `docs/problems/**`
      and `docs/benchmarks/schema.md` (the publish tests read both), and the workflow file itself
- [ ] The suite is removed from `rust.yml`, so it does not run twice for one PR — decide and
      record whether the Rust job keeps it for Rust-only changes
- [ ] It does **not** require the coverage tooling, so it finishes far faster than the Rust job
- [ ] The workflow triggers for a PR whose base is a feature branch (the stacked-PR case that
      currently gets nothing)
- [ ] `shellcheck` clean if the workflow adds shell; the workflow YAML lints (actionlint if the
      repo adopts it, otherwise by review)
- [ ] Documented in `AGENTS.md` or the Taskfile description, so the next person does not add the
      suite to a Rust workflow again

## The Taskfile question, to settle first

The ask was to add the task "into Taskfile of teeline-cli", but **`teeline-cli/` has no
Taskfile** — only the root `Taskfile.yml`, `teeline-qt/Taskfile.yml` and `teeline-web/Taskfile.yml`
have one. So there are two readings, and the first step is to pick one:

1. **Create `teeline-cli/Taskfile.yml`** following the `teeline-qt` precedent, with an `e2e` task
   that builds the binary and runs `tests/e2e/`, so the suite lives next to the crate it tests.
   This is probably what was meant, and it matches the existing per-crate convention.
2. **Extend the root `Taskfile.yml`**, which already has `test:e2e`; then this criterion is
   already met and only the `desc` needs work.

Either way the root `test:e2e` should keep working, since it is the documented entry point.

## Notes

- The suite is fast: 85 cases run in well under a minute locally, most of it the solve/solvers
  cases. It needs no network — `publish-benchmarks.bats` deliberately uses `--dry-run
  --no-fetch-previous` and a local HTTP server.
- `tests/e2e/publish-benchmarks.bats` also reads `bench/budgets.json`, `bench/solvers.json` and
  `scripts/publish-benchmarks.sh`, so `bench/**` belongs in `paths`.
- Do not make this workflow run the whole e2e directory for `teeline-web/**` or
  `teeline-api/**` changes; those have their own workflows.
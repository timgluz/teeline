---
id: 01a12a78-a413-7033-9744-876e15d07cd7
slug: tasks/migrate-jaq-for-toml-queries
title: "chore(tests): use jaq for TOML queries in shell and BATS, and wire the tooling into CI"
type: task
status: draft
priority: low
tags: [testing, tooling, ci]
---

## Overview

Shell and BATS code that needs to read a TOML file currently shells out to a `python3` heredoc
with `tomllib`. That is verbose enough to hide the assertion it wraps: the budget-agreement check
in `tests/e2e/bench-matrix.bats` is a 20-line Python block whose actual claim is one equality.

`jaq` — the Rust implementation of the jq language — reads TOML natively, so the same check
becomes a one-liner. It selects the format by extension (`.toml`), and also accepts `--from toml`,
plus the `fromtoml`/`totoml` filters and `--to toml`.

Verified against the real files:

```console
$ jaq -c '.budgets' bench/matrix/tier-a.toml
{"exact":60,"local":300,"meta":300,"constructive":30,"utility":60}

$ [ "$(jq -Sc '.budgets | map_values(.seconds)' bench/budgets.json)" \
    = "$(jaq -Sc '.budgets' bench/matrix/tier-a.toml)" ] && echo agree
agree
```

## What it replaces

`tests/e2e/bench-matrix.bats`:

- **"the published budgets match the tier the numbers were measured under"** — a 20-line Python
  heredoc becomes the two-line comparison above. `-S` on both sides, because JSON key order and
  TOML file order are not the same thing and the comparison must not depend on either.
- **"every instance in the tier parses"** — Python enumerates the instance names; with jaq that is
  `jaq -r '.instances | keys | join(" ")'`. The parse check itself still needs the binary.
- Separately, and not part of the jaq migration: the file has **10 hand-built TOML fixtures**
  written as heredocs. A `tier()` helper would cut most of that repetition, and it needs no new
  dependency — worth doing on its own.

## Scope: queries, not the production path

This is about *shell and test* queries. `scripts/bench-matrix-plan.py` keeps `tomllib`: it is
program logic that builds the cell plan, not a query, and shelling out to jaq per sweep would be
slower and would add a runtime dependency to the harness. Do not migrate it.

## The blocker: CI installs nothing

`.github/workflows/rust.yml` — the job that runs the BATS suite — installs no tools. It relies on
the runner image, where `jq` and `python3` exist but `jaq` does not. `jaq` is also absent from
`.mise.toml`. So a test using jaq fails in CI today; the tooling has to be wired up first.

The pattern already exists in the repo: `build-api.yml` uses `jdx/mise-action@v5.0.1` to install
`hurl`. So the change is `cargo:jaq` in `.mise.toml` plus a `mise-action` step in `rust.yml`.

**Record the cost first:** `mise-action` lands on the Rust workflow's critical path, so it runs for
every PR that touches Rust. Confirm that is acceptable, or install only jaq to keep it cheaper.

## Side benefit worth taking while wiring this up

`taplo-cli` is already declared in `.mise.toml`, and `.githooks/pre-commit` already calls taplo on
staged TOML — but it silently skips with *"taplo not installed — did you forget to run mise
install?"*. So the tier file has never been formatted or linted by the tool the repo chose for it.
Installing the declared mise tools turns that on, which is the `mise install` the hook has been
asking for.

## Acceptance Criteria

- [ ] `cargo:jaq` added to `.mise.toml` (or jaq installed another way that CI will actually run)
- [ ] `rust.yml`'s `build` job installs it, following the `mise-action` pattern from `build-api.yml`
- [ ] The BATS suite passes in CI with the jaq-based tests — checked on the runner, not only locally
- [ ] The budget-agreement check compares with `-S` on both sides, so key order cannot make it flaky
- [ ] `scripts/bench-matrix-plan.py` still uses `tomllib` (unchanged)
- [ ] taplo runs on the tier file and the pre-commit no longer skips it; `bench/matrix/tier-a.toml`
      is formatted
- [ ] The CI time cost is measured and stated in the PR, since it applies to every Rust PR
- [ ] A `tier()` helper in `bench-matrix.bats` replaces the hand-built TOML fixtures

## Notes

- jaq is a drop-in for jq with extra formats, but it is **not** installed here by default — it lives
  at `~/.cargo/bin/jaq` (v3.1.1) from a manual `cargo install`, which is exactly why CI would not
  have it.
- Keep the comparison behavioural ("these two must agree") rather than reimplementing the parse:
  jaq reads the file the harness reads, so the test cannot drift from the real config format.
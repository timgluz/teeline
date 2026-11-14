# Backlog

Pending work migrated from the former task tracker. This is the canonical list of
open items; completed tasks are recorded in git history and pull requests, not here.

Pick an item directly, or open a GitHub issue when it needs discussion or assignment.
Nothing here is a commitment — the list exists so ideas are not lost.

| # | Item | Area | Priority | Status |
| --- | --- | --- | --- | --- |
| 1 | [Pipeline solver](#1-pipeline-solver) | solver | high | likely complete — verify |
| 2 | [Tier A benchmark campaign](#2-tier-a-benchmark-campaign) | benchmarks | high | mostly complete |
| 3 | [Benchmark docs and schema update](#3-benchmark-docs-and-schema-update) | docs | medium | open |
| 4 | [Benchmark regression history](#4-benchmark-regression-history) | benchmarks | low | open |
| 5 | [Single-solver benchmark skill](#5-single-solver-benchmark-skill) | tooling | medium | open |
| 6 | [Lin-Kernighan shared plateau option](#6-lin-kernighan-shared-plateau-option) | solver | medium | open |
| 7 | [DuckDB blog post](#7-duckdb-blog-post) | blog | medium | open |
| 8 | [Dedicated CLI e2e workflow](#8-dedicated-cli-e2e-workflow) | ci | medium | open |
| 9 | [jaq for TOML queries](#9-jaq-for-toml-queries) | testing | low | open |
| 10 | [Missing optimal tours](#10-missing-optimal-tours) | data | medium | open |
| 11 | [Progress listener seam](#11-progress-listener-seam) | solver | low | open |
| 12 | [Passkeys blog — technical recipe](#12-passkeys-blog--technical-recipe) | blog | medium | waiting |

## 1. Pipeline solver

**Likely complete — verify and close.** The pipeline is documented and implemented:
`teeline pipeline --steps=...`, the `fast` / `classic` / `thorough` presets, and the
`nn`/`shuffle` auto-expansion for local-search and stochastic solvers. The original task
predates the current API (`solve_problem` / `AppOptions` instead of `SolverOptions`).

**Next step:** confirm every original acceptance criterion against the current README and
CLI behaviour, then drop this item.

## 2. Tier A benchmark campaign

The Tier A sweep ran and published: `bench/v2/index.json` reports 22 solvers and 12
problems, and `bench/runs/tier-a.jsonl` holds 1,245 rows with a per-row `status`
(1,235 `ok`, 10 `timeout`).

**Still to confirm:**

- Whether the "both budget-limited and to-convergence runs per cell" criterion is stale:
  the raw configs are `budget` and `exact`, with no convergence config.
- `bench/solvers.json` against the 22 solvers actually present.

**Next step:** reconcile the remaining criteria against the committed data and close, or
scope the missing convergence run as a new item.

## 3. Benchmark docs and schema update

Benchmark documentation is inconsistent with the current data contract.

- `docs/benchmarks.md` still holds pre-fix numbers and is slated for retirement.
- `docs/benchmarks/schema.md` documents a single dataset-level `git_commit`, which the
  per-solver provenance change invalidates.
- Algorithm docs may state termination behaviour changed by the plateau work.

**Next step:** retire the stale page (with redirects/updated links), move the schema to
per-solver `measurement` blocks plus per-family budgets, and add a methodology page
covering tiers, timeouts, non-determinism and the cross-family comparison rule.

The stable rationale is in [`docs/benchmarks/design.md`](benchmarks/design.md).

## 4. Benchmark regression history

Per-release snapshots and the criteriondb mirror exist so a regression can be compared
against an earlier release, but nothing consumes that history yet.

**Next step:** decide the presentation — per-algorithm history, a dedicated regressions
page, or queryable-only via criteriondb — then verify the retained snapshots are
sufficient. Do not plot incomparable budgets as one line.

## 5. Single-solver benchmark skill

The cadence is manual and differential: at each release, re-measure only the algorithms
that changed. That loop warrants a project-local `benchmark-solver` skill.

**Next step:** add `.agents/skills/benchmark-solver/SKILL.md` that builds the release
binary, runs `scripts/bench-matrix.sh --solver <id>`, folds the rows in, publishes to R2
and verifies the live data. It must refuse to publish when shared code
(`distance_matrix`, `kdtree`, `tsplib`, `comparison`, `mod`, `pipeline`, `graph`) has
changed since the other solvers were measured — otherwise it mixes incomparable numbers
under one version label.

## 6. Lin-Kernighan shared plateau option

`lin_kernighan` is the last solver not using the shared plateau option. It uses
`HeuristicOptions::platoo_epochs` (historical misspelling, default 10) as a restart
threshold. Migrating to `stagnation_epochs` retires the duplicate field and the
`--plateau_epochs` flag.

**Not mechanical:** the defaults differ (`platoo_epochs` 10, `stagnation_epochs` 0 =
disabled). Decide explicitly whether to preserve LK's early-stop default by setting
`LKOptions::default().stagnation_epochs = 10`, or adopt the convention and change default
run behaviour.

## 7. DuckDB blog post

A post showing how to query the published benchmark data with DuckDB so readers can
verify claims instead of trusting a chart. A single shard reads over HTTPS; globs over
HTTP do not, so multi-file queries need an explicit list or a local fetch.

**Next step:** write the post with every query run against the live tree, including the
budget/timeout/spread caveats and a worked comparison.

## 8. Dedicated CLI e2e workflow

The BATS suite (`tests/e2e/`, ~85 cases) runs only as a step inside `rust.yml`. That
gives it the Rust workflow's triggers: too broad for `scripts/` changes, and too narrow
for PRs based on a feature branch (which get no run at all).

**Next step:** add a dedicated `.github/workflows/cli-e2e.yml` that checks out submodules,
builds the debug binary, runs the suite on `pull_request` and `push` to `master`, and
uses `paths` covering `scripts/**`, `tests/**`, `bench/**`, `teeline-cli/**`, `src/**`,
`Taskfile.yml`, `docs/problems/**`, `docs/benchmarks/schema.md` and the workflow itself.
Remove the suite from `rust.yml` so it runs once per PR.

## 9. jaq for TOML queries

Shell and BATS code reads TOML through `python3` heredocs. `jaq` reads TOML natively, so
the budget-agreement check in `tests/e2e/bench-matrix.bats` can become a short
comparison.

**Next step:** add `cargo:jaq` to `.mise.toml` (or install it in CI another way), wire
`mise-action` into the BATS job following `build-api.yml`, and compare with `-S` on both
sides so key order cannot make the test flaky. Keep
`scripts/bench-matrix-plan.py` on `tomllib` — it is program logic, not a query.

## 10. Missing optimal tours

25 of the 111 problems publish an optimum but ship no `.opt.tour`, so the web overlay
silently skips them. Two are Tier A instances (`burma14`, `dsj1000`).

**Next step:** source and verify each tour by scoring it against the published optimum
under the instance's own distance type, publish the files to R2, and add a producer or a
checked procedure for `data/tsplib/` — `data/` is gitignored and nothing currently
uploads the `tsplib/` prefix. Add a check that fails when a problem with a known optimum
has no tour.

## 11. Progress listener seam

Every solver takes `progress_tx: Option<&mpsc::Sender<ProgressMessage>>` — one identical
signature across ~24 files, with two consumers (the Qt engine and one test). Replacing it
with an observer/listener abstraction would let one emission reach zero, one or many
listeners, and would retire the test-only iteration counter in
`simulated_annealing.rs`.

**Next step:** settle the design first (trait object vs generic, per-event methods vs a
single `emit`, hot-path cost, whether the Qt channel sits behind an adapter listener),
then change the signatures in one focused PR.

## 12. Passkeys blog — technical recipe

The story post ("Passkeys instead of a login provider") is published. The follow-up
technical recipe is deliberately waiting on feedback.

**Next step:** when feedback arrives, write post #2 covering the WebAuthn registration and
login ceremonies, the D1 schema, challenge TTL/single-use storage, the session cookie,
SimpleWebAuthn edge-bundling constraints and the D1-vs-Durable-Objects rationale.

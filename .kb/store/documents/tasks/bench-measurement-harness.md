---
id: 01a12651-5604-74c1-9e04-1a8d386a2ee9
slug: tasks/bench-measurement-harness
title: "feat(bench): bench-matrix.sh harness with tier config and resume"
type: task
status: draft
priority: high
tags: [benchmarks, tooling]
blocked_by: [tasks/bench-sa-honours-epoch-budget, tasks/bench-plateau-stop-all-iterative, tasks/bench-platoo-epochs-help-name]
---

## Overview

The measurement harness named by the data contract does not exist. `docs/benchmarks/schema.md`
and `scripts/publish-benchmarks.sh` both reference `scripts/bench-matrix.sh` and
`bench/runs/tier-<tier>.jsonl`, but the only script present is `scripts/bench-solvers.sh`,
which:

- runs 4 of 22 solvers (`nn lk fourier branch_bound`) with hardcoded dataset lists
- writes six columns with no `config` and no `status`, and no provenance at all
- enforces no timeout
- cannot resume, though the schema calls the sweep "resumable; safe to interrupt and re-run"

The consumer half already expects the target contract, so this task completes the producer
half. It is also the last moment at which the raw format is cheap to settle: the writer does
not exist yet, so there is no legacy data and no second writer.

## The raw format is JSONL, and provenance is per row

Headline decision, already implemented on the consumer side:

- **One JSON object per run, one run per line.** Required `solver`, `dataset`, `run`,
  `wall_s`, `peak_rss_kb`, `tour_cost`; optional `config`, `status`, `measurement`.
- **`measurement` on every row**, not in a file header: `git_commit`, `git_commit_source`,
  `dirty`, `teeline_version`, `measured_at`, `environment{os,build,rust_version}`.

This matters because of how the harness resumes. A resumed or `--solver` run **merges new
rows into an existing file**, so once the solver is rebuilt the file holds rows from two
builds. With a file-level header, one commit would silently stamp the older rows — the same
false-provenance bug the publish contract just removed, one layer down. Per-row provenance
makes it detectable, and `publish-benchmarks.sh` now **refuses** a file whose rows for one
solver disagree rather than picking a winner.

The previous TSV plan was inherited from an ad-hoc perf script (PR #378) rather than chosen;
it could not represent per-row provenance, had positional columns, and left `status` an
untyped string where a typo counted as an error.

## Goals

- `scripts/bench-matrix.sh` driven by a tier config file rather than bash arrays
- Emit JSONL with a complete `measurement` on every row
- Enforce the per-family timeout; record `status: "timeout"` for a killed run
- Resume by skipping cells already present in the tier file
- `--solver <id>` filter, required by the single-algorithm skill
- Retire `scripts/bench-solvers.sh`

## Instances

Anchored on the specific instances already settled during planning, but this task defines the
tier configuration mechanism rather than the final set; the campaign task owns the numbers.

Capability-tiered rather than exhaustive: exact solvers are exponential and large instances
exceed several metaheuristics, so coverage is chosen to span every distance type and a range
of sizes. All chosen instances must be verified to parse — note `si175` fails on the
unsupported `UPPER_DIAG_ROW` format and must be substituted, not silently skipped.

## Acceptance Criteria

- [ ] `scripts/bench-matrix.sh` reads a tier config and writes `bench/runs/tier-<tier>.jsonl`
- [ ] Every row is a single-line JSON object with the required fields; `status` is one of
      `ok`, `timeout`, `error`
- [ ] Every row carries a validated `measurement`, accepted by `publish-benchmarks.sh`
- [ ] `measurement` is written once per run and is identical across the rows of one build, so
      the publisher's per-solver agreement check passes
- [ ] Per-family timeout enforced; a killed run is recorded with `status: "timeout"` and no
      fabricated `tour_cost`
- [ ] Re-running after interruption skips completed cells (verified by interrupting once)
- [ ] `--solver <id>` measures exactly one solver and appends to the existing file, and the
      resulting file still passes the publisher's agreement check
- [ ] A rebuild between runs produces rows that disagree, and the publisher **refuses** them —
      verified as part of this task, since it is the case the format exists for
- [ ] `--dry-run` lists what would run without running it
- [ ] A missing or unparseable instance fails loudly rather than being skipped silently
- [ ] `scripts/bench-solvers.sh` removed and references to it updated (Taskfile, docs)
- [ ] `shellcheck` clean

## Notes

- `scripts/publish-benchmarks.sh` already reads this path via `--runs` and already points at
  `scripts/bench-matrix.sh`, so no publisher change is needed for the basic flow. It validates
  the file more strictly than the old TSV path did: unknown keys, a non-boolean `dirty`, a
  non-numeric required field and a mixed-provenance file are all hard errors.
- The 7-column TSV and its `#`-provenance header are gone; `--tsv` is now `--runs`.

### Progress — 2026-10-11

Implemented on `feat/bench-matrix-harness`, [PR #571](https://github.com/timgluz/teeline/pull/571),
awaiting merge.

`scripts/bench-matrix.sh` (runner) + `scripts/bench-matrix-plan.py` (planner) +
`bench/matrix/tier-a.toml` (tier config) replace `scripts/bench-solvers.sh`, which is deleted;
`task bench:solvers` is now `task bench:tierA`. All acceptance criteria are covered by
`tests/e2e/bench-matrix.bats` (22 cases) and the full e2e suite is 107 green.

Two things worth carrying forward, both found while building it:

- **`default_timeout_s` was removed rather than kept as a fallback.** A family measured under a
  fallback while `bench/budgets.json` published another number would put an allowance on the page
  the data was not taken under. Every family named in `[coverage]` must now state its `[budgets]`
  entry, and a test fails if the tier and `bench/budgets.json` disagree.
- **`bench/budgets.json` disagreed with the tier** when the harness was first written — `exact`
  30s vs 60s and `local` 120s vs 300s. Aligned to the tier (which follows the Tier A plan). Worth
  knowing that the earlier values were placeholders, not decisions.

**The volume figure in `tasks/bench-tier-a-campaign` needs correcting.** It says 2,640 runs
(22 solvers x 12 instances x 2 budgets x 5 runs), but capability tiering means exact solvers are
never planned on the large instances — they cannot finish, so their budget would only buy a
timeout that was never in doubt. Tier A plans **249 cells / 1245 runs**. The task says the
campaign "owns the numbers", so it is left to that task rather than changed here.

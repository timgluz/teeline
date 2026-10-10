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
`bench/runs/tier-<tier>.tsv`, but the only script present is `scripts/bench-solvers.sh`, which:

- runs 4 of 22 solvers (`nn lk fourier branch_bound`) with hardcoded dataset lists
- writes 6 columns — `solver dataset run wall_s peak_rss_kb tour_cost` — while the contract
  requires 7, omitting `config` and `status`
- enforces no timeout, though the contract has `default_timeout_s`
- cannot resume, though the schema calls the sweep "resumable; safe to interrupt and re-run"

The consumer half already expects the target contract, so this task completes the producer half.

## Goals

- `scripts/bench-matrix.sh` driven by a tier config file rather than bash arrays
- Emit the 7-column TSV plus the `#`-provenance header the publisher prefers
  (`git_commit`, `dirty`, `teeline_version`, `rust_version`, `tier`, `timeout_s`)
- Enforce the per-family timeout; record `status=timeout` for a killed run
- Resume by skipping cells already present in the tier TSV
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

- [ ] `scripts/bench-matrix.sh` reads a tier config and writes `bench/runs/tier-<tier>.tsv`
- [ ] Output has all 7 columns including `config` and `status`
- [ ] Provenance header written and readable by `publish-benchmarks.sh`
- [ ] Per-family timeout enforced; a killed run is recorded as `status=timeout`, not `ok`
- [ ] Re-running after interruption skips completed cells (verified by interrupting once)
- [ ] `--solver <id>` measures exactly one solver and merges into the existing TSV
- [ ] `--dry-run` lists what would run without running it
- [ ] A missing or unparseable instance fails loudly rather than being skipped silently
- [ ] `scripts/bench-solvers.sh` removed and references to it updated (Taskfile, docs)
- [ ] `shellcheck` clean

## Notes

`scripts/publish-benchmarks.sh` already reads this path and already points at
`scripts/bench-matrix.sh`, so no publisher change is needed for the basic flow.
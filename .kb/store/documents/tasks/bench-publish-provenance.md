---
id: 01a12651-5631-7b92-b921-802fa80a731d
slug: tasks/bench-publish-provenance
title: "feat(bench): publish per-solver provenance, snapshots and criteriondb mirror"
type: task
status: draft
priority: high
tags: [benchmarks, r2, criteriondb, schema]
blocked_by: [tasks/bench-tier-a-campaign]
---

## Overview

Publish the measured results as versioned JSON to R2 and mirror the raw runs into criteriondb,
with **provenance attached to each solver individually**. This is the correctness-critical task
of the effort.

The contract currently records a single `git_commit` and a single `environment` for the whole
dataset, and states that the commit identifies the measured binary. That holds only while every
solver is re-measured together. Under the agreed per-release differential cadence it becomes
false: after a release changing two algorithms, the other twenty are still measured at an older
commit, yet a dataset-level commit presents all of them as current.

This is the same class of defect as the distance-metric bugs previously fixed — a plausible
number carrying a false claim. Unlike a miscalculated value it **cannot be reconstructed after
publication**: once live, the link between a solver's numbers and the commit that produced them
is gone unless it was recorded.

## Goals

- Provenance per solver, not per dataset
- A current view the frontend can read per algorithm page
- Immutable per-release snapshots for regression checking
- Raw per-run observations archived in criteriondb

## Layout

```
bench/v1/index.json               solvers, problems, budgets, environment
bench/v1/algorithms/{id}.json     current per-algorithm results  <- frontend reads this
bench/v1/problems/{id}.json       current per-problem leaderboard
bench/v1/snapshots/{tag}/...      immutable copy written at each release tag
```

Snapshots are ~154KB each, so retaining several releases is cheap.

## Acceptance Criteria

- [ ] `algorithms/{id}.json` carries a `measurement` block: commit, dirty, version,
      measured_at, environment
- [ ] A solver that was not re-measured retains its previous `measurement` block unchanged
- [ ] `index.json` no longer implies a single measured commit for all solvers
- [ ] Re-publishing one solver updates only that solver's shard and provenance
- [ ] Per-family budgets published and visible to consumers
- [ ] Unknown optimal remains `null`, never `0`
- [ ] Timeouts counted separately and excluded from gap aggregates
- [ ] Snapshot written at a release tag and verified retrievable
- [ ] criteriondb mirror carries `config_label` so `(solver, dataset, config, run)` is unique
- [ ] `scripts/read-benchmarks.sh` can read back index, algorithm and problem shards
- [ ] `schema_version` and the `/bench/vN/` prefix bumped together if the shape breaks

## Notes

`docs/benchmarks/schema.md` is the contract and must be updated in the same change, since this
alters what `git_commit` means. Cross-reference: `decisions/benchmarking-design` section 3.5.
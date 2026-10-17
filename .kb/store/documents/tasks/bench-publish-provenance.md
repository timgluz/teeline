---
id: 01a12651-5631-7b92-b921-802fa80a731d
slug: tasks/bench-publish-provenance
title: "feat(bench): publish per-solver provenance, snapshots and criteriondb mirror"
type: task
status: completed
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

## Progress Log

### 2026-10-11

Implemented as [#569](https://github.com/timgluz/teeline/pull/569), branch
`feat/bench-per-solver-provenance`.

Design decisions taken (both confirmed with the maintainer before implementing):

- **Previous state for the differential merge is fetched from the public URL into
  `build/bench-prev/`**, with `--previous <dir>` as an override and `--no-fetch-previous` to work
  offline. Chosen over requiring an explicit directory (no manual step) and over committing a mirror
  (avoids a second source of truth). The first v2 publish has no previous v2 tree, so it is a
  full baseline — expected, and the reason the merge machinery is covered by tests rather than by a
  live run.
- **Per-family budgets live in a committed `bench/budgets.json`**, which the harness can adopt later
  instead of inventing a second format. Only `meta` (300s, decision 10) is fixed by a decision; the
  others were never specified numerically, so they are published with `agreed: false` rather than
  presented as settled.

Also decided:

- **Bumped to `/bench/v2/` + `schema_version: 2`.** Removing provenance fields from `index.json` is
  breaking for a consumer of them, and the contract's own rule is that the prefix and version move
  together. v1 stays served and untouched. It is live but has no deployed consumer yet, so nothing
  needs migrating.
- **The criteriondb mirror now records the measured commit**, not publish-time HEAD — it had the same
  false-provenance defect this task exists to fix, and the acceptance criterion only asked for
  `config_label`.

Scope left out, deliberately:

- The **staleness guard** that refuses a single-solver re-run when shared code changed (design
  section 6). It belongs with `tasks/bench-single-solver-skill`, and now has the per-solver commits
  it needs in order to compare.
- The harness (`bench-matrix.sh`, `tier-a.toml`) — separate task; budgets were published now so the
  harness can read them.
- Migrating the live v1 data to v2; the campaign's baseline run establishes v2.

### Review rounds (2026-10-11)

Four open-code-review passes on #569: **13 → 7 → 5 → 8** findings, all addressed. The count going up
on the last pass is worth noting: the reviewer was by then probing edge cases rather than defects in
the main path, and the last round was mostly the same rule applied inconsistently.

What the rounds actually caught — nearly all in the "framework" around the transform rather than the
provenance shape itself:

- **Any fetch failure was read as "no previous tree"**, so a network blip would have published a tree
  containing only this TSV's solvers, replacing the live manifest and leaderboards. Only a confirmed
  404 means first publish now.
- **A failed shard download only warned**, silently dropping that solver from the republished tree.
- **The previous tree was trusted forever** from a cache that nothing refreshed, so a differential
  publish could merge against a stale tree and revert solvers.
- **The first publish could never succeed**: `fetch_previous` created the shard directories before
  discovering the 404, and the transform read the leftovers as a tree with no manifest and aborted.
  Verified against the live endpoint, since `bench/v2/` 404s today — the exact first-publish case.
- **`load_previous` returned early before reading the manifest**, dropping every solver the manifest
  named when the shard directory happened to be missing.
- **`set -e` is suspended inside a function called in a condition**, so failed `mkdir`/`cp` inside
  `fetch_previous` and `snapshot_stage` were ignored and a partial copy was reported as a written
  snapshot.
- **A partial TSV header paired a measured commit with the publisher's `dirty`, version, rustc, `os`
  and `build`** — the same false-claim class the task exists to fix, in miniature. Every field the
  TSV does not record is now `null`.
- Exposed a pre-existing CI bug: `upload_r2` demanded an R2 client even under `--dry-run`, so no
  publish test could have passed in CI. No publish test existed before this PR.

The lesson worth carrying: the *schema* change was straightforward, and everything expensive was in
the merge/IO framework around it. Tests that only exercised the shape (my first 11) passed while the
first publish was completely broken; the test that caught it had to drive the real HTTP path.

Also added `codecov.yml` with a 0.1% project threshold: `codecov/project` failed this branch at
-0.01% on a change touching no Rust at all, and the default 0% threshold reports that noise as a
failure. `codecov/patch` left strict.

### Completed — 2026-10-11

Merged in two PRs:

- [#569](https://github.com/timgluz/teeline/pull/569) (merge `8c9b2fff`) — per-solver `measurement`
  blocks, `index.json` stripped of its dataset-level commit claim, per-family budgets, immutable
  snapshots, the criteriondb mirror recording the measured commit, and the contract bumped to
  `/bench/v2` with `schema_version: 2`.
- [#570](https://github.com/timgluz/teeline/pull/570) (merge `b715ab9a`) — the raw run format moved
  from TSV to JSONL with per-row provenance, which is what makes a resumable sweep's mixed-build
  rows detectable. See `decisions/benchmarking-design` §3.5a.

Acceptance criteria are met and covered by tests (`tests/e2e/publish-benchmarks.bats`, 36 cases;
`cargo test --workspace` 29 groups; shellcheck clean on merged master).

**One criterion is verified by tests rather than by a live run:** "snapshot written at a release tag
and verified retrievable". The mechanism, its immutability guard and its read-back path are
implemented and tested locally, but **no v2 data has been published yet** — `bench/v2/` still 404s,
and the first publish will be a full baseline since there is no v2 tree to merge with. Exercising
that path against R2 belongs to `tasks/bench-tier-a-campaign`, which is where a real publish
happens.

Also produced along the way: `tasks/bench-lk-use-shared-plateau-option` and
`tasks/ci-cli-e2e-bats-workflow` (both draft), and a rewritten `tasks/bench-measurement-harness`
specifying JSONL with per-row provenance.

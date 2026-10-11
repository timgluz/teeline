---
id: 01a12651-561a-7fc0-8f40-23bcf5a773ba
slug: tasks/bench-tier-a-campaign
title: "chore(bench): run Tier A sweep across all solvers and instances"
type: task
status: draft
priority: high
tags: [benchmarks, campaign]
blocked_by: [tasks/bench-measurement-harness]
---

## Overview

Run the Tier A sweep across all 22 solvers and 12 instances, producing the raw measurements the
published data is derived from: **2,640 runs** (22 solvers x 12 instances x 2 budgets x 5 runs).

> **This figure is wrong, and the corrected one is 1245 runs / 249 cells.** It counts cells that
> capability tiering excludes: an exponential solver cannot produce a result on `dsj1000`, so
> measuring it there would only spend the budget proving a timeout that was never in doubt.
> `bench/matrix/tier-a.toml` therefore gives the `exact` family only the three small instances.
> The harness reports the real plan (`scripts/bench-matrix.sh bench/matrix/tier-a.toml --dry-run`),
> so take the number from there rather than from this estimate — and the ~11h wall-clock estimate
> below is high for the same reason.

Requires the three prerequisites, because the run depends on two things they provide: a working
iteration budget (SA currently ignores `--epochs`) and a convergence criterion (20 of 22 solvers
have none, so the to-convergence half of the matrix would otherwise be undefined).

## Goals

- Complete the Tier A sweep with no silently missing cells
- Record every run, including failures and timeouts, with an honest `status`
- Capture the environment alongside the numbers

## Budgets

Per family, recorded in the data and stated on the page. Comparisons are meaningful within a
family; cross-family gap comparisons are not like-for-like and must not be presented as such.

- exact: 60s
- local search (`2opt`, `3opt`, `or_opt`, `lk`): 300s
- metaheuristic / learned: 300s
- one-shot constructors: effectively trivial

Configs: `default` for all solvers, plus `branch_bound`'s `exact` config — it is the only solver
declaring a second config, and at its default `n_nearest` it is a beam search, so publishing it
unqualified as "exact" would misrepresent it.

## Acceptance Criteria

- [ ] Sweep completes with a row for every expected `(solver, instance, config, run)` cell, or a
      recorded `status` explaining its absence
- [ ] Both configs for `branch_bound` measured and labelled
- [ ] Both budget-limited and to-convergence runs present per cell
- [ ] Provenance header records the measured commit, version, rust version and tier
- [ ] Environment block captured (os, cpu, n_threads, ram, build, rust version)
- [ ] Every solver in `bench/solvers.json` appears, or is explicitly excluded with a reason
- [ ] Raw TSV committed to `bench/runs/` (or otherwise retained) so summaries stay recomputable
- [ ] Known timeouts on the largest instances documented as findings, not hidden

## Notes

Estimated ~11h wall clock, dominated by the metaheuristics. The estimate is not a measurement;
the first cells should pin it down before committing to a full run.

Expect timeouts on `a280` and especially `dsj1000` at the 300s metaheuristic budget. That is
acceptable and intended to be visible; the budget is expected to rise in later runs.

### Measurement basis: measure from a merged commit

The first Tier A attempt was started from `feat/bench-tier-a-budget-config`, and that branch was
then **squash-merged** (#573). A squash replaces the branch's commits with one new commit, so the
SHA the rows recorded — `f37b564d` — was left with no path from `master`. Every published row would
have named a commit a reader could not check out, which is the one thing per-solver provenance
exists to make possible.

The measurements themselves were sound: `git diff f37b564d origin/master -- src/ bench/matrix/
scripts/` was empty, so master's tree is byte-identical to what produced them and the numbers were
not in question. Only the identity was orphaned. The sweep was restarted from `18d8b274` after 490
rows, because re-measuring the cheap rows cost ~40 minutes while the expensive timeouts were still
ahead — cheaper than publishing an unverifiable reference or inventing a tag to rescue it.

**Two things to carry forward:**

- Start a long measurement from `master`, or from a commit already merged. The harness records the
  commit of the tree it runs from, which is correct; the hazard is that the tree is a branch that
  may not survive the merge.
- This project squash-merges. Any measurement taken from an open PR's branch will be orphaned the
  same way.

Checked afterwards: the restarted run produced identical tour costs for the re-measured rows, so
the restart cost wall-clock time and nothing else.

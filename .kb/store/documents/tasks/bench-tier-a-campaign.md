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
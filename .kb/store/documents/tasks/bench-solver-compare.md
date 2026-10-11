---
id: 01a12b16-2fb5-77a1-bda3-a36249775339
slug: tasks/bench-solver-compare
title: "feat(web): compare an algorithm's published results with another solver"
type: task
status: completed
priority: high
tags: [benchmarks, teeline-web, frontend]
---

## Overview

A reader on `/algorithms/lk/` can see how LK scores, but not how that compares with `2opt` on the
same instances — today that means two tabs, or downloading the shards and diffing them by hand. The
published data already supports the comparison: one shard per solver, so the page can fetch a second
one at runtime and show the two side by side.

The point is comparing **without leaving the page or downloading anything**, which is also what makes
it useful to someone who has not installed teeline.

## What exists to build on

- `teeline-web/src/lib/benchmark.ts` — `fetchAlgorithmShard`, `buildRows`, `chartSeries`,
  `summarise`, `provenance`, and the rules below.
- `AlgorithmBenchmarks.tsx` already fetches one shard as a `client:visible` island, so a second fetch
  is the same shape and can be deferred until a solver is actually chosen.
- `solver-index.ts` lists every solver with its doc id, family and determinism: the picker's options,
  minus the page's own solver.
- `?compare=<id>` makes a comparison a link someone can share, and gives the island state to read at
  hydration.

## Honesty rules this has to extend

1. **Align by instance, never zero-fill.** A comparator not measured on an instance reads "not
   measured". Absence and failure are different things — the point
   [[tasks/bench-problem-leaderboard]] makes about leaderboards applies here too.
2. **Compare like-for-like configs.** `branch_bound` publishes `budget` and `exact`; pairing its beam
   search against another solver's budget run compares configurations, not algorithms.
3. **Two families means two budgets.** State both, and say cross-family gaps are not like-for-like
   (decision 4).
4. **Spread stays visible.** If either solver varies between runs, both show best and worst: a
   median-only comparison turns a distribution into a fact.
5. **Provenance is per solver.** The two shards may have been measured at different commits, and the
   footer states each rather than presenting one build for both.
6. **One axis rule for the pair.** Gaps when either has them, tour length when neither does, decided
   once for the comparison, with the caption saying what was omitted.

## Suggested shape

- A picker in the benchmark section, defaulting to nothing selected so the page does not get heavier
  for readers who do not want a comparison.
- Selected: the table gains the comparator's columns beside the primary's, the scatter gains a second
  series in a distinct colour with a legend, and the provenance footer covers both measurements.
- Offering only solvers that share at least one instance beats offering all 22 and then rendering an
  empty comparison.

## Acceptance Criteria

- [ ] An algorithm page can compare against any solver that shares an instance, without leaving the page
- [ ] `?compare=<id>` restores the comparison, so it can be shared
- [ ] An instance the comparator was not measured on reads "not measured", never 0
- [ ] `branch_bound`'s two configs are compared like-for-like, or labelled as unlike
- [ ] Both families' budgets are stated whenever the comparison spans families
- [ ] A non-deterministic solver on either side shows its spread
- [ ] The footer shows each solver's own provenance
- [ ] A comparator with no published shard says so rather than erroring
- [ ] Unit tests cover the join (alignment, absence, config mismatch); e2e covers picking a solver
      and following a `?compare=` link
- [ ] The initial page load still fetches exactly one shard

## Open questions

- Two series on one chart, or the primary's chart plus a per-instance delta?
- Is a picker enough, or should `/algorithms/` offer a pre-built comparison of two chosen solvers?
- Should problem pages link into a comparison? That page belongs to
  [[tasks/bench-problem-leaderboard]], so the join should live in the shared read layer either way.

## Progress Log

### 2026-10-11 — shipped in #576, one criterion short

The comparison, the `?compare=` deep link, the comparator columns and summary row, both provenance
blocks and the config-keyed join all shipped in #576. One acceptance criterion is **not** met: a
non-deterministic *comparator* shows only its median gap, where the criterion asks for its spread.
The primary's spread is unaffected (it has its own best/worst columns). Fixing that is the next
commit on this task, not a new one.

### 2026-10-11 — the missing criterion is fixed in #577

A non-deterministic comparator now renders its spread in its column and its summary row
(`8.18% (7.62%–8.92%)`), which is what the criterion asked for. This task can be marked completed
once #577 merges; the fix landed on `feat/bench-problem-leaderboard` rather than a branch of its
own, since both touch the same component.

### 2026-10-11 — completed (#577 merged)

The task's own note said it could be marked completed once #577 merged, since the missing
non-deterministic-spread criterion was fixed there. #577 is merged (`e126ccc0`).

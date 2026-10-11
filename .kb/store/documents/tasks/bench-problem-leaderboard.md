---
id: 01a12a7f-a6c5-7322-9471-5b900527510b
slug: tasks/bench-problem-leaderboard
title: "feat(web): render the per-instance leaderboard and graph on problem pages"
type: task
status: active
priority: high
tags: [benchmarks, teeline-web, frontend]
---

## Overview

Problem pages describe an instance and link to the solver, but show no results: a reader can see
that `berlin52` has 52 cities and an optimal tour, and nothing about how any solver does on it.
The published leaderboard has been live since the first `/bench/v2/` publish and is unused.

This reuses the read layer and the rendering rules built by `tasks/bench-algorithm-pages`. It is a
separate task because the shape is inverted — one instance across many solvers, rather than one
solver across many instances — so the table, the ordering and the graph are all different.

## The shape being rendered

`problems/{dataset}.json`:

```
schema_version, dataset, cities, optimal_cost,
leaderboard[]: {solver, config, runs, timeouts, errors,
  [gap_pct{median,best,worst,mean}], tour_cost{...}, wall_s{...}, [peak_rss_kb{...}],
  [optimal_cost]}
```

Confirmed against the live shard. Two things it already shows:

- `optimal_cost` is **null** for six of the twelve Tier A instances, and when it is, no `gap_pct` is
  present. Those pages rank on tour cost and say so; they must not show a gap column of blanks.
- A solver may appear more than once: `branch_bound` publishes both its `budget` and `exact`
  configs. The leaderboard is a list of `(solver, config)` rows, not of solvers, and collapsing
  them would present a beam search and an exact run as one entry.

## Content

- **Summary line** — how many solvers are measured, and the budget each family was given, since a
  ranking mixes families and cross-family gaps are not like-for-like (decision 4)
- **Leaderboard table** — rank, solver, config, gap (median, best) *or* tour cost when the optimum
  is unknown, wall time (median), runs, and a timeout count where there were any
- **Graph** — the same ranking as a horizontal bar chart, drawn as inline SVG in the same style as
  the landing chart. Bars are coloured or grouped by family so the budget difference is visible
  rather than something the reader has to remember.
- **Provenance footer** — per solver, because each has its own `measurement`; two rows in one table
  may legitimately have been measured at different commits

## Honest rendering rules

Inherited from `tasks/bench-algorithm-pages`, and each one bites harder here because a leaderboard
ranks:

- **A timed-out solver is ranked last and marked, not omitted and not ranked as a result.** Dropping
  it hides a real outcome; ranking it on a fabricated number is worse. A solver that timed out on
  the instance has no tour to place.
- **A solver that was never measured on this instance is absent, not last.** Absence and failure are
  different things and the table must not conflate them.
- **The spread is shown.** Median and best are both columns; a leaderboard of medians alone turns a
  non-deterministic solver into a deterministic-looking one.
- **Ranking within a family is like-for-like; across families it is not.** If the table sorts all
  rows together, the budget column and the family grouping must make that legible.
- **`optimal_cost: null` means no gap.** The column is dropped, not zero-filled.

## Acceptance Criteria

- [ ] Problem page renders the leaderboard table, the graph and a per-solver provenance footer
- [ ] An instance with no published shard renders no results section and does not error
- [ ] A timed-out `(solver, config)` row is shown as a timeout, ranked last, with no gap and no cost
- [ ] A solver not measured on this instance is absent rather than ranked last
- [ ] `branch_bound`'s `budget` and `exact` configs appear as separate rows
- [ ] Instances with no known optimum render on tour cost with no gap column, and the graph says so
- [ ] Families are visually distinguishable, and the budget is stated wherever families are mixed
- [ ] `optimalCost: null` on the page itself (which already drives `solverLink`) stays consistent
      with what the leaderboard shows
- [ ] `tsc`, vitest and the web build pass; no new build-time network dependency
- [ ] Unit tests cover: ranking with a timeout present, ranking without a gap, and two configs of
      one solver staying distinct

## Notes

- The page already computes `sizeIcon` and a `solverLink` that is omitted when `optimalCost` is null.
  The results section should reuse that logic rather than re-deriving "is the optimum known".
- `problems/index.astro` lists instances; consider whether it should show a coverage hint (how many
  solvers are measured per instance) — deliberately out of scope here, noted so it is a decision
  rather than an oversight.
## Findings before implementing

Two things the published shape does not support as written, both checked against the live tree:

- **Problem shards carry no provenance.** `problems/{id}.json` has exactly `schema_version`,
  `dataset`, `cities`, `optimal_cost` and `leaderboard`; the per-row `measurement` block lives only
  in `algorithms/{id}.json`. A per-solver provenance footer therefore cannot be built from this
  shard without fetching up to 22 algorithm shards. The page instead links each row's solver to its
  own page, where that solver's `measurement` is stated, and says why.
- **A solver whose every run timed out is absent, not last.** `scripts/publish-benchmarks.sh` drops
  rows with `runs == 0` before ranking ("Solvers that only timed out never reach the leaderboard:
  there is nothing to rank"), so the all-timeout case is indistinguishable from never measured. The
  criterion "ranked last and marked" holds for a *partially* timed-out cell (runs > 0, timeouts > 0)
  and renders from the data; making the all-timeout case renderable needs the publisher to publish
  `runs: 0` rows, which is a change to the contract and is not in this task.

## Plan

1. **Read layer** (`src/lib/benchmark.ts`): `leaderboardRows(shard)` — one row per `(solver,
   config)` with family and determinism from `bench/solvers.json`, ranked on the instance's axis
   (gap when an optimum is published, tour cost otherwise), with unrankable rows last and `rank:
   null`. Reuses `parseProblemShard`, `chooseColumns`' rules and `format*`.
2. **Component** (`ProblemResults.tsx`, `client:visible`): summary line stating every family's
   allowance (families are mixed here by design), the ranked table, a horizontal bar chart coloured
   by family, the solver-page link per row, and the data links from
   [[tasks/bench-data-download]].
3. **Page**: mount it on `problems/[id]/index.astro` after the markdown body, passing the doc's
   `optimalCost` so a disagreement between the page's claim and the shard's can be stated rather
   than silently shown twice.
4. **Tests**: ranking with a timeout present, ranking without a gap, two configs of one solver
   staying distinct, the no-shard case, and an e2e for the rendered board.

---
id: 01a12a9f-cbab-71d0-9901-7aeb4b890f51
slug: tasks/fix-euc-2d-rounding
title: "fix(solvers): EUC_2D must use TSPLIB's nint rounding, as Att and Ceil2D already do"
type: task
status: draft
priority: high
tags: [solver, correctness, benchmarks, distance]
---

## Overview

TSPLIB defines `EUC_2D` as `nint(sqrt(dx^2 + dy^2))` — an integer. teeline computes
`(dx*dx + dy*dy).sqrt()` unrounded, in two places:

- `src/tsp/kdtree.rs:341` — `KDPoint::distance`, which builds the distance matrix
- `src/tsp/comparison.rs:34` — tour-cost comparison

`Att` and `Ceil2D` are both correctly rounded, and their doc comments even say "Integer-valued".
`Euc2D` has no doc comment, no test pinning its rounding, and no mention of a metric anywhere in
the docs. The parser accepts `EDGE_WEIGHT_TYPE: EUC_2D`, so the code claims a TSPLIB type it does
not implement.

## Evidence

Scoring TSPLIB's own optimal route for an instance should give that instance's published optimum.
It does for every type except EUC_2D:

| instance | type | published optimum | teeline scores the optimal tour | |
| --- | --- | --- | --- | --- |
| gr120 | EXPLICIT | 6942 | 6942.00000 | ok |
| att48 | ATT | 10628 | 10628.00000 | ok |
| berlin52 | EUC_2D | 7542 | 7544.36572 | wrong |
| kroA100 | EUC_2D | 21282 | 21285.43945 | wrong |
| a280 | EUC_2D | 2579 | 2586.76953 | wrong |
| eil76 | EUC_2D | 538 | 545.38763 | wrong |

Independently confirmed in Python: berlin52's optimal route rounds to **7542** and is
**7544.36590** unrounded, matching teeline's 7544.36572 to `f32` precision. a280: 2579 rounded,
2586.76965 unrounded, teeline 2586.76953.

## Why it matters

This is not rounding noise. TSPLIB's shipped route is optimal *for the rounded objective*, so it is
not optimal for ours — the two are different optimisation problems with different optima.

Measured consequences:

- **`eil76`'s optimal route scores 545.39 against a stated optimum of 538** — a gap of +1.37%. On a
  page that means no solver can ever show a gap below +1.37%, an impossible floor.
- Six of the twelve Tier A instances are EUC_2D (`ulysses22`, `berlin52`, `eil76`, `kroA100`,
  `ch150`, `a280`), so half the published gap columns compare teeline's objective against a
  different one.
- `docs/problems/*.md` cannot publish an optimum for an EUC_2D instance whose `.opt.tour` states no
  cost, because the score of that route is not the optimum of anything we optimise. `berlin52`
  (7542) and `a280` (2579) are both held null for exactly this reason, and
  `scripts/gen-problem-pages.mjs` deliberately refuses to score EUC_2D routes.

Solver-vs-solver rankings are unaffected: every solver shares the metric, so the comparison is
internally consistent. It is specifically *gap to optimal* that is wrong.

## Fix

Round `EUC_2D` with `nint` in both places, matching the TSPLIB definition the parser already
advertises and the treatment `Att`/`Ceil2D` already receive. Add a test pinning an instance whose
rounded and unrounded lengths differ, using a route whose optimal score is published — `berlin52`
→ 7542 is the obvious one.

## Blast radius — check before changing

- **Every EUC_2D result changes**, so any measured data becomes stale. The benchmark runs are
  re-measurable, but check `docs/`, `teeline-web/` content and any blog post that quotes a number.
- The KD-tree uses `KDPoint::distance` for pruning. Rounding keeps it monotone in the true distance
  (ties become more likely, which the search already handles), but this deserves a test rather than
  an assumption.
- `f32` storage means a rounded distance is exact while an unrounded one is not, so results may
  become *more* reproducible, not less.
- After the fix, `scripts/gen-problem-pages.mjs` should score EUC_2D routes too: allow `EUC_2D` in
  `tourLength`, and `berlin52`/`a280` fill in as 7542 and 2579 with no further work.

## Publish ordering — do not publish between the fix and the re-run

State as of 2026-10-11: the fix is in the working tree, and `36f80072` has already filled the EUC_2D
optima in `docs/problems/*.md` from TSPLIB's published list (**`berlin52` 7542, `a280` 2579,
`eil76` 538, `kroA100` 21282, `ch150` 6528**). Those are the *rounded* objectives, while every
published shard was measured against the unrounded one, so publishing before re-measuring produces
shards that mix two metrics:

- `scripts/publish-benchmarks.sh:635-650` builds `instances` from `docs/problems/*.md`, so
  `problems/{id}.json` takes `optimal_cost` from the docs regardless of how the rows were measured.
- A solver in the run log is recomputed (gap from its fresh tour costs against that optimum), but a
  solver **not** in the run log is carried over byte for byte (`:800-814`), keeping tour costs from
  the old metric.
- `:830-869` then merges the two sets into one leaderboard and re-ranks them together. The result
  ranks rows measured under different objectives against each other — the mixed-metric ranking the
  differential cadence is otherwise careful to avoid.

So the order is: land the fix → re-measure every EUC_2D instance (6 of the 12 Tier A instances;
`berlin52` and `eil76` in the current prelim set) → then publish. A republish alone is not enough and
is worse than not publishing, because it attaches the new optima to old measurements.

## Acceptance Criteria

- [ ] `EUC_2D` distances are `nint`-rounded in both `KDPoint::distance` and the comparison path
- [ ] The optimal route for an EUC_2D instance scores its published optimum, verified for
      `berlin52` (7542), `eil76` (538) and `kroA100` (21282)
- [ ] `Att` and `Ceil2D` are unchanged; a test pins all three
- [ ] The KD-tree's nearest-neighbour results stay correct with rounded distances
- [ ] `scripts/gen-problem-pages.mjs` accepts `EUC_2D` in `tourLength`, and the regenerated docs
      show `berlin52` 7542 and `a280` 2579
- [ ] Affected `docs/` and web content numbers are found and corrected or noted
- [ ] A benchmark re-run follows, since the previous EUC_2D rows are invalidated
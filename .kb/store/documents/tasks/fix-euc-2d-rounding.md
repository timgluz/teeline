---
id: 01a12a9f-cbab-71d0-9901-7aeb4b890f51
slug: tasks/fix-euc-2d-rounding
title: "fix(solvers): EUC_2D must use TSPLIB's nint rounding, as Att and Ceil2D already do"
type: task
status: completed
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

**Resolved.** That order was followed: the shards behind `bench/v2/` were measured at `fe89b6a9`, the
fix commit itself, so no row mixes the two metrics.

## Acceptance Criteria

- [x] `EUC_2D` is `nint`-rounded wherever a *cost is reported*. Amended: the original wording asked
      for rounding in `KDPoint::distance` too, and #575 deliberately left that one unrounded — it is
      the spatial index's distance and the "are these the same point" check, where rounding would
      make distinct points compare equal. The scoring path (`DistanceMatrix`) rounds, and that is what
      a reported tour cost is computed from.
- [x] The optimal route for an EUC_2D instance scores its published optimum: `berlin52` 7542,
      `eil76` 538, `kroA100` 21282 and `a280` 2579, verified by scoring all 32 shipped tours with both
      the JS scorer and the CLI's own metric.
- [x] `ATT` and `CEIL_2D` are unchanged; `teeline-web/scripts/problem-tour.test.mjs` pins `EUC_2D`,
      `ATT`, `CEIL_2D` and `GEO`.
- [x] The KD-tree's nearest-neighbour results stay correct: its distance stays unrounded, so the
      search is untouched by the rounding, and `tests/test_kdtree_and_distance_matrix.rs` covers it.
- [x] `scripts/gen-problem-pages.mjs` accepts `EUC_2D` in `tourLength`, and the regenerated docs show
      `berlin52` 7542 and `a280` 2579. Regenerating is a *no-op* — the fix removes the ability to
      publish a wrong derived value rather than changing a published one.
- [x] Affected numbers found and noted: the audit produced the two findings below.
- [x] A benchmark re-run followed. The published shards were measured *at* this fix:
      `bench/v2/algorithms/lk.json` records `git_commit: fe89b6a9`, `measured_at:
      2026-10-11T11:58:20Z`, and `lk`'s best tour cost on `berlin52` is exactly 7542, the rounded
      optimum.

## Completion Evidence

- Solver half: **#575** (`fe89b6a9`). Generator half: **PR #587**.
- Audit behind #587: all 32 shipped `.opt.tour` files scored with the JS scorer and the CLI's
  `--optimal-tour`. The 22 coordinate-type instances agree exactly with each other and with the docs;
  eight EXPLICIT instances score `null` in JS by design, because the scorer handles coordinate types
  and their pages take the cost from the tour's COMMENT.
- The audit also found two defects in the fallback that no corpus instance could expose: routes read
  one id per line (pr1002 writes 1002 across 64 lines) silently scored a *subset*, and nothing
  checked that a route named every city (rd100's 11-of-100 ids scored 4344 against a 7910 optimum).
  Both are fixed in #587; the scorer moved to `scripts/lib/problem-tour.mjs` with 8 unit tests,
  because the corpus cannot exercise it.
- Two findings left open, neither caused nor fixed by this work:
  - **`rd100`'s tour file has no `DIMENSION` header.** The CLI's `--optimal-tour` requires one and
    reports `DIMENSION=0 but parsed 100 cities`, so that page cannot overlay its optimal tour or show
    a gap. The JS scorer takes the dimension from the `.tsp` and scores 7910, the published optimum.
  - **`tsp225` is precision-sensitive.** Its route scores 3916 with f64 coordinates (the published
    optimum, and what the docs publish via `bench/optima.json`) but 3919 with teeline's f32 storage:
    124 of its 225 edges sit within 1e-6 of a `.5` rounding boundary and 3 flip. This is the case that
    justifies consulting a published optimum before deriving one. Changing the solver's coordinate
    precision would invalidate published rows, so it is recorded rather than done.

## Progress Log

### 2026-10-11 — generator half implemented, in review

#575 fixed the metric; the remaining criterion was the generator's, and the audit behind it turned up
the two latent defects above. Marked `review` rather than `completed` until #587 merges.

### 2026-10-11 — solver half landed

#575 made `EUC_2D` round as TSPLIB defines it, in the distance matrix that reported costs come from.
The published shards were then re-measured at that commit.

### 2026-10-11 — merged

PR #587 merged as `350c5919`, on top of a second `ocr` pass that found a hole in the completeness
check this branch added (a route of the right length that repeated a city scored a square as 18) and
a missing `acos` clamp in the GEO metric. Both are in the branch; the merge commit is the evidence.

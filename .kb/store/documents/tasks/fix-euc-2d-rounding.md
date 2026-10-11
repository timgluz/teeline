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
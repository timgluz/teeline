---
id: 01a12ab3-9a9d-7cb2-8df8-f77bd9e41ede
slug: tasks/ship-missing-optimal-tours
title: "data(tsplib): ship the 25 optimal tours that are missing from the repo and R2"
type: task
status: draft
priority: medium
tags: [data, tsplib, benchmarks, web]
---

## Overview

Every problem page should be able to show and offer an optimal route: the page links
`/solve/?dataset=<id>`, and the solver fetches `<id>.opt.tour` at runtime
(`teeline-web/src/upload.ts`, `datasetOptTourUrl`) to overlay it for a gap comparison.

**25 of the 111 problems have a published optimum and no optimal route anywhere** — not in
`data/tsplib/`, and not on R2 (`https://static.tspsolver.com/tsplib/<id>.opt.tour` returns 404).
The other 86 ship theirs. For those 25 the solver silently skips the overlay (`upload.ts` swallows
the failed fetch) and nothing on the site says the route is missing.

## What is missing

| instance | published optimum | notes |
| --- | --- | --- |
| ali535 | 202339 | |
| att532 | 27686 | |
| bier127 | 118282 | |
| brazil58 | 25395 | |
| brd14051 | 469385 | |
| burma14 | 3323 | GEO, in the Tier A set |
| d1291 | 50801 | |
| d15112 | 1573084 | |
| d1655 | 62128 | |
| d18512 | 645238 | |
| d198 | 15780 | |
| d2103 | 80450 | |
| d493 | 35002 | |
| d657 | 48912 | |
| dantzig42 | 699 | |
| dsj1000 | 18660188 | CEIL_2D, in the Tier A set |
| fl1577 | 22249 | |
| fl3795 | 28772 | |
| linhp318 | 41345 | |
| pla33810 | 66048945 | |
| pla85900 | 142382641 | CEIL_2D |
| rl11849 | 923288 | |
| rl5915 | 565530 | |
| rl5934 | 556045 | |
| usa13509 | 19982859 | |

Two of them are Tier A instances (`burma14`, `dsj1000`), so this is not only an obscure-instance
problem.

## Root cause

`download_data.sh` never fetches `.opt.tour` at all — `grep -n 'opt.tour' download_data.sh` finds
nothing. The 86 existing tours were added by hand or by an older path, so the set drifted from the
instance set as problems were added. A fix that only adds the 25 files leaves the gap to reopen with
the next instance.

## Verification is mandatory, and cheap

A tour is only worth shipping if it scores the published optimum under that instance's own distance
definition. `pla85900` was checked this way — 85900 cities, each visited exactly once, CEIL_2D score
142382641, matching `docs/problems/pla85900.md`:

```python
# score under the instance's distance type: CEIL_2D, ATT, EUC_2D (nint), GEO, EXPLICIT
total = sum(distance(coords[c], coords[tour[(i + 1) % n]]) for i, c in enumerate(tour))
```

A score that disagrees with the published optimum means a wrong file, a wrong distance definition,
or a wrong optimum — resolve it before committing. Scoring a **EUC_2D** route needs the rounding fix
in [[tasks/fix-euc-2d-rounding]]: until that lands, an unrounded score is not the optimum of
anything teeline optimises.

## Sources

TSPLIB95's own distribution is the natural bulk source, but `comopt.ifi.uni-heidelberg.de` did not
resolve from this machine (HTTP 000), and the `mastqe/tsplib` GitHub mirror carries no `.opt.tour`
files. The Waterloo/Concorde pages serve them per instance:
`https://www.math.uwaterloo.ca/tsp/pla85900/data/pla85900_opt.tour` works, while guessing the same
pattern for `dsj1000`, `att532`, `usa13509`, `d15112` and `rl11849` returns 403 — so each instance
needs its own page located rather than a patterned URL.

## Acceptance Criteria

- [ ] Every problem with a published optimum ships a matching `.opt.tour` in `data/tsplib/`
- [ ] Each added tour is verified by scoring it against the instance's published optimum under the
      instance's own distance type, with the verification recorded
- [ ] The tours are published to R2, and probing confirms no `tsplib/*.opt.tour` 404s
- [ ] `download_data.sh` fetches `.opt.tour` alongside `.tsp`, so the set cannot drift again
- [ ] Something fails when a problem with a known optimum has no tour, so the next instance added
      does not reintroduce the gap

## Notes

- Found while answering a question about `pla85900`: the Waterloo tour independently confirms the
  optimum already in `docs/problems/pla85900.md` (filled from TSPLIB's published list in 36f80072).
  The cost was never the gap — the route is.
- Sizes: the largest tours are ~500 KB (`pla85900_opt.tour`), so a bulk import grows the R2 object
  set by a few MB. Not a concern, but worth knowing before the import.

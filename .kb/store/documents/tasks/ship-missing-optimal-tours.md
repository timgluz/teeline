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

There is no producer for the TSPLIB set at all — not a broken one, an absent one.

- `data/` is **gitignored** (`/data` in `.gitignore`) and no file under it is tracked, so neither the
  instances nor the tours are version-controlled.
- `download_data.sh` does fetch a `data.zip`, but from the R2 dev bucket
  `pub-ce8744ae82a64ae7b60e364ba9c7ab52.r2.dev`, and that archive is the **DiscOpt** set (80 files
  under `tsp/data/`, `tsp_99_1`-style names, plus `_coursera/` and `handout.pdf`) — it contains **zero
  `.tsp` and zero `.opt.tour`** files. It populates `data/tsp`, not `data/tsplib`.
- `data/tsplib/` (111 `.tsp`, 86 `.opt.tour`, 9.7 MB) was therefore assembled out of band. The
  mtimes split cleanly: 32 tours carry TSPLIB's `1995-04-27` timestamp and 54 were added
  `2026-08-09`.
- **Nothing in the repo uploads the `tsplib/` prefix to R2.** The only R2 writer is
  `scripts/publish-benchmarks.sh`, and it writes `bench/v2/`. `static.tspsolver.com/tsplib/*` is
  therefore published out of band too.

So the 25 missing tours are missing from a set that has no reproducible origin, and fixing only the
25 files leaves the same hole for the next instance. Either the local set and the R2 prefix get a
producer, or the procedure is written down and made checkable.

## Verification is mandatory, and cheap

A tour is only worth shipping if it scores the published optimum under that instance's own distance
definition. Two are verified so far:

- `pla85900` — 85900 entries, each city once, CEIL_2D score **142382641** (matches the doc).
- `d15112` — 15112 entries, each city once, EUC_2D `nint` score **1573084** (matches; the file's own
  `COMMENT` also states it).

Both were scored against the instance's `.tsp` from `static.tspsolver.com/tsplib/`, not against a
number in the tour file:

```python
# score under the instance's distance type: CEIL_2D, ATT, EUC_2D (nint), GEO, EXPLICIT
total = sum(distance(coords[c], coords[tour[(i + 1) % n]]) for i, c in enumerate(tour))
```

Gotcha found while writing that check: a tour's header carries numbers too
(`DIMENSION: 15112`, `COMMENT: ... length 1573084`). Parsing every integer in the file instead of
only the `TOUR_SECTION` silently yields a bogus permutation, so parse the section and stop at `-1`.

A score that disagrees with the published optimum means a wrong file, a wrong distance definition,
or a wrong optimum — resolve it before committing. Scoring **EUC_2D** needs `nint` to match the
published figure; note this is the *published* optimum, and the shipping code does not round yet
([[tasks/fix-euc-2d-rounding]]).

## Sources

TSPLIB95's own distribution is the natural bulk source, but `comopt.ifi.uni-heidelberg.de` did not
resolve from this machine (HTTP 000), and the `mastqe/tsplib` GitHub mirror carries no `.opt.tour`
files.

The Waterloo/Concorde site carries them per instance, but **under a per-application directory, not a
pattern**: its index (`https://www.math.uwaterloo.ca/tsp/`) groups by application (`vlsi/`, `data/usa/`,
`pla85900/`, `d15sol/`, `world/`, `amz/`, …). Two confirmed so far:

| instance | source | verified |
| --- | --- | --- |
| pla85900 | `https://www.math.uwaterloo.ca/tsp/pla85900/data/pla85900_opt.tour` | 142382641 (CEIL_2D) |
| d15112 | `https://www.math.uwaterloo.ca/tsp/d15sol/d15112.tour` (linked from `d15sol/d15opt.html`) | 1573084 (EUC_2D nint) |

Guessing the pattern for other instances returns 403 (`dsj1000`, `att532`, `usa13509`, `rl11849`), so
each needs its own page located. Worth trying first, since the directories cover several missing
instances at once: `vlsi/` (pla33810, pla85900), `data/usa/` (usa13509).

## Acceptance Criteria

- [ ] Every problem with a published optimum ships a matching `.opt.tour` in `data/tsplib/`
- [ ] Each added tour is verified by scoring it against the instance's published optimum under the
      instance's own distance type, with the verification recorded
- [ ] The tours are published to R2, and probing confirms no `tsplib/*.opt.tour` 404s
- [ ] `data/tsplib/` has a reproducible producer, or a written procedure with a check — today
      `download_data.sh` fetches an unrelated DiscOpt archive and no script uploads the `tsplib/`
      prefix at all
- [ ] Something fails when a problem with a known optimum has no tour, so the next instance added
      does not reintroduce the gap

## Notes

- Found while answering a question about `pla85900`: the Waterloo tour independently confirms the
  optimum already in `docs/problems/pla85900.md` (filled from TSPLIB's published list in 36f80072).
  The cost was never the gap — the route is. `d15112` came the same way, from
  `d15sol/d15opt.html`.
- Sizes: the largest tours are ~500 KB (`pla85900_opt.tour`), so a bulk import grows the R2 object
  set by a few MB. Not a concern, but worth knowing before the import.
- The 25 verified-and-installed files are not a git change: `data/` is gitignored, so this lands in
  the local set and on R2, and the repo only records the procedure (and, if built, the verifier).

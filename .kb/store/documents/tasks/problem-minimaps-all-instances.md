---
id: 01a12b55-d379-7202-be99-170112c8918d
slug: tasks/problem-minimaps-all-instances
title: "feat(web): a minimap on every problem page, not only the coordinate-bearing instances"
type: task
status: completed
priority: medium
tags: [teeline-web, docs, tooling, seo]
---

## Overview

Every `/problems/{id}/` page is meant to carry a small inline-SVG minimap of the instance. The
generator only produces one when it can find points, so 17 of the 111 pages — all `EXPLICIT` — have
no image at all: `bayg29`, `bays29`, `brazil58`, `brg180`, `dantzig42`, `fri26`, `gr17`, `gr21`,
`gr24`, `gr48`, `gr120`, `hk48`, `pa561`, `si175`, `si535`, `si1032`, `swiss42`.

The premise "we only visualise EUC_2D" is not quite right and the real shape matters: 94 pages
already have a map, and they cover every coordinate-bearing type — EUC_2D (78), GEO (10),
CEIL_2D (4), ATT (2). Verified against the live site, e.g. <https://www.tspsolver.com/problems/ali535/>
renders 535 circles. The hole is `EXPLICIT` only, and it has two different causes:

- **5 pages have coordinates in the file, in a section the generator never reads.**
  `bayg29`, `bays29`, `dantzig42`, `gr120`, `pa561` ship a `DISPLAY_DATA_SECTION`. `parseCoords`
  reads only `NODE_COORD_SECTION`, and `hasCoords` hard-excludes anything whose `EDGE_WEIGHT_TYPE`
  contains `EXPLICIT`. Both are one-line fixes.
- **12 pages publish no coordinate-like data at all**: `brazil58`, `brg180`, `fri26`, `gr17`,
  `gr21`, `gr24`, `gr48`, `hk48`, `si1032`, `si175`, `si535`, `swiss42`. The three `si*` instances
  even declare `DISPLAY_DATA_TYPE: NO_DISPLAY`. No parsing change can produce a map for these; the
  geometry has to be derived from the distance matrix, and the page has to say so.

While measuring the current output, a second problem surfaced: the inline SVG is proportional to
the city count, so `pla85900` is a **6.2 MB** page (85,900 `<circle>` elements) and `docs/problems/`
is 19.6 MB of markdown in total.

## Goals

- Every one of the 111 problem pages shows a minimap.
- Where the source publishes no coordinates, the map is a derived layout and is labelled as one —
  on the image (`aria-label`) and in visible text.
- A large-instance page is not multiple megabytes of inline markup, without losing the structure the
  current rendering shows.
- The 94 maps that already work are byte-for-byte unchanged, so the diff is only the instances that
  gain or change an image.

## Decisions already made

**Derived layout = classical MDS over the distance matrix, clearly labelled.** Not a placeholder, not
a heatmap. This is the same kind of object TSPLIB itself publishes: scoring classical MDS against
the `DISPLAY_DATA_SECTION` of the five instances that have *both* a matrix and display data gives
r = 0.998 (bayg29), 0.986 (dantzig42), 0.971 (gr120), 0.953 (pa561). So for the 12 matrix-only
instances, MDS produces what TSPLIB would have published had it published anything. Runtime is
negligible (si1032, the largest at 1,032 cities, takes ~1.1 s).

**Large instances render as a density grid, not as thinned points.** Taking every Nth dot is *not*
an option: at `pla85900` a stride of 29 turns the structured grid into noise. Bucketing the projected
points into 2 px cells and drawing one `<rect>` per non-empty cell preserves the visible structure
and takes the page from 5.79 MB to 0.35 MB.

**Keep the inline-SVG-in-markdown delivery.** The generator writes the SVG into the page body; there
is no runtime cost and no new component. Only `scripts/gen-problem-pages.mjs` and the regenerated
`docs/problems/*.md` change, so the diff does not collide with
[[tasks/bench-problem-leaderboard]], which is adding a leaderboard to the same Astro page.

**No new unit tests.** The generator has none today; the chosen delivery keeps that as-is. Correctness
is pinned by the regeneration being byte-reproducible (see Acceptance Criteria) rather than by a new
harness.

## Workstreams

Each is a separate commit; the file is regenerated from scratch each time, so they are independently
revertable.

### 1. Read `DISPLAY_DATA_SECTION` (5 pages)

- Generalise `parseCoords` into a point loader that tries `NODE_COORD_SECTION`, then
  `DISPLAY_DATA_SECTION`.
- Drop the blanket `EXPLICIT`/`MATRIX` exclusion in favour of "no points found ⇒ no map", so a
  coordinate-less instance falls through to workstream 2 instead of silently producing nothing.
- Result: `bayg29`, `bays29`, `dantzig42`, `gr120`, `pa561` get a normal map.

### 2. Derived layout for the 12 matrix-only instances

- Parse `EDGE_WEIGHT_SECTION` by `EDGE_WEIGHT_FORMAT` (`UPPER_ROW`, `LOWER_DIAG_ROW`,
  `UPPER_DIAG_ROW`, `FULL_MATRIX`) into the squared-distance matrix; require the values to be
  present in the expected count, and fail loudly rather than emit a wrong picture.
- Classical MDS (double-centre, top-2 eigenvectors by power iteration with deflation) → 2-D points.
- Reuse the existing projection/`viewBox`/dot styling so the map does not look like a different
  feature.
- Mark it as derived: `aria-label` and a one-line visible note (e.g. "Layout derived from the
  distance matrix — this instance publishes no coordinates").
- Result: all 12 pages get a map; the note is the disclaimer that keeps the picture honest.

### 3. Density grid above the size threshold

- Above ~3,000 cities, project and bucket into 2 px cells; emit one `<rect>` per non-empty cell with
  opacity scaled by `log1p(count)`.
- Below the threshold, keep today's circles untouched.
- Instances affected: `pla85900`, `pla33810`, `d18512`, `d15112`, `brd14051`, `usa13509`, `rl11849`,
  `pla7397`, `rl5934`, `rl5915`, `fnl4461`, `fl3795`, `pcb3038`.
- Tune the opacity floor against the current rendering before/after, so the change reads as a weight
  fix rather than a restyle.

## Acceptance Criteria

- [x] `node scripts/gen-problem-pages.mjs` on a clean master produces a zero-byte diff (proves the
      generator is byte-reproducible before the change; verified once already). Re-running it on the
      branch also produces no diff.
- [x] After the change, all 111 `docs/problems/*.md` contain exactly one `<svg … role="img">`
      (checked: none missing, none with two).
- [x] The 94 pages that already had a map are byte-identical to master (`git diff --stat` shows only
      the 17 previously-empty pages, plus the 13 density-grid pages). The diff against master is
      exactly 30 files under `docs/problems/`, and the YAML frontmatter of all 111 pages is
      byte-identical.
- [x] The 5 `DISPLAY_DATA_SECTION` instances use their published coordinates — not MDS output.
      Recomputing the projection from `DISPLAY_DATA_SECTION` reproduces every one of their 781
      circles exactly.
- [x] Each of the 12 matrix-only pages carries the derived-layout note in its body (grep: 12).
- [x] No map is emitted for an instance whose matrix cannot be parsed; the generator exits non-zero
      instead of writing a partial or wrong image. The main loop now builds every page before
      writing any.
- [x] Page weight: every page ≤ 0.5 MB. Measured: `pla85900` 6.22 → 0.40 MB, largest page
      (`rl11849`) 0.48 MB, top-10 15 → 2.95 MB, total 19.6 → 6.90 MB. The original targets here
      ("top-10 under 2 MB", "total under 4 MB") were guesses made before measuring: the grid's cost
      is the number of *occupied cells*, not the point count, so the win is bounded by how spread
      out an instance is — `rl11849`'s 11,849 cities occupy ~5,000 cells and still cost 0.48 MB.
      Reaching those numbers would mean a coarser cell or a lower threshold, i.e. a worse picture
      for a saving that does not matter once no page is in the megabytes.
- [x] `task web:build` succeeds and `/problems/berlin52/`, `/problems/bayg29/`, `/problems/gr48/`,
      `/problems/si1032/` and `/problems/pla85900/` render a map in `dist/` (astro check + tsc +
      165 pages; largest built problem page 483 KB).
- [x] Visual check of one page per workstream against the previews captured in this task.

## Known limitations

- A derived layout is a projection, not geometry. Measured fit (correlation of the layout's pairwise
  distances against the true matrix distances) is r ≥ 0.96 for ten of the twelve, 0.924 for si175 —
  and **0.233 for brg180**, whose matrix holds six distinct values (0, 20, 30, 3500, 9000, 10000).
  Neither a rank-transformed MDS (0.252) nor any 2-D embedding recovers its distances; what the
  picture shows is its cluster structure, which is why the note on the page matters.
- `si175`, `si535` and `si1032` are `UPPER_DIAG_ROW` instances teeline's own parser cannot read (see
  [[tasks/bench-tier-a-campaign]]). The generator reads them, so their pages work; the solver still
  cannot open them.


## Context

- Generator: `scripts/gen-problem-pages.mjs` — invoked manually, referenced by nothing in CI or the
  Taskfiles.
- Visually verified on the live site: maps already exist for GEO (`ali535`) and ATT (`att48`); they
  do not for `EXPLICIT` (`bayg29`, `gr48`).
- Overlap: [[tasks/bench-problem-leaderboard]] edits `src/pages/problems/[id]/index.astro`. This task
  deliberately touches neither that file nor `src/`, so the two do not conflict.
- Scratch previews used for the decisions (real coords vs published display data vs MDS; all-dots vs
  density grid) live in `.worktrees/problem-minimaps/.scratch/` and are not committed.

## Progress Log

### 2026-10-11 — implemented on `feat/problem-minimaps`

Three commits, each regenerating the pages from scratch so they revert independently:

| Commit | Change | Pages |
|---|---|---|
| `245f6f01` | read `DISPLAY_DATA_SECTION` when there is no `NODE_COORD_SECTION` | 5 |
| `3116bbe9` | classical MDS for the matrix-only instances, labelled as derived | 12 |
| `8f1ceee0` | density grid past 3,000 cities | 13 |

Things worth remembering when picking this up:

1. **The corrected deflation matters.** The prototype's deflation re-added the eigenvalue shift to
   the diagonal after each axis, which leaves eigenvectors right but scales the second axis wrongly.
   Fixing it moved the match against TSPLIB's published display data to r = 1.000 (`bayg29`),
   0.994 (`dantzig42`), 0.986 (`bays29`) — `bayg29` landing exactly on 1.000 is what says the MDS is
   faithful rather than merely plausible.
2. **The failure mode is a wrong picture, not a missing one.** A matrix parsed with the wrong
   `EDGE_WEIGHT_FORMAT` still lays out *something*, so `parseDistanceMatrix` checks the value count
   against the format's expected shape and returns null rather than drawing.
3. **`brg180` is the known bad case** (r = 0.233). It is not fixable by a better embedding; its
   matrix is six discrete values. If a future reader distrusts the derived maps, this is the one to
   look at first, and the reason the note is on the page rather than only in the `aria-label`.

### 2026-10-11 — planned, not implemented

- Created the branch/worktree `feat/problem-minimaps` (`.worktrees/problem-minimaps`).
- Established that regeneration is byte-reproducible, and that the gap is `EXPLICIT` only.
- Validated MDS against TSPLIB's own `DISPLAY_DATA_SECTION` (r = 0.95–0.998).
- Measured the page-weight problem and rejected stride sampling in favour of a density grid.

## Completion Evidence

**Superseded.** PR #581 (this task's implementation, inline SVG in the markdown) was closed
unmerged; the work shipped as [[tasks/problem-minimaps-as-r2-assets]] — **PR #584, merged** — with
images in R2, the 111-page review problem solved. Its commits were `245f6f01`, `3116bbe9`,
`8f1ceee0` on `feat/problem-minimaps` (now deleted).
- Verification: generator re-run is a zero-byte no-op; diff vs `master` is exactly 30 files under
  `docs/problems/` plus `scripts/gen-problem-pages.mjs`; frontmatter identical on all 111 pages;
  111/111 pages carry exactly one `<svg role="img">`; the 781 circles of the five
  `DISPLAY_DATA_SECTION` pages reproduce their published coordinates exactly; `npm run build`
  passes (astro check + tsc + 165 pages) with the largest built problem page at 483 KB.
- Visual: before/after captured for the density grid (`pla85900`, `rl11849`, `d18512`, `fnl4461`,
  `pcb3038`, `usa13509`) and a full-page screenshot of the built `gr48` and `pla85900` pages.

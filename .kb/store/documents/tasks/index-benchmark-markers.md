---
id: 01a12b6c-7972-75a3-8d90-7b09d57a8172
slug: tasks/index-benchmark-markers
title: "feat(web): mark instances and solvers that have published results on the index pages"
type: task
status: active
priority: medium
tags: [benchmarks, teeline-web, frontend]
---

## Overview

The index pages list everything and say nothing about what has been measured: `/problems/` renders
111 instances and `/algorithms/` the 21 documented solvers, while `/bench/v2/` currently publishes
12 instances and 22 solver shards. A reader who wants to know where the results are has to open
pages one at a time. This marks the rows that have published results, on both indexes.

## Source of truth — the part that matters

A marker means **published**, and the only thing that says published is the manifest:
`index.json` names the datasets (`problems[]`) and the solvers (`solvers[]`). Two tempting
shortcuts are wrong:

- **`bench/matrix/tier-a.toml` is intent, not fact.** It lists the datasets the campaign plans to
  measure. A marker built from it would claim results for instances that have none, and would keep
  claiming them after a publish withdraws an object — the publisher retires things by *omitting*
  them from the manifest while the shard keeps answering `200` (see
  [[tasks/bench-problem-leaderboard]] for that trap).
- **A build-time manifest fetch breaks the site's rules.** The build must succeed with R2
  unreachable, and republishing must not need a redeploy (decision 9). Every other benchmark view
  already fetches at runtime.

## How

- **The pages stay server-rendered.** 111 rows and 21 rows are content: a reader without JS gets
  all of them, and search engines keep seeing the list.
- Each row carries `data-dataset` / `data-solver`. One small client script fetches the manifest
  **once per page**, badges the rows it names, links each badge to that page's results section
  (`/problems/<id>/#results` and `/algorithms/<id>/#benchmarks` — both anchors already exist), and
  states the count, e.g. "12 of 111 instances have published results".
- Precedent for a page-level `<script>` doing progressive enhancement in this codebase:
  `Topbar.astro`, `LandingHero.astro`, `src/pages/solve/index.astro`.
- **Manifest unreachable → no badges at all.** Absence of a badge means "not published as far as we
  could tell", never "measured and empty". The count line is omitted rather than guessed.

## What a marker can and cannot say

It is binary. The manifest carries no per-instance or per-solver result counts, so "20 solvers" or
"5 runs" would cost 12 extra shard fetches, or a publisher change to carry counts in the manifest.
Neither is in this task.

## Acceptance Criteria

- [ ] Problem-index rows for instances the manifest names carry a badge; other rows do not
- [ ] Algorithm-index rows for solvers the manifest names carry a badge; other rows do not
- [ ] Each badge links to that page's results section
- [ ] Exactly one manifest request per page, not one per row
- [ ] Both pages render their full lists with JS disabled, and the build needs no network
- [ ] With the manifest unreachable, no badges appear and nothing claims a dataset has no results
- [ ] The count is stated and attributed to the manifest
- [ ] Unit test for "which ids does this manifest mark"; e2e for both indexes, including the
      unreachable-manifest case

## Open Questions

- Badge wording and shape: a "results" pill, a dot, or a check. The style should match the existing
  badges on the problem and algorithm pages.
- Should the group headings on `/problems/` also carry a count ("Small (≤100) — 6 with results")?
  Cheap once the rows are marked, and it helps a reader scan 111 rows.
- A filter ("only instances with results") is a bigger change: URL state plus hiding server-rendered
  rows. Not now.

## Notes

- Requested after the problem-page leaderboard shipped ([[tasks/bench-problem-leaderboard]]).
- `/algorithms/` marks solvers, which is the symmetric case; the manifest carries `solvers[]` with
  ids and families.

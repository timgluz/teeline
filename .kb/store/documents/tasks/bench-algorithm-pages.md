---
id: 01a12651-5646-7e10-ba40-b75b1a4fc3b2
slug: tasks/bench-algorithm-pages
title: "feat(web): render per-algorithm benchmark tables from published data"
type: task
status: draft
priority: high
tags: [benchmarks, teeline-web, frontend]
blocked_by: [tasks/bench-publish-provenance]
---

## Overview

Algorithm pages render no benchmarks. The only benchmark surface is the landing chart, fed by a
hand-maintained array in `teeline-web/src/lib/benchmark-data.ts` that is stale: 20 of 22 solvers,
missing `aco`, `bhk` and `shuffle`, and carrying an `hk` entry that corresponds to no solver.

`/bench/v2/` is now live, so this task builds the shared read layer and the algorithm page on top
of it. `tasks/bench-problem-leaderboard` then reuses the layer for the problem pages; the split is
because the two pages need different shapes (one solver across instances; one instance across
solvers) and the shared layer is the only thing worth landing first.

## What exists to build on

- `algorithms/[id]/index.astro` (42 lines) renders the docs entry through `DocsLayout`. The
  benchmark section is added here, not in the markdown, so no doc has to change.
- `LandingBenchmarks.astro` already draws an inline `<svg>` scatter by hand. **There is no charting
  dependency** and this task does not add one: same approach, one component.
- `upload.ts` fetches TSPLIB at runtime and switches base URL on `import.meta.env.DEV`;
  `astro.config.mjs` proxies `/tsplib` to `https://static.tspsolver.com`. Benchmarks need the same
  two things: a `benchBaseUrl(dev)` helper and a `/bench` proxy entry.
- The site is a static build. Runtime fetch is what lets data be republished without a redeploy
  (decision 9), so the section is a client island, not build-time.

## The shapes being rendered

`algorithms/{id}.json`:

```
solver, configs[], measurement{git_commit, git_commit_source, dirty, teeline_version,
  measured_at, environment{os,build,rust_version}},
results{dataset:{cities, optimal_cost, by_config{config:{runs, timeouts, errors,
  [gap_pct{median,best,worst,mean}], tour_cost{...}, wall_s{...}, [peak_rss_kb{...}],
  [optimal_cost]}}}}
```

Note `gap_pct` is **absent** when the instance has no known optimum — `berlin52`, `att48`,
`burma14`, `ch150`, `a280` and `dsj1000` are all null today. The table must therefore render
without a gap column rather than showing 0 or a blank that reads as one.

## Content

- **Summary line** — median gap and median wall time, with the budget and config stated inline, so
  a cross-family row is never read as like-for-like
- **Per-instance table** — instance, cities, distance type, optimal, gap (median, best), wall time
  (median), runs, timeouts
- **Graph** — gap against instance size, one point per instance, drawn as inline SVG. Same numbers
  as the table, chosen for shape: the table answers "what did it score here", the graph answers
  "how does it scale". Where an optimum is unknown the point is plotted on tour cost instead and
  the axis is labelled as such; it is never silently mixed with gaps.
- **Provenance footer** — the solver's own `measurement`: commit, version, environment, measured_at

## Honest rendering rules

Each of these is a way a benchmark page lies, and each is cheap to get right once:

- **Timeouts render as timeouts.** No gap and no tour cost for a killed run — there is no result to
  report (decision 5). Not a zero, not a blank cell that reads as one.
- **Non-deterministic solvers show their spread.** `tour_cost` carries median/best/worst; a page
  that prints only the median presents a distribution as a single fact.
- **A budget is never omitted.** A gap without the allowance it was measured under is
  uninterpretable, and cross-family gaps are not like-for-like (decision 4).
- **Provenance is per solver.** One solver may be measured at an older commit than another, and the
  footer says so per section rather than implying one build for the page.
- **Partial data is normal, not an error.** Two of 22 solvers are published today. The page renders
  whatever its own shard holds.

## Read path

`fetch` `algorithms/{id}.json` at runtime, mirroring `upload.ts`. A **404 is a normal state**, not
a failure: an unpublished solver, or a solver with no measurements yet, renders no benchmark
section and no error. Any other status is a real error and should be visible rather than swallowed.
Nothing may be required at build time (acceptance criterion below), because the build must not fail
when the data is unreachable.

## Acceptance Criteria

- [ ] Algorithm page renders summary, per-instance table, graph and provenance footer
- [ ] An algorithm with no published shard renders no benchmark section and does not error
- [ ] A solver measured at an older version displays that version honestly
- [ ] Cross-family comparison is not presented without its budget
- [ ] Timeouts render as timeouts; no gap cell is fabricated for them
- [ ] Non-deterministic solvers show a spread, never a single value presented as *the* result
- [ ] An instance with no known optimum renders without a gap column, and the graph says what it
      is plotting instead
- [ ] `BERLIN52_BENCHMARKS` deleted; the landing chart reads the published source
- [ ] The phantom `hk` entry and the missing `aco`/`bhk`/`shuffle` solvers resolved by the migration
- [ ] `tsc`, vitest and the web build pass
- [ ] No new dependency on network access at build time
- [ ] Unit tests cover the pure parts: parsing a shard, choosing table columns when `gap_pct` is
      absent, and formatting a timeout — the rendering rules above are what regress silently

## Notes

- `src/lib/solver-index.test.ts` already enforces that `bench/solvers.json` and `nav-data.ts`
  agree; keep that invariant intact.
- The dev proxy needs `/bench` alongside `/tsplib`, or local development silently renders no
  benchmarks and looks like a bug in the component.

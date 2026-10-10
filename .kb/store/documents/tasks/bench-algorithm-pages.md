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

Algorithm pages currently render no benchmarks at all. The only benchmark surface is the landing
page chart, fed by a hand-maintained array in `teeline-web/src/lib/benchmark-data.ts` that is
stale: 20 of the 22 solvers, missing `aco`, `bhk` and `shuffle`, and carrying an `hk` entry that
corresponds to no solver.

## Goals

- Every algorithm page shows its own measured results
- One source of benchmark truth; delete the hand-maintained array
- State the version and environment that produced each number

## Content per page

- **Summary line** — median gap and median wall time across the tier, with the budget and config
  stated inline, so a cross-family row is never read as like-for-like
- **Per-instance table** — instance, cities, distance type, optimal, gap (median, best),
  wall time (median), runs, timeouts
- **Provenance footer** — the solver's own measurement commit, version and environment

## Read path

Fetch `algorithms/{id}.json` at runtime from the published location, mirroring the existing
TSPLIB fetch in `teeline-web/src/upload.ts` (with the dev Vite proxy). The site is a static
build, so runtime fetch is what allows benchmark data to be republished without a redeploy; an
absent shard must degrade to "no benchmark section" rather than a build failure.

## Acceptance Criteria

- [ ] Algorithm page renders summary, per-instance table and provenance footer
- [ ] An algorithm with no published shard renders no benchmark section and does not error
- [ ] A solver measured at an older version displays that version honestly
- [ ] Cross-family comparison is not presented without its budget
- [ ] Timeouts render as timeouts; no gap cell is fabricated for them
- [ ] Non-deterministic solvers show a spread, never a single value presented as *the* result
- [ ] `BERLIN52_BENCHMARKS` deleted; landing chart reads the published source
- [ ] The phantom `hk` entry and the missing `aco`/`bhk`/`shuffle` solvers resolved as part of
      the migration
- [ ] `tsc`, vitest and the web build pass
- [ ] No new dependency on network access at build time

## Notes

`teeline-web/src/lib/solver-index.test.ts` already enforces that `bench/solvers.json` and
`nav-data.ts` agree; keep that invariant intact.
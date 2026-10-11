---
id: 01a12b43-c3f8-7071-b11a-8f3a2d452632
slug: tasks/ci-runs-only-auth-e2e
title: "ci(web): run the web e2e suite in CI, not only the auth specs"
type: task
status: active
priority: high
tags: [ci, teeline-web, testing]
---

## Overview

`.github/workflows/web-e2e.yml` runs `npm run test:e2e:auth`, which is `playwright test --config
playwright.auth.config.ts`. Every other spec in `teeline-web/tests/` — the algorithm benchmark
section, the landing chart, the problem-page leaderboard, the solver comparison, `webmcp` — runs
only on a developer's machine. A green `e2e` check on a pull request therefore means "the auth
flows still work", not "the site still works".

## Evidence

Running the full suite locally against `npm run dev` (the same server the committed
`playwright.config.ts` spawns) fails five specs that no recent change touched:

```
tests/webmcp.spec.ts:56  4 WebMCP forms are present in the DOM with toolname attributes
tests/webmcp.spec.ts:77  WebMCP tool invocation › listAlgorithms ...
tests/webmcp.spec.ts:91  ... parseProblem ...
tests/webmcp.spec.ts:105 ... solveTSP ...
tests/webmcp.spec.ts:115 ... compareTours ...
```

`webmcp.spec.ts` navigates to `/` and expects four `form[toolname]` elements. Those forms live in
`src/pages/solve/index.astro` (four occurrences, confirmed in the served HTML of `/solve/`), and
neither `src/pages/index.astro` nor the layouts mention `toolname` anywhere. The spec has been
stale since the forms left the home page, and nothing noticed because nothing runs it.

## Acceptance Criteria

- [ ] CI runs the non-auth specs, so a change to a benchmark section, a chart or a problem page
      cannot merge with a broken page
- [ ] The stale `webmcp` specs are fixed (pointed at `/solve/`) or deleted, with the reason recorded
- [ ] Specs that read published shards keep routing them from `src/lib/__fixtures__`, so the suite
      does not depend on R2 being reachable
- [ ] The job stays inside a reasonable time budget, reusing the same dev server as the auth job

## Notes

- `npm run test:e2e` (`playwright test`) already exists and is what a developer runs; the workflow
  needs to call it, split it, or add a project that excludes `tests/auth/**`.
- The auth job exists because those specs need a Cloudflare/D1 dev server, so whatever split is
  chosen the auth job keeps its own config.

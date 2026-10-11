---
id: 01a12b43-c3f8-7071-b11a-8f3a2d452632
slug: tasks/ci-runs-only-auth-e2e
title: "ci(web): run the web e2e suite in CI, not only the auth specs"
type: task
status: completed
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

- [x] CI runs the non-auth specs, so a change to a benchmark section, a chart or a problem page
      cannot merge with a broken page — a second job, `e2e-web`, runs them against `astro dev`.
- [x] The stale `webmcp` specs are fixed: the four `form[toolname]` elements and the
      `.status-dot--ready` wait both moved to `/solve/`, which is where the solver UI lives.
- [x] Specs that read published shards keep routing them from `src/lib/__fixtures__`, so the suite
      does not depend on R2 being reachable — checked: every spec that touches published data
      (`algorithm-benchmarks`, `index-benchmark-markers`, `landing-benchmarks`, `problem-minimaps`,
      `problem-results`) stubs it with `page.route`, and none asserts on it without a stub.
- [x] The job stays inside a reasonable time budget. It runs one project rather than two, and
      `CI=true npx playwright test --project=chromium` is 75 passed in 40s including a fresh dev
      server; with `npm ci` and the browser install the job should be 1-2 minutes. The auth job is
      untouched, so "reusing the same dev server as the auth job" was the wrong shape — they need
      different servers, which is why they are separate jobs.

## Progress Log

### 2026-10-11 — implemented on `ci/run-web-e2e`

- PR: <https://github.com/timgluz/teeline/pull/586>
- Full local suite before the fix: 138 passed, **12 failed** (10 webmcp across two projects, and 2
  from the minimap spec's fallback SVG, which had no intrinsic size and so only passed where the
  staging dir existed).
- After: 150 passed in 57s with both projects.
- Two config values were also one slow machine from failing on their own: `bhk`'s DP run takes
  ~26s against the 30s per-test timeout, and a cold Vite optimisation can exceed the 30s web-server
  timeout, which aborts the run before the first test. Both raised, and retries enabled in CI so an
  animation flake is reported as flaky rather than hidden.

## Notes

- `npm run test:e2e` (`playwright test`) already exists and is what a developer runs; the workflow
  needs to call it, split it, or add a project that excludes `tests/auth/**`.
- The auth job exists because those specs need a Cloudflare/D1 dev server, so whatever split is
  chosen the auth job keeps its own config.

### 2026-10-11 — status: in review

Implemented and green on the PR: `e2e` (auth) 42s, `e2e-web` 75 passed / 1m50s. Marked `review`
rather than `completed` until it merges — the criteria in the body are met on the branch, not on
master.

### 2026-10-11 — merged; `e2e-web` runs on every web PR

The branch merged, and the job is live: PR #587's checks show `e2e-web` passing in 1m49s alongside
the auth `e2e` job, which is the evidence that the non-auth suite is no longer skipped. Two of its
findings were real bugs in the suite rather than the workflow — five WebMCP specs asserted the home
page for forms and a WASM status dot that live on `/solve/`, and the cards spec's stub had no
intrinsic size — and the job provisions the released WASM component so the first of those cannot
silently pass on a fresh checkout.

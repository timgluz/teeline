---
id: 01a12b43-c414-7e02-b8d9-42141ae3f5ee
slug: tasks/type-check-the-islands
title: "chore(web): type-check the Preact islands"
type: task
status: completed
priority: high
tags: [teeline-web, typescript, testing]
---

## Overview

`teeline-web/tsconfig.json` excludes `src/components/**` (along with `src/pages/**` and
`src/layouts/**`), so `npx tsc --noEmit` never looks at the Preact islands. `astro check` — the
check the pre-commit hook and CI rely on — does **not** report an undefined identifier inside a
`.tsx` component either. The islands are the largest hand-written part of the site (the benchmark
section, the landing chart and the problem leaderboard are together over a thousand lines) and they
are not statically checked.

## Evidence

Found while building the problem leaderboard: a helper was renamed in
`src/components/AlgorithmBenchmarks.tsx` and one call site kept the old name. Both `npx tsc
--noEmit` and `npx astro check` reported **0 errors**, with the stale call inside the module-level
`buildComparison` — a guaranteed `ReferenceError` the first time a reader asked for a comparison.
A grep found it, and the Playwright spec that exercises the comparison would have caught it.

Reproduced deliberately afterwards: replacing a call with `definitelyNotDefined(...)` and re-running
`astro check` still reports `0 errors, 0 warnings, 0 hints` over 92 files.

## Acceptance Criteria

- [x] Adding an undefined reference to a `.tsx` island fails a check that CI runs
- [x] The islands are type-checked, or the reason they cannot be is recorded here
- [x] The new check is wired in beside the existing `tsc`/prettier checks, so it cannot be skipped
- [x] Existing islands reach zero errors rather than the check shipping with a suppression list

## Notes

- The guess in this note was half right. `astro check` does honour the `exclude` list — so removing
  `src/components/**` from it fixed both checks at once, and no second `tsconfig` was needed. What
  the note missed: the exclusion was hiding an **`.astro`** error as well (`LandingHero.astro` keyed
  its `<line>` elements, which Astro's JSX types reject), which only `astro check` could see.
- Preact's JSX types are already available (`jsxImportSource: preact`), so this is about including
  the files, not about finding types.

## Progress Log

### 2026-10-11 — fixed in #580

The fix is one line of config rather than a new check: `"src/components/**"` came out of `exclude`,
so the `tsc --noEmit` that CI's `Lint (tsc)` step, the pre-commit hook and `npm run build:check`
already run now covers the islands. Pages, layouts and content stay excluded — they need the
`astro:*` virtual modules — and that reasoning is a comment in `tsconfig.json` now.

Four errors had been hiding there, which is why this is a small PR rather than a rewrite: a
`children` prop typed `unknown` in `AlgorithmBenchmarks` and `ProblemResults` (not renderable), an
unused `ApiError` import in `ApiKeyManager`, and the keyed `<line>` elements in `LandingHero.astro`.

Evidence for the first criterion, produced by reintroducing the class of bug that started this
(a renamed helper leaving a call to the old name — the `buildComparison` incident):

```
src/components/AlgorithmBenchmarks.tsx(138,11): error TS2304: Cannot find name 'definitelyNotDefined'.
```

Before the change the same line passed `tsc --noEmit` **and** `astro check` with 0 errors.

Verification: `tsc` and `astro check` 0 errors; 569 unit tests; the full non-auth e2e suite 63
passed with the 5 pre-existing `webmcp` failures tracked in [[tasks/ci-runs-only-auth-e2e]]; 165-page
build. The `teeline-web` CI job (which owns the tsc step) ran green on the PR.

Mark this completed once #580 merges. The one thing still unguarded: someone could re-add the
exclusion and CI would stay green while the islands went dark again — a comment is the only defence
today, and a config assertion would be belt and braces if that is wanted.

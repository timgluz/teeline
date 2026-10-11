---
id: 01a12b43-c414-7e02-b8d9-42141ae3f5ee
slug: tasks/type-check-the-islands
title: "chore(web): type-check the Preact islands"
type: task
status: active
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

- [ ] Adding an undefined reference to a `.tsx` island fails a check that CI runs
- [ ] The islands are type-checked, or the reason they cannot be is recorded here
- [ ] The new check is wired in beside the existing `tsc`/prettier checks, so it cannot be skipped
- [ ] Existing islands reach zero errors rather than the check shipping with a suppression list

## Notes

- The likely cause is that `astro check` reports diagnostics only for `.astro` files while the
  project's `tsconfig` keeps the component directory out of the program it builds. A second
  `tsconfig` including `src/components`, `src/lib` and `src/nav-data.ts` run with `--noEmit` in CI
  would close the gap without touching the Astro-side config.
- Preact's JSX types are already available (`jsxImportSource: preact`), so this is about including
  the files, not about finding types.

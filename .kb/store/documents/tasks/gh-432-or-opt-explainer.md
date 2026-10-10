---
id: 01a00bed-75e5-7251-ab45-89a53088d79a
slug: tasks/gh-432-or-opt-explainer
title: "feat(web): interactive Or-opt explainer (GH #432)"
type: task
status: completed
priority: high
tags: [teeline-web, explainer, preact, or-opt]
---

## Overview

Interactive explainer for Or-opt local search on `https://tspsolver.com/algorithms/or_opt/explainer/`. Faithful TypeScript port of `src/tsp/or_opt.rs`: relocation moves (Or-1 / Or-2 / Or-3) of 1–3 consecutive cities, best-improvement scan per pass, reversed insertions for length ≥ 2.

## Goals

- Rust-faithful best-improvement scan (`scanBestMove`) with the same `-1e-3` acceptance threshold as the Rust solver
- Relocation rendering: highlight the removed segment and its insertion point, show the tour mutation per move
- Scenario that demonstrates Or-opt beating plain 2-opt (the classic "two_opt_stuck" case)

## Acceptance Criteria

- [x] `or-opt-algo.ts` pure simulation; `scanBestMove` starts `bestDelta = -1e-3` and only updates on strict improvement (mirrors Rust)
- [x] `applyRelocation(tour, i, segLen: 1|2|3, j, reversed)` handles all three segment sizes + reversal
- [x] `two_opt_stuck` scenario uses the actual 2-opt two-optimal tour as its starting tour (proves Or-opt escapes a 2-opt local optimum)
- [x] Vitest unit tests + Playwright e2e (`tests/or-opt.spec.ts`)
- [x] `ocr` review feedback applied; astro check + tsc + vitest + Playwright green

## Completion Evidence

- Commit: `fa7e8a8` — feat(web): add interactive Or-opt explainer (#432) (#469)
- PR: [#469](https://github.com/timgluz/teeline/pull/469) (merged)

## Files

| File | Change |
|------|--------|
| `teeline-web/src/explainers/or-opt-algo.ts` | New — pure simulation |
| `teeline-web/src/explainers/or-opt.tsx` | New — Preact component |
| `teeline-web/src/explainers/or-opt.test.ts` | New — vitest |
| `teeline-web/tests/or-opt.spec.ts` | New — Playwright e2e |
| `teeline-web/src/explainers/registry.ts` | `or_opt` entry |

## References

- [[tasks/gh-48-or-opt]] — the original Rust solver task
- GH issue #432

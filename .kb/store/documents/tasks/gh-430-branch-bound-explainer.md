---
id: 01a00bed-81ec-7ae3-8432-7e634f4b7342
slug: tasks/gh-430-branch-bound-explainer
title: "feat(web): interactive Branch & Bound explainer (GH #430)"
type: task
status: completed
priority: high
tags: [teeline-web, explainer, preact, branch-bound]
---

## Overview

Interactive explainer for Branch & Bound exact search on `https://tspsolver.com/algorithms/branch_bound/explainer/`. Faithful TypeScript port of `src/tsp/branch_bound.rs` (6 cities): DFS over a partial-tour search tree, MST-based lower bound per node, pruning when bound ≥ best, leaf = closed tour.

## Goals

- Render the search tree live: node = partial tour prefix, shown pruned when the bound kills it; leaf events carry the closed-cycle cost
- MST lower-bound computation visualized per node; candidate ordering approximates bound-ascending expansion (as in Rust)
- Demo that pruning is what makes exact search feasible on small instances

## Acceptance Criteria

- [x] `branch-bound-algo.ts` pure simulation; `bestTour` is a **closed cycle** (`[...path, state.startCity]`) and leaf events print the closed cycle (`const closed = [...path, state.startCity].join('→')`) — fix applied after user feedback "TSP is about closed circles"
- [x] `mstCost` guards `u === -1` (early return) — matches Rust `if (u == -1) return total`
- [x] Candidate sort by `dm[prev][a] - dm[prev][b]` with comment that it approximates bound-ascending
- [x] Tree container `height: 740px` (fixed, not max-height) with scroll; minimap moved into the right panel; `.bb-mt6/.bb-mt10` utility classes (no inline styles)
- [x] SSR-safe first render (chip guard `s.mstRevealed === 0` — `mstEdges[-1]` crash)
- [x] 14 vitest tests incl. random-instance brute-force sweep + closed-cycle assertions; Playwright e2e (`tests/branch-bound.spec.ts`) with hydration-resilient `waitHydrated` probes
- [x] `ocr` review feedback applied; astro check + tsc + vitest + Playwright green

## Completion Evidence

- Commit: `e3fd1b3` — feat(web): add interactive Branch & Bound explainer (#430) (#474)
- PR: [#474](https://github.com/timgluz/teeline/pull/474) (merged)

## Files

| File | Change |
|------|--------|
| `teeline-web/src/explainers/branch-bound-algo.ts` | New — pure simulation |
| `teeline-web/src/explainers/branch-bound.tsx` | New — Preact component |
| `teeline-web/src/explainers/branch-bound.test.ts` | New — vitest |
| `teeline-web/tests/branch-bound.spec.ts` | New — Playwright e2e |
| `teeline-web/src/explainers/registry.ts` | `branch_bound` entry |

## References

- GH issue #430

---
id: 01a00bed-820a-70c3-be36-a94003cd7bfe
slug: tasks/gh-429-bhk-explainer
title: "feat(web): interactive Bellman-Held-Karp explainer (GH #429)"
type: task
status: completed
priority: high
tags: [teeline-web, explainer, preact, bhk]
---

## Overview

Interactive explainer for the Bellman–Held–Karp dynamic program on `https://tspsolver.com/algorithms/bhk/explainer/`. Faithful TypeScript port of `src/tsp/bellman_karp.rs` (6 cities): DP over subsets with city 0 as the fixed start, `m = n-1` rows × `2^m` columns, row-by-row fill that mirrors the Rust fill order.

## Goals

- Animate the DP table fill: one cell per step, showing `dp[subset][end] = min over last-hop k` with the predecessor reconstruction
- Highlight the difference between BHK's exact exponential cost and the heuristic solvers (2^n · n² table vs. polynomial search)
- 6 cities so the full 32-column table is legible on screen

## Acceptance Criteria

- [x] `bhk-algo.ts` pure simulation; `computeFillOrder` starts at subset size 2 (size-1 base pre-filled, as in Rust); `computeRoute` guards `end < 0`
- [x] `makeDm` imported **and** re-exported from `explainer-cities` (bare `export { x } from` does not create a local binding — TS2304)
- [x] Vitest unit tests (fill order, DP values vs brute force, route reconstruction) + Playwright e2e (`tests/bhk.spec.ts`)
- [x] e2e Back button uses `getByRole('button', { name: '⏴ Back' })` — plain `'Back'` also matched the "Read-back" panel
- [x] `ocr` review feedback applied; astro check + tsc + vitest + Playwright green

## Completion Evidence

- Commit: `8352111` — feat(web): add interactive Bellman-Held-Karp explainer (#429) (#475)
- PR: [#475](https://github.com/timgluz/teeline/pull/475) (merged) — this squash also absorbed the earlier #473 menubar work (see [[tasks/gh-476-algorithms-index]])

## Files

| File | Change |
|------|--------|
| `teeline-web/src/explainers/bhk-algo.ts` | New — pure simulation |
| `teeline-web/src/explainers/bhk.tsx` | New — Preact component |
| `teeline-web/src/explainers/bhk.test.ts` | New — vitest |
| `teeline-web/tests/bhk.spec.ts` | New — Playwright e2e |
| `teeline-web/src/explainers/registry.ts` | `bhk` entry |

## References

- GH issue #429

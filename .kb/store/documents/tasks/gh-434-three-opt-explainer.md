---
id: 01a00bed-7604-73b1-b4b7-14a3f9e6f118
slug: tasks/gh-434-three-opt-explainer
title: "feat(web): interactive 3-opt explainer (GH #434)"
type: task
status: completed
priority: high
tags: [teeline-web, explainer, preact, three-opt]
---

## Overview

Interactive explainer for the 3-opt local search on `https://tspsolver.com/algorithms/3opt/explainer/`. Faithful TypeScript port of `src/tsp/three_opt.rs` including all 7 move cases (2-opt reversals plus the four 3-opt "reconnect" patterns with one or two reversed segments).

## Goals

- Port all 7 3-opt cases exactly: `scanBestMove` enumerates every (i, j, k) triple and each case's delta; `apply3Opt` reconstructs the tour via `head / s1 / s2 / tail`
- Visualize each case with a hexagon pattern diagram (which edges are removed/added per case) so the 7 cases become legible
- Show per-pass best-improvement behavior matching the Rust solver

## Acceptance Criteria

- [x] `three-opt-algo.ts` pure simulation; `scanBestMove` covers cases 1–7 with the same deltas as Rust
- [x] `apply3Opt` cases 1–7 with `head/s1/s2/tail` reconstruction; `caseNo` range guard (clamp to 1..=7)
- [x] Hexagon pattern-diagram cells in the tsx (removed edges in red, added in green) with per-case label
- [x] Vitest unit tests + Playwright e2e (`tests/three-opt.spec.ts`)
- [x] `ocr` review feedback applied (incl. a stochastic-hill step-race fix surfaced during this round); astro check + tsc + vitest + Playwright green

## Completion Evidence

- Commit: `ac41b21` — feat(web): add interactive 3-opt explainer (#434) (#470)
- PR: [#470](https://github.com/timgluz/teeline/pull/470) (merged)

## Files

| File | Change |
|------|--------|
| `teeline-web/src/explainers/three-opt-algo.ts` | New — pure simulation |
| `teeline-web/src/explainers/three-opt.tsx` | New — Preact component |
| `teeline-web/src/explainers/three-opt.test.ts` | New — vitest |
| `teeline-web/tests/three-opt.spec.ts` | New — Playwright e2e |
| `teeline-web/src/explainers/registry.ts` | `3opt` entry |

## References

- GH issue #434

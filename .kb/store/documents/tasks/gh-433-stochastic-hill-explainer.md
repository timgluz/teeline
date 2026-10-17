---
id: 01a00bed-75c8-77c0-8903-67d87b300df3
slug: tasks/gh-433-stochastic-hill-explainer
title: "feat(web): interactive Stochastic Hill Climbing explainer (GH #433)"
type: task
status: completed
priority: high
tags: [teeline-web, explainer, preact, stochastic-hill]
---

## Overview

Interactive explainer for Stochastic Hill Climbing (8 cities) on `https://tspsolver.com/algorithms/stochastic_hill/explainer/`. The simulation is a faithful TypeScript port of `src/tsp/stochastic_hill.rs` (move semantics, acceptance threshold `-1e-3`, random-restart behavior), so the demo behaves like the real Rust solver.

## Goals

- Rust-faithful stochastic-hill simulation: random 2-opt-style moves, accept strictly-better (threshold `-1e-3`), patience/epoch budget, random restarts
- Interactive controls: Play/Pause, Step, Reset, epoch/patience readouts, current-best tour highlight
- 3 curated scenarios (quick_convergence, rugged, needle) with fixed seeds so the demo is deterministic and each scenario tells a different story
- 8 cities (per user directive: reduce city count if epochs take too long) — known optimum 827.774076596031

## Acceptance Criteria

- [x] `stochastic-hill-algo.ts` pure simulation (no DOM), fully deterministic given seed
- [x] `stochastic-hill.tsx` Preact component with inline CSS (matches the other explainers' white "island" card style)
- [x] Vitest unit tests (`stochastic-hill.test.ts`) — sim state, move application, scenario determinism
- [x] Playwright e2e (`tests/explainer-stochastic-hill.spec.ts`) — hydration, Run/Step/Reset, scenario switching, final tour cost matches sim
- [x] Registry entry + `hasExplainer: true` in `docs/algorithms/stochastic-hill.md`
- [x] `ocr` review feedback applied; astro check + tsc + vitest + Playwright green

## Key Design Decisions

- **Seed-based RNG**: `RngState{seed, counter}` with a pure `nextRand` (mulberry32-style) — no `Math.random()`, so every run is reproducible and tests can assert exact tour sequences
- **Scenarios**: `quick_convergence` (seed 371, initTour `[2,1,0,3,4,5,6,7]`, epochs 30 / patience 25), `rugged` (seed 917), `needle` (seed 1548) — quick_convergence shows fast improvement from a poor start; needle shows the algorithm's weakness on a narrow optimum
- **Guard against late events**: `stepForward` returns early when `phase === 'done'` (avoids playing queued steps after completion — race found during 3-opt work)
- **Button gating**: Reset / scenario buttons `disabled={running}` so a running sim can't be desynced mid-epoch

## Completion Evidence

- Commit: `7115c57` — feat(web): add interactive Stochastic Hill Climbing explainer (#433) (#468)
- PR: [#468](https://github.com/timgluz/teeline/pull/468) (merged)
- astro check 0 errors, `tsc --noEmit` clean, vitest + Playwright (chromium via `.pw-browsers`) all green

## Files

| File | Change |
|------|--------|
| `teeline-web/src/explainers/stochastic-hill-algo.ts` | New — pure simulation |
| `teeline-web/src/explainers/stochastic-hill.tsx` | New — Preact component |
| `teeline-web/src/explainers/stochastic-hill.test.ts` | New — vitest |
| `teeline-web/tests/explainer-stochastic-hill.spec.ts` | New — Playwright e2e |
| `teeline-web/src/explainers/registry.ts` | `stochastic_hill` entry |

## References

- [[tasks/gh-45-lin-kernighan]] — explainer pattern precedent
- GH issue #433

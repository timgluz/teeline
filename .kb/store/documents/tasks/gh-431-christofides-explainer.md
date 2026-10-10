---
id: 01a00bed-81cb-7e93-87ed-ad219b25b15d
slug: tasks/gh-431-christofides-explainer
title: "feat(web): interactive Christofides explainer (GH #431)"
type: task
status: completed
priority: high
tags: [teeline-web, explainer, preact, christofides]
---

## Overview

Interactive explainer for the Christofides ≤1.5× approximation algorithm on `https://tspsolver.com/algorithms/christofides/explainer/`. Faithful TypeScript port of `src/tsp/christofides.rs`: Prim MST → odd-degree vertices → greedy minimum matching → Eulerian multigraph → Hierholzer circuit → shortcut to a Hamiltonian tour.

## Goals

- Step through the full pipeline: MST, odd set, matching, multigraph, Euler tour, shortcut — each stage visualized on the map
- Show the ≤1.5× guarantee with a live ratio readout against the brute-forced optimum (≤10 cities)
- Curated scenarios that demonstrate the approximation ratio in different regimes

## Acceptance Criteria

- [x] `christofides-algo.ts` pure simulation: `primMst`, odd-degree collection, greedy matching, `doubledMstTourCost`, Hierholzer + shortcut; `bruteForceOpt` computes the true optimum (≤10 cities)
- [x] Ratio readout matches theory: `balanced` 1.107×, `near_optimal` 1.00× (circle layout), `matching_heavy` 50% matching share, `clustered`, `worst_case` 1.387×
- [x] Vitest unit tests (incl. ratio scenario assertions) + Playwright e2e (`tests/christofides.spec.ts`)
- [x] `ocr` review feedback applied; astro check + tsc + vitest + Playwright green

## Completion Evidence

- Commit: `9dbe4a4` — feat(web): add interactive Christofides explainer (#431) (#472)
- PR: [#472](https://github.com/timgluz/teeline/pull/472) (merged)

## Files

| File | Change |
|------|--------|
| `teeline-web/src/explainers/christofides-algo.ts` | New — pure simulation |
| `teeline-web/src/explainers/christofides.tsx` | New — Preact component |
| `teeline-web/src/explainers/christofides.test.ts` | New — vitest |
| `teeline-web/tests/christofides.spec.ts` | New — Playwright e2e |
| `teeline-web/src/explainers/registry.ts` | `christofides` entry |

## References

- GH issue #431

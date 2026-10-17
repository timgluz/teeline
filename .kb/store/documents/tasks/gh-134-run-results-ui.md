---
id: 019ea137-8f57-7720-b053-9dc4e25addfb
slug: tasks/gh-134-run-results-ui
title: "feat(teeline-web): Step 04 run, SVG canvas, results, run history (GH #134)"
type: task
status: completed
priority: high
tags: [teeline-web, ui, frontend]
---

## Overview

Wire the Run button end-to-end: WASM worker → SVG tour canvas → results table + run history. Completes the teeline-web wizard Steps 03/04.

## Goals

- `src/canvas.ts`: renderTour(svgEl, cities, route, optRoute?) draws SVG
- Run button triggers solve, shows "solving…" overlay, then renders result
- Results table: tour length, gap vs optimal, runtime (ms), iterations
- Run history panel: list of previous runs, click to re-apply
- "↻ try another solver" resets to Step 02 without clearing problem
- .opt.tour file content parsed and stored for gap calculation

## Acceptance Criteria

- [x] `src/canvas.ts` created with `renderTour` and `scaleCoords` (pure, testable)
- [x] `parseOptTour(text)` added to upload.ts and tested
- [x] `formatGap` and `formatRuntime` pure helpers in results.ts and tested
- [x] berlin52 + nn runs end-to-end; tour renders in SVG canvas
- [x] Optimal ghost path shown when .opt.tour loaded; gap % in results table
- [x] Runtime measured client-side (Date.now() delta)
- [x] Run history accumulates across re-runs
- [x] "↻ try another solver" resets solver state, keeps problem
- [x] All Vitest tests pass; TypeScript clean
- [x] Playwright: 4 browser verification scenarios

## Completion Evidence

- PR #157 merged 2026-06-07
- 57/57 Vitest tests passing at merge
- Playwright MCP: 5 scenarios verified in browser

## Progress Log

### 2026-06-07
- Implemented TDD: parseOptTour, scaleCoords, buildPolylinePoints, formatGap, formatRuntime
- Fixed step-03→04 transition bug in solver-form.ts
- Fixed run history index bug in results.ts
- Refactored wizard 4→3 steps
- **Merged PR #157** — all acceptance criteria met

---
id: 019ea11d-5980-7fe3-827a-113a54fd73a1
slug: tasks/gh-133-solver-config-ui
title: "feat(teeline-web): Step 02 solver config UI + Step 03 precondition checklist (GH #133)"
type: task
status: completed
priority: high
tags: [teeline-web, ui, frontend]
---

## Overview

Build Step 02 (solver selection + per-solver config form) and Step 03 (precondition checklist + Run button) for the teeline-web wizard. Follows PR #154 which completed Step 01 (file upload).

## Goals

- Solver selection: quick-pick pills (NN, 2-opt, SA, GA) + full dropdown (all 13 solvers)
- Per-solver config form: correct fields per solver with defaults from SolveOptions
- Precondition checklist in Step 03: problem loaded + solver selected + optional route
- Run button disabled until required preconditions met
- Branch: `feat/teeline-web-solver-config`

## Acceptance Criteria

- [ ] `src/solver-config.ts` created: solver metadata, param definitions, solverByAlias()
- [ ] All 13 solvers selectable via dropdown
- [ ] Config form renders correct fields per solver with defaults
- [ ] Precondition checklist reacts to app state
- [ ] Run button enabled/disabled correctly
- [ ] ~20 Vitest unit tests in `src/solver-config.test.ts` pass
- [ ] Playwright browser verification passes all 4 scenarios
- [ ] TypeScript compiles cleanly (tsc --noEmit)

## Implementation Plan

1. `src/solver-config.ts` — pure data module (TDD first)
2. `src/solver-form.ts` — DOM module (follows upload.ts pattern)
3. `index.html` — fill Step 02, add Step 03
4. `src/main.css` — pills, config panel, checklist styles
5. `src/main.ts` — wire up initSolverConfig

## Progress Log

### 2026-06-07
- Created KB task, created branch feat/teeline-web-solver-config

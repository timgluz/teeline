---
id: 019ea7f8-e660-7d02-b7e4-d16c34d43f28
slug: tasks/gh-177-dynamic-solver-list
title: "feat(teeline-web): dynamic solver list from WASM — GH #177"
type: task
status: completed
priority: high
tags: [teeline-web, teeline-wasm, wasm, frontend]
---

## Overview

Replace the hardcoded `solver-config.ts` in teeline-web with dynamic data from the WASM binary. `list_algorithms()` (from PR #178) already exists but only returns id/name/description/recommendation. This task extends it with `kind` and `params`, adds `get-version`, wires both into the frontend, and removes `solver-config.ts`.

GH issue: https://github.com/timgluz/teeline/issues/177

## Goals

- WASM binary is the single source of truth for solver metadata
- Adding a new solver to Rust + redeploying WASM is sufficient — no frontend change needed
- Footer shows WASM readiness (grey → green dot + version string)

## Acceptance Criteria

- [x] `list-algorithms` returns all solvers with `id`, `name`, `kind`, `description`, `recommendation`, and `params`
- [x] `get-version` returns the Cargo package version string
- [x] `solver-config.ts` is deleted — no hardcoded solver list remains in the frontend
- [x] Step 2 dropdown and config panels render from the WASM response
- [x] Footer shows grey → green dot sequence on load; version string appears when WASM is ready
- [x] Footer shows red dot if WASM fails to initialise
- [x] All existing tests pass; worker integration tests cover `list-algorithms` and `get-version`
- [x] `teeline-web.yml` CI updated to regenerate JS bindings before npm ci

## Implementation Phases

1. Pre-flight: regenerate stale JS bindings; update CI
2. WIT: add `param-spec` record (using `value-type` field), extend `algorithm-info`, add `get-version`
3. Rust/WASM: `kind_for`, `params_for_solver`, `get_version` helpers; update `list_algorithms`
4. Worker: `list-algorithms` + `get-version` message types
5. Frontend: `wasm-types.ts`, update `solver-form.ts` + `main.ts`, add footer
6. Delete `solver-config.ts` + `solver-config.test.ts`

## Progress Log

### 2026-06-08
- Branch: `feat/gh-177-dynamic-solver-list`
- Plan reviewed by Opus; key findings incorporated (stale bindings, `value-type` rename, CI gap, worker mock extension, wasmtime test coverage)
- Implemented all phases; 39/39 wasmtime tests pass, 58/58 Vitest tests pass, tsc clean
- Commit: f3ddb41 — feat(wasm+web): dynamic solver list from WASM binary (#177)
- `param-spec.key` uses camelCase to match `keyof SolveOptions` directly (user decision)
- `value-type` WIT field → `valueType` TypeScript (jco camelCase) — avoids `type_` ambiguity
- `teeline-wasm/js-bindings/teeline_wasm.js` committed (tracked despite gitignore; CI will regenerate)

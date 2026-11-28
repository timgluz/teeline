---
name: explainer-workflow
description: Shared workflow and definition of done for Teeline's interactive algorithm explainers. Use when brainstorming, planning, implementing, or reviewing an explainer. Covers the learning objective, storyboard, pinned fixture and invariants, honest statistics, accessibility, reduced motion, registration, tests, and the role/model lifecycle.
allowed-tools: Bash(openspec:*), Bash(npx:*), Bash(task:*), Bash(gh:*)
license: MIT
metadata:
  author: teeline
  version: "1.0"
---

# Explainer Workflow

An explainer is a small interactive lesson, not a second solver and not a blog post. It
pairs a pure-TypeScript simulation (`*-algo.ts`) with a Preact component (`*.tsx`), and it
must teach one algorithm's mechanism through a pinned, legible example.

## Lifecycle

```text
explainer-brainstormer   Gemini   concepts
        ↓ pick one
explainer-planner        Opus     OpenSpec change (proposal, specs, design, tasks)
        ↓
plan PR + Copilot review          plan review; no separate model
        ↓ human approves
explainer-implementor    DeepSeek openspec-apply-change + code
        ↓
explainer-build-reviewer Opus     delivery review
        ↓
merge
```

- **Explore** uses Gemini. **Author the plan** and **review the build** use Opus.
- **Plan review** is GitHub Copilot on the plan PR; do not duplicate it in another model.
- **Implement** uses DeepSeek.

## Phase 1: brainstorm

The brainstormer returns 2–4 concepts. Each must state the one idea the visitor should
leave with, the misconception it corrects, a candidate storyboard, the interaction model
(what the visitor changes and what is pinned), a pinned-fixture idea, the visual language,
and the risks. No generic "what is TSP" concepts and no invented numbers.

## Phase 2: plan (OpenSpec)

The planner runs the `openspec-propose` workflow for `add-<id>-explainer` and applies the
explainer additions below. The change's `proposal.md` must state the learning objective and
the non-goals (no WASM call, no solver change, no second production path).

### `specs/web-explainers/spec.md`

Add only the algorithm-specific requirements. The existing capability already covers the
cross-cutting requirements: discoverability (route, docs CTA, index link), accessibility,
responsive layout, reduced motion, and deterministic simulation. A new explainer adds:

- its storyboard and stage order;
- its fixture and the invariants the fixture must satisfy;
- its interaction model (controls, selections) and the honesty rule for statistics;
- any algorithm-specific teaching requirement.

Each requirement needs a `#### Scenario:` with a WHEN/THEN. Keep the delta testable.

### `design.md` additions

The design must include, beyond the generic sections:

1. **Learning objective.** One sentence: what the visitor should understand after the last
   stage. Name the misconception it corrects.
2. **Storyboard.** The stages, in order, and why that is the shortest path to the aha. State
   what the visitor sees at each stage and what changes between them.
3. **Pinned fixture.** Either the exact coordinates/tours as constants, or a deterministic
   generation rule. List the invariants that make the lesson visible (shared-edge ratio,
   cycle count, diff degree, exact costs). The fixture is pinned, not searched at runtime.
4. **Interaction model.** What the visitor can change (stage, playback, selections,
   strategy) and what is fixed for reproducibility. State which changes rebuild later
   frames.
5. **Visual language.** Edges, colours, dashes, badges, legends, and the statistics panel.
   Explainers use their own scoped palette; follow `docs/web-design.md`.
6. **Honesty rule.** Every length, delta, and count is computed from the fixture; the
   explainer never fabricates an improvement. If a branch is unreachable in the UI, cover it
   in a unit test and say so.
7. **Registration checklist.** `EXPLAINER_META` / `EXPLAINER_IDS`, the static import and
   `id === ...` branch in `pages/algorithms/[id]/explainer/index.astro`, `EXPLAINER_SOLVERS`
   in `nav-data.ts`, `hasExplainer: true` on the algorithm page, the docs CTA, and the
   algorithms-index link.
8. **Non-goals.** No WASM call, no solver/CLI/API/benchmark change, no general playground,
   no arbitrary uploaded instances.

### `tasks.md` groups

Use the same shape as the SFC and EAX explainers:

1. fixture and pure simulation (each task lists its Vitest invariant);
2. storyboard component (render, stages, controls, selections, statistics, a11y);
3. registration and docs;
4. Playwright coverage (route, docs CTA, index link, keyboard, narrow viewport, seeded
   selection, the empty/unreachable branches);
5. integration gate (`npx tsc --noEmit`, `task web:test`, `task web:build`, the new
   Playwright spec, `markdownlint` on the docs page).

Every task states how to verify it. Do not leave testing or documentation to a final
catch-all group.

## Phase 3: plan review (Copilot)

The planner opens a PR for the OpenSpec change. The plan-reviewer confirms
`openspec validate --strict` passes, collects GitHub Copilot's review comments with `gh`,
and hands the actionable ones back to the planner. Copilot is the plan reviewer; there is no
separate plan-review model.

## Phase 4: implement

The implementor follows `openspec-apply-change` and builds the approved plan:

- `*-algo.ts`: pure simulation, pinned fixture, invariants from the design, no imports from
  the solver or WASM.
- `*.tsx`: the storyboard, controls, selections, statistics, screen-reader summary, reduced
  motion, narrow layout.
- registration in every place listed in the design.
- Vitest for the pure module; Playwright for the interaction and accessibility flow.

Do not change the Rust solver, WASM, API, CLI, or benchmarks.

## Phase 5: build review

The build reviewer checks the implementation against the approved spec and this checklist,
and runs the tests. Report a verdict and prioritized findings; do not edit files.

## Definition of done

- [ ] The learning objective is stated and visible by the final stage.
- [ ] The storyboard is ordered and each stage earns its place.
- [ ] The fixture is pinned, legible, and its invariants are asserted in Vitest.
- [ ] Every statistic is computed from the fixture; no fabricated numbers.
- [ ] The explainer is a visualization, not a second solver.
- [ ] Keyboard, screen-reader summary, reduced motion, and narrow viewport are covered.
- [ ] Registered in `EXPLAINER_META`, `EXPLAINER_IDS`, the explainer route, `nav-data.ts`,
      the algorithm page, and the algorithms index.
- [ ] `npx tsc --noEmit`, `task web:test`, `task web:build`, the Playwright spec, and
      `markdownlint` pass.

## References

- `openspec/specs/web-explainers/spec.md` — the cross-cutting behavioral contract.
- `openspec/changes/archive/2026-11-14-add-sfc-explainer/` and
  `2026-11-22-add-eax-explainer/` — two worked examples.
- `docs/web-design.md` — palette, typography, and anti-patterns.
- The EAX, SFC, BHK, and Christofides explainers under `teeline-web/src/explainers/` are the
  strongest current references for a pinned fixture, a storyboard, and honest statistics.

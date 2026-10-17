---
id: 01a00bed-8225-75d1-8c61-4376d440f4b0
slug: tasks/gh-476-algorithms-index
title: "feat(web): topbar dropdown to algorithms index + complexity column (PR #476)"
type: task
status: completed
priority: medium
tags: [teeline-web, nav, algorithms-index]
---

## Overview

Navigation and index-page work (PR #476, merged as commit `314c82e`): replaced the topbar "Algorithms" `<details>` dropdown with a plain link to a new `/algorithms/` index page, and added a Complexity column to both the index page tables and the docs meta tables — closing a long-standing inconsistency where the NN page showed a different complexity than the sidebar/table.

## Goals

- Topbar: dropdown → `<a href="/algorithms/">Algorithms</a>` (simpler, crawlable, no client JS)
- New `/algorithms/` index page: solver groups (Exact / Constructive / Local search / Metaheuristic) with 2–3 line `groupDesc` blurbs, per-solver tables with a Complexity column, `▶ interactive` links with `aria-label="Open interactive explainer for <name>"`
- Make index-page complexity match the docs meta table on every solver (single source of truth + a test that enforces it)

## Acceptance Criteria

- [x] `Topbar.astro` dropdown removed; nav-data import dropped from topbar
- [x] `pages/algorithms/index.astro` renders groups + tables + Complexity column + aria-labels
- [x] `nav-data.ts` `SOLVER_META[id].complexity` mirrors each `docs/algorithms/*.md` Complexity row (21/21 files), including curated values (nn `O(n log n) with KD-tree`, bhk `O(2ⁿ · n²) time, O(2ⁿ · n) space`, branch_bound worst-case note, fourier/som per-run bounds, etc.)
- [x] `nav-data.test.ts` complexity-consistency test uses `import.meta.glob<string>('../../docs/algorithms/*.md', { query: '?raw', ... })` — node:fs avoided because `@types/node` is absent from this tsconfig
- [x] All `docs/algorithms/*.md` gained a `| **Complexity** | … |` meta row
- [x] Playwright e2e (`tests/algorithms-index.spec.ts`); astro check + tsc + vitest + Playwright green

## Related PRs (same session)

- **#471** (commit `2d79a37`) — refactor(web): extract shared `explainer-cities.ts` (CITIES_10/12/8, `makeDist`, `makeTourLength`, `makeDm`, prebound `dist/tourLength` per city count, defaults `CITIES/N_CITIES/dist/tourLength`) — an `ocr` finding that later explainers all build on
- **#473** — menubar PR, **closed as empty diff** (content absorbed into the #475 squash merge); the stacked-branch squash absorption is documented in [[notes/agent-dev-notes]]

## Completion Evidence

- Commit: `314c82e` — feat(web): replace topbar algorithms drop-down with a link to an index page (#476)
- PR: [#476](https://github.com/timgluz/teeline/pull/476) (merged)

## References

- [[notes/agent-dev-notes]] — git workflow gotchas hit during this session

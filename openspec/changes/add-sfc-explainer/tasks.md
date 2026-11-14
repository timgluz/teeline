# Tasks

## 1. Simulation logic

- [x] 1.1 Add the fixed 24-city illustrative layout pinned in `design.md` (spans `256 × 254`) plus the uniform projection and statistics helpers to `teeline-web/src/explainers/sfc-algo.ts`; verify Vitest covers the larger span filling the grid, the shorter span leaving a margin below 1%, the max-edge clamp, identical points, and collinear points.
- [x] 1.2 Implement Hilbert indexing for `p = 1..8`; verify the `p = 1` and `p = 2` reference grids, the bijection over all cells for `p <= 6`, and Manhattan adjacency of consecutive indices.
- [x] 1.3 Implement Morton indexing with a 32-bit-safe spread (bit loop or 32-bit masks); verify known Morton keys, the bijection for `p <= 6`, and a regression test that fails if the 64-bit Rust masks are copied verbatim into JavaScript.
- [x] 1.4 Implement `curveCells(p, curveType)` returning the ordered cell coordinates; verify the length is `4^p`, the mapping is a bijection for `p <= 6`, consecutive cells are either grid-adjacent or counted as long jumps, and the `p = 8` boundary is correct.
- [x] 1.5 Implement order assembly, closed-tour edges, seam/longest-edge selection (first edge in tour order on ties), and the cells/bucket/curve-jump statistics with lengths in layout units; verify the pinned fixture changes order at `p = 2, 3, 4, 5, 6` for both curve types, same-cell cities keep input order, and the statistics are deterministic.

## 2. Explainer component

- [x] 2.1 Create `teeline-web/src/explainers/sfc.tsx` with the existing explainer shell (header, eyebrow, subtitle, legend, stats, order list, controls) and a square canvas/SVG viewport; verify `task web:lint` passes and the page renders at the default `p = 1`.
- [x] 2.2 Render the Hilbert and Morton curves on the background canvas for `p = 1..8`, draw non-adjacent curve steps in a distinct hue with longer jumps more opaque, and draw cities, order badges, the closed tour, and the seam/longest-edge highlights in the SVG overlay; verify visually at `p = 1, 2, 4, 8` for both curve types and that the tour stays legible at `p = 8`.
- [x] 2.3 Add the live grid-order slider, Hilbert/Morton toggle, Play subdivision and Reset controls; start at `p = 1`, run Play from 1 to 8, return Reset to 1, and stop playback on slider/toggle/Reset; add `prefers-reduced-motion` behavior (Play jumps to 8), the live summary, and `aria-valuetext`; verify keyboard operation and that Play stops at `p = 8`.
- [x] 2.4 Add the seam/longest-edge callout and the concise 2-opt explanation, plus the statistics that expose cells used, largest bucket, tour length, seam length, longest edge length, and long curve jumps, with lengths in layout units; verify the callout references the highlighted edges and the stats update with `p` and curve type.
- [x] 2.5 Add the responsive layout and accessibility affordances: stacked narrow-viewport layout, visible focus states, a screen-reader summary (curve, grid order, city order, tour length, seam, longest edge), and an ordered city list; verify keyboard traversal and a narrow viewport without horizontal scrolling.
- [x] 2.6 Add `teeline-web/tests/sfc.spec.ts` covering hydration, slider updates, Hilbert/Morton switching, Play/Reset, reduced-motion behavior, seam and longest-edge presence, and keyboard operation; verify the targeted Playwright spec passes.

## 3. Registration and docs

- [x] 3.1 Add the `space_filling_curve` entry with the title, description, and backLabel pinned in `design.md` to `teeline-web/src/explainers/registry.ts`, update the stale "15" comment to 22, and add the static import plus `id === 'space_filling_curve'` branch to `teeline-web/src/pages/algorithms/[id]/explainer/index.astro`; verify the explainer route renders and appears in the explainer sidebar.
- [x] 3.2 Add `hasExplainer: true` to `docs/algorithms/space-filling-curve.md`; verify the docs page shows the "Open interactive explainer" link.
- [x] 3.3 Correct the stale `EXPLAINER_SOLVERS` set in `teeline-web/src/nav-data.ts` to list every explainer ID and change `nav-data.test.ts` to assert it equals `EXPLAINER_IDS`; verify `task web:test` passes.
- [x] 3.4 Update `teeline-web/tests/algorithms-index.spec.ts` from 21 to 22 interactive links and add assertions for the SFC explainer link from the index and the docs-page CTA; verify the targeted Playwright specs pass.

## 4. Integration gate

- [x] 4.1 Run `task web:test`, `task web:lint`, and `task web:build`; verify every command exits 0.
- [x] 4.2 Run the targeted Playwright specs (including the reduced-motion check) and the existing explainer smoke specs; verify they pass with no new flakes.
- [x] 4.3 Run the repository pre-commit checks on the changed files and `graphify update .`; verify the hook is green and the graph is current.

## Workflow follow-up

- Branch off `master` as `feat/sfc-explainer`, commit with a Conventional Commit scoped to `web`, and open the PR with `gh pr create`.
- Archive the change after review and a green integration gate.

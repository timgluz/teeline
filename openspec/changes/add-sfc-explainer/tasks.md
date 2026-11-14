# Tasks

## 1. Simulation logic

- [ ] 1.1 Add the fixed 24-city illustrative layout plus the uniform projection and statistics helpers to `teeline-web/src/explainers/sfc-algo.ts`; verify Vitest covers a square bounding box, the max-edge clamp, identical points, and collinear points.
- [ ] 1.2 Implement Hilbert indexing for `p = 1..8`; verify the `p = 1` and `p = 2` reference grids, the bijection over all cells for `p <= 6`, and Manhattan adjacency of consecutive indices.
- [ ] 1.3 Implement Morton indexing with a 32-bit-safe spread (bit loop or 32-bit masks); verify known Morton keys, the bijection for `p <= 6`, and a regression test that fails if the 64-bit Rust masks are copied verbatim into JavaScript.
- [ ] 1.4 Implement order assembly, closed-tour edges, seam/longest-edge selection, and the cells/bucket/curve-jump statistics; verify the order changes across the expected `p` values, same-cell cities keep input order, and the seam/longest-edge selection is deterministic.

## 2. Explainer component

- [ ] 2.1 Create `teeline-web/src/explainers/sfc.tsx` with the existing explainer shell (header, eyebrow, subtitle, legend, stats, order list, controls) and a square canvas/SVG viewport; verify `task web:lint` passes and the page renders at the default `p`.
- [ ] 2.2 Render the Hilbert and Morton curves on the background canvas for `p = 1..8`, distinguish long curve jumps for Morton, and draw cities, order badges, the closed tour, and the seam/longest-edge highlights in the SVG overlay; verify visually at `p = 1, 2, 4, 8` for both curve types.
- [ ] 2.3 Add the live grid-order slider, Hilbert/Morton toggle, Play subdivision and Reset controls, `prefers-reduced-motion` behavior, and the live summary/`aria-valuetext`; verify keyboard operation and that Play stops at `p = 8`.
- [ ] 2.4 Add the seam/longest-edge callout and the concise 2-opt explanation, plus the statistics that expose cells used, largest bucket, tour length, seam length, and long curve jumps; verify the callout references the highlighted edges and the stats update with `p` and curve type.
- [ ] 2.5 Add the responsive layout and accessibility affordances: stacked narrow-viewport layout, visible focus states, a screen-reader summary for the visualization, and an ordered city list; verify keyboard traversal and a narrow viewport without horizontal scrolling.
- [ ] 2.6 Add `teeline-web/tests/sfc.spec.ts` covering hydration, slider updates, Hilbert/Morton switching, Play/Reset, seam and longest-edge presence, and keyboard operation; verify the targeted Playwright spec passes.

## 3. Registration and docs

- [ ] 3.1 Add the `space_filling_curve` entry to `teeline-web/src/explainers/registry.ts` and the static import plus `id === 'space_filling_curve'` branch to `teeline-web/src/pages/algorithms/[id]/explainer/index.astro`; verify the explainer route renders and appears in the explainer sidebar.
- [ ] 3.2 Add `hasExplainer: true` to `docs/algorithms/space-filling-curve.md`; verify the docs page shows the "Open interactive explainer" link.
- [ ] 3.3 Correct the stale `EXPLAINER_SOLVERS` set in `teeline-web/src/nav-data.ts` to list every explainer ID and update `nav-data.test.ts`; verify `task web:test` passes.
- [ ] 3.4 Update `teeline-web/tests/algorithms-index.spec.ts` from 21 to 22 interactive links and add assertions for the SFC explainer link from the index and docs page; verify the targeted Playwright specs pass.

## 4. Integration gate

- [ ] 4.1 Run `task web:test`, `task web:lint`, and `task web:build`; verify every command exits 0.
- [ ] 4.2 Run the targeted Playwright specs and the existing explainer smoke specs; verify they pass with no new flakes.
- [ ] 4.3 Run the repository pre-commit checks on the changed files and `graphify update .`; verify the hook is green and the graph is current.

## Workflow follow-up

- Branch off `master` as `feat/sfc-explainer`, commit with a Conventional Commit scoped to `web`, and open the PR with `gh pr create`.
- Archive the change after review and a green integration gate.

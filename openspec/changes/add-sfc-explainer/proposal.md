# Proposal

## Why

Issue #644 asks for an interactive explainer for the Space-Filling Curve solver added in #643. The solver's output depends on a geometric idea that is hard to see from prose: cities are projected onto a `2^p × 2^p` grid, ordered by a Hilbert or Morton index, and the resulting tour inherits the curve's locality plus one long closing seam. Now that the solver is merged, the web explainers are the right place to make that mechanism visible.

## What Changes

- Add a browser-only, interactive Space-Filling Curve explainer at `/algorithms/space_filling_curve/explainer/`.
- Provide a live grid-order slider for `p = 1..8` and a Hilbert/Morton toggle. Changing either redraws the curve and the city ordering.
- Draw a faint curve on a `2^p × 2^p` grid, the city order along it, and the resulting tour. Use a background canvas for the curve (65,536 segments at `p = 8`) with an SVG overlay for cities, tour edges, labels, and the seam.
- Highlight the closing seam edge, surface the longest tour edge separately when it differs, and explain concisely why SFC tours are usually paired with a 2-opt pass.
- Add teaching stats that make the curve's behavior legible: cells used, largest bucket, tour length, seam length, and long curve jumps (zero for Hilbert, many for Morton).
- Add a "Play subdivision" control that steps `p` from 1 through 8, while the slider remains directly scrubbable. Respect `prefers-reduced-motion`.
- Implement only the subset of solver behavior the demonstration needs: uniform normalization, Hilbert and Morton indexing for `p <= 8`, and input-order tie-breaking. This is a pedagogical simulation, not a one-to-one port of `src/tsp/space_filling_curve.rs`.
- Register the explainer in the web explainer registry, the Astro explainer route, and the Space-Filling Curve docs frontmatter so the existing CTA and sidebars pick it up.
- Add unit tests for the simulation logic and a Playwright smoke test for the page.

No solver, CLI, API, or WASM behavior changes. No breaking changes.

## Capabilities

### New Capabilities

- `web-explainers`: Interactive, browser-only visual explanations of Teeline's solvers, including their geometry, state transitions, and the trade-offs behind their results.

### Modified Capabilities

<!-- None: this adds a new capability; no existing capability requirements change. -->

## Impact

- Web explainers: `teeline-web/src/explainers/sfc-algo.ts`, `sfc-algo.test.ts`, `sfc.tsx` (new)
- Web registration: `teeline-web/src/explainers/registry.ts`, `teeline-web/src/pages/algorithms/[id]/explainer/index.astro`, `teeline-web/src/nav-data.ts` and its test if the explainer set is corrected
- Docs: `docs/algorithms/space-filling-curve.md` (`hasExplainer: true`)
- Web tests: new `teeline-web/tests/sfc.spec.ts`; update `teeline-web/tests/algorithms-index.spec.ts` from 21 to 22 interactive links
- No changes to `src/`, `teeline-cli`, `teeline-api`, `teeline-wasm`, or solver behavior

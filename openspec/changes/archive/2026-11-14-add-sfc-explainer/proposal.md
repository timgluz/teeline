# Proposal

## Why

Issue #644 asked for an interactive Space-Filling Curve explainer. The first version shipped a final-state viewer: it loads with the complete tour, the curve is a background wash, and the order panel is a bare list of `visit · city` pairs. At `p = 8` the curve is a 256×256 texture, every city is in its own cell, and nothing on screen explains why the order is what it is or how the curve produced the tour.

The missing explanation is the construction: **project → index → sort → connect**. The curve is a 1D sort key, not the tour path. This change turns the explainer into a staged construction with a 1D index strip, a sort-key table, and an explicit "why the order changed" diff.

## What Changes

- Present the construction in four stages: **Project** (cities → cells), **Index** (cells → curve index), **Sort** (index → visit order), and **Connect** (visit order → tour edges + seam). Start at Project with no completed tour; default to `p = 3`.
- Add a **1D number-line strip** below the canvas: each city is a tick at its curve index; same-cell cities stack; hover is synchronized with the canvas and table; a scrubbable playhead traces the curve and tour.
- Replace the flat order list with a **compact sort table**: visit, city, cell, curve index; shared cells are grouped and the input-order tie is labelled.
- Add an **occupied-cell overlay** with bucket counts; outline shared cells and highlight changed cells/cities when `p` or curve type changes. Show a "why the order changed" callout.
- Add construction playback with **Back, Step, Run/Pause, Reset**, and stage-jump buttons. Add **Pause and discrete `p−`/`p+` stepping** for the subdivision. Keep the `p` slider and Hilbert/Morton toggle live.
- Reduce visual noise: keep the full curve crisp for `p <= 4`; fade it and draw the occupied-cell **spine** for `p > 4`; default to `p = 3`.
- Improve the Morton lesson: highlight **quadrant-crossing jumps** with a dashed callout.
- Clarify labels and colours: visit order is the primary node label, city index on hover/table; **indigo tour edges**; encode seam/longest overlap with a dashed/striped stroke.
- Add an interactive **2-opt preview**: ghost the seam and its best partner edge, draw the two replacement edges, and show the actual tour-length delta for the fixture.
- Keep the existing seam, statistics, registration, and accessibility work; update the registry description and tests.

No solver, CLI, API, or WASM behavior changes. No breaking changes.

## Capabilities

### New Capabilities

- `web-explainers`: Interactive, browser-only visual explanations of Teeline's solvers, including their construction steps, geometry, and state transitions.

### Modified Capabilities

<!-- None: this adds a new capability; no existing capability requirements change. -->

## Impact

- Web explainers: `teeline-web/src/explainers/sfc-algo.ts`, `sfc-algo.test.ts`, `sfc.tsx`
- Web registration: `teeline-web/src/explainers/registry.ts` (description), Astro route; the `EXPLAINER_SOLVERS` set is already correct
- Docs: `docs/algorithms/space-filling-curve.md` (`hasExplainer` already set)
- Web tests: `teeline-web/tests/sfc.spec.ts` rewritten; the algorithms-index count stays 22
- No changes to `src/`, `teeline-cli`, `teeline-api`, `teeline-wasm`, or solver behavior

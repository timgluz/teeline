# Design

## Context

See `proposal.md` — Why. Relevant current state:

- The solver already exists in `src/tsp/space_filling_curve.rs` and is wired through the core, CLI, API, WASM, and web options. This change does not touch it.
- All existing explainers are self-contained Preact components with a pure-TypeScript `*-algo.ts` simulation module. None import `teeline-wasm`.
- The Astro route requires a static import and a literal `id === ...` branch; a dynamic component lookup fails at render time (`NoMatchingImport`).
- The docs page CTA is driven by `hasExplainer: true` in the Markdown frontmatter; the algorithms index and explainer sidebar are driven by `EXPLAINER_META`.
- `EXPLAINER_SOLVERS` in `nav-data.ts` is stale: it names 13 explainers while the registry has 21. It is not used by the pages, only by its own test.
- The explainer pages use the existing light docs-page explainer palette. The dark `web-design.md` tokens apply to page shells, not these embedded islands.

## Goals / Non-Goals

**Goals:**

- A self-contained, deterministic explainer that makes the SFC projection, ordering, curve-type contrast, and seam visible for `p = 1..8`.
- A small simulation module that is easy to audit against the Rust source and cheap enough to run on every slider input.
- Registration that reuses the existing explainer shells, sidebars, CTA, and test patterns.
- Accessibility and responsive behavior consistent with the existing explainers.

**Non-Goals:**

- A one-to-one port of `SfcOptions`. The explainer does not implement rotation, mirroring, multi-orientation, aspect scaling, or the `lexicographic`/`nearest_prev` tie-breakers.
- Calling the WASM solver or adding a WASM debug export.
- Replacing or improving the solver, changing CLI/API behavior, or benchmarking the explainer.
- A general-purpose SFC playground for arbitrary uploaded instances.

## Decisions

### 1. Pure-TypeScript pedagogical simulation, not a full solver port

`sfc-algo.ts` implements only what the demonstration needs:

- uniform bounding-box normalization with the solver's `EPSILON` zero-span guard and round-and-clamp to `0..2^p - 1`,
- Hilbert and Morton indices for `p = 1..8`,
- sorting by `(curve index, input index)`.

The module intentionally omits the solver's rotation, mirror, aspect mode, multi-orientation, and alternate tie-breakers. The UI explains that the real solver can try eight orientations; the visualization fixes orientation at 0 degrees and no mirror so the curve does not flip while the visitor scrubs `p`.

This is a visualization, not a second solver path: it never produces solver output, and the production web solve path remains `Solvers -> tsp::solve` through the WASM bridge. The constitution's "solver logic lives in the root crate" constraint is preserved because this module only draws a teaching model of the construction.

Alternatives considered:

- **Call the WASM solver for the tour order.** Rejected: it would add async/worker plumbing, still leave the curve geometry to TypeScript, and only cover the order, not the 65k-cell curve.
- **Port every `SfcOptions` field.** Rejected: the extra controls would bury the core lesson, and the extra code would be harder to keep aligned with Rust than the small subset.
- **Add a WASM debug export returning grid coordinates and keys.** Rejected: it expands the public WASM surface and the ambient `.d.ts` for one page.

The module cites the Rust functions it mirrors and uses the same reference vectors in tests (Hilbert `p=1`, Hilbert `p=2`, bijection and Manhattan adjacency for `p <= 6`, max-edge clamping, identical/collinear points).

### 2. Canvas for the curve, SVG for everything interactive

At `p = 8` the curve has `4^8 = 65,536` segments. Per-segment SVG elements are not viable, and a single SVG path is roughly half a megabyte of `d` data and can jank while the slider moves.

The visualization uses:

- a background `<canvas>` that draws the full curve once per `p`/curve-type change,
- an SVG overlay with the same `0 0 300 300` coordinate system for city markers, order badges, tour edges, seam/longest-edge highlights, and labels.

`sfc-algo.ts` exposes `curveCells(p, curveType)` returning the ordered cell coordinates. Unit tests pin its length (`4^p`), bijection, adjacency, and jump count, and the canvas consumes the same function. This keeps the drawing path testable instead of reducing it to "a canvas exists".

The canvas is `aria-hidden="true"`; the SVG carries a concise `role="img"` label, and a visible order list plus statistics provide the text equivalent. The canvas backing store is sized for the device pixel ratio, and redraws are coalesced with `requestAnimationFrame`. Faint grid lines are drawn for `p <= 5`; above that the curve itself shows the subdivision.

Non-adjacent curve steps are drawn in a second hue so Morton's quadrant jumps are visible against Hilbert's continuous path. The tour is drawn above the curve with a light contrast halo so it stays legible at `p = 8`.

Alternatives considered:

- **Single SVG path.** Simpler DOM, but the `p=8` path is large and rebuilt on every input event.
- **Canvas for everything.** Good for the curve, but labels, focus styling, and accessible structure are easier to keep in SVG.
- **Cap the drawn curve at `p = 6`.** Rejected: it would not show the selected curve at `p = 7..8`.

### 3. Fixed 24-city illustrative layout

The explainer uses a fixed 24-city layout with integer coordinates, arranged as four multi-scale corner clusters. This is a deliberate teaching fixture, not a benchmark instance.

The pinned layout is:

```text
[22,208] [46,232] [70,256] [94,280] [30,270] [88,214]
[206,208] [230,232] [254,256] [278,280] [214,270] [272,214]
[22,26]  [46,50]  [70,74]  [94,98]  [30,88]  [88,32]
[206,26] [230,50] [254,74] [278,98] [214,88] [272,32]
```

The spans are `256 × 254`. This near-square shape is intentional: an exact `256 × 256` grid-aligned layout stabilizes the city order at `p = 4`, while the 2-unit shorter y span keeps both Hilbert and Morton changing order at `p = 2, 3, 4, 5, 6`. The shorter axis leaves a margin below 1% under uniform scaling, which is visually imperceptible.

Why 24 cities:

- the existing 12-city layouts put most cities in distinct cells by `p = 2`, so the Hilbert order barely changes for the rest of the slider range;
- 24 cities change order at several orders (`p = 2..6`) while still fitting readable markers and two-digit order badges;
- four clusters create meaningful same-cell buckets at low `p` and separate cleanly at higher `p`.

At `p = 1` the layout uses 4 cells with a largest bucket of 6; at `p = 2` it uses 16 cells with a largest bucket of 2; by `p = 4` all 24 cities have their own cell.

City positions use the same uniform normalization as the solver (scale both axes by the larger span, anchor at the minimum, `y` up). They are not snapped to cell centers, so the tour stays visually readable when several cities share a cell; the bucket statistics expose the projection instead.

Alternatives considered:

- **Reuse `CITIES_12`.** Rejected: the order stabilizes too early.
- **Use a real TSPLIB subset.** Rejected: it introduces an asymmetric bounding box and invites benchmark comparisons the explainer does not make. The docs page remains the place for real-instance results.

### 4. Highlight the seam and the longest edge separately

The tour is the closed sequence produced by the curve order. The edge from the last city back to the first is always the seam and is drawn in amber. The longest edge is computed separately; when it is not the seam, it is drawn in a contrasting colour and labelled. If two edges share the maximum length, the first edge in tour order wins, keeping the display deterministic.

Tour length, seam length, and longest-edge length are computed on the fixture coordinates, not canvas pixels, and the UI labels them as layout units. This keeps the statistics stable when the viewport changes.

A long curve jump is a step between consecutive curve cells that are not grid neighbours (Manhattan distance greater than 1). The statistic counts all of them, and the canvas draws all non-adjacent steps in a distinct hue; longer jumps get more opacity so the quadrant structure remains readable at high `p`. At `p = 8` that is 32,767 for Morton and 0 for Hilbert.

A short callout explains that a 2-opt pass removes the seam plus another edge and reconnects the four endpoints, which is why SFC tours are usually fed to `2opt` or `lk`.

Alternatives considered:

- **Highlight only the seam.** Simple, but hides the Morton quadrant jumps that can be worse than the closing edge.
- **Highlight every above-average edge.** Too noisy to read.
- **Count only quadrant-scale jumps.** Rejected: the spec defines a long jump as any non-adjacent step, and a separate threshold would make the statistic and the drawing disagree.

### 5. Subdivision is a stateful step, not decorative motion

The explainer starts at `p = 1`. A "Play subdivision" control advances `p` to 8 on a timer; Reset returns to 1. Moving the slider, switching the curve type, or pressing Reset stops playback. There is no Pause button: the run is short, the slider is direct, and reduced-motion users skip it. A Pause toggle can be added later if parity with the other explainers matters.

With `prefers-reduced-motion: reduce`, Play jumps straight to `p = 8` instead of animating through intermediate orders.

Alternatives considered:

- **CSS draw-on animation of the curve.** Decorative, hard to time sensibly for 65k segments, and at odds with the project's "motion is functional only" rule.
- **No animation.** The slider alone covers the acceptance criteria, but the issue explicitly asks for an animated subdivision.

### 6. Registration follows the existing explainer pattern

Add the metadata to `EXPLAINER_META`, a static import and `id === 'space_filling_curve'` branch to the Astro route, and `hasExplainer: true` to the docs frontmatter. Update the algorithms-index E2E count from 21 to 22.

The registry entry pins:

- `title`: `Space-Filling Curve — Interactive Explainer — Teeline`
- `description`: `Interactive walkthrough of the Space-Filling Curve construction: watch Hilbert and Morton curves order cities on a 2^p × 2^p grid, and see the seam edge that a 2-opt pass removes.`
- `backLabel`: `SFC`

The explainer uses the existing docs-page explainer light palette (the same approach as Greedy Edge and Savings); the dark `web-design.md` tokens apply to page shells, not these embedded explainers.

Also correct `EXPLAINER_SOLVERS` in `nav-data.ts` to list every explainer ID and change `nav-data.test.ts` to assert the set equals `EXPLAINER_IDS`, so adding this explainer does not leave another stale registry behind and future additions cannot silently drift.

### 7. Tests pin the math and the interaction, not the pixels

- Vitest (node) tests cover the simulation: Hilbert reference grids, bijection and adjacency, Morton keys, normalization edge cases, `curveCells` length/bijection/jumps, order changes at the pinned `p` values, same-cell input order, seam/longest-edge selection and tie-breaking, and statistics in layout units.
- Playwright smoke tests cover hydration, the slider, the Hilbert/Morton toggle, Play/Reset, reduced-motion behavior, seam/longest-edge presence, keyboard operation, and the docs/index links.
- No screenshot tests: the drawing is intentionally display-resolution dependent.

## Risks / Trade-offs

- **JavaScript bitwise operations are 32-bit; the Rust Morton spread is 64-bit.** → Implement the `p <= 8` spread with a simple bit loop (or 32-bit masks) and test Morton keys against known values. The explainer deliberately does not support `p > 8`.
- **The simulation drifts from the Rust solver.** → Keep the ported subset small, cite the source functions, pin the same reference vectors, and state in the UI that the explainer uses a fixed orientation and input-order tie-breaking.
- **`p = 8` redraw cost.** → Draw the curve on a canvas with a single path, coalesce redraws with `requestAnimationFrame`, and avoid re-rendering 65k SVG nodes.
- **The 24-city fixture could still produce long stable stretches.** → The pinned near-square layout changes both curves at `p = 2..6`; add a unit test asserting those transitions so a future fixture edit cannot silently remove them.
- **Morton's 32,767 non-adjacent steps at `p = 8` could swamp the canvas.** → Draw all non-adjacent steps in a low-alpha second hue, give the longer jumps more opacity, and keep the tour above them with a contrast halo. If the result is still unreadable, reduce the jump opacity, not the statistic.
- **A dense visual is hard to read for screen-reader and low-vision users.** → Provide the order list, statistics, `aria-valuetext` on the slider, and a concise SVG summary; keep the canvas decorative.
- **Stale registry/test counts.** → Update the algorithms-index count and assert `EXPLAINER_SOLVERS` equals `EXPLAINER_IDS`; the Vitest test becomes the guard.
- **The docs page CTA silently stays hidden.** → Add `hasExplainer: true` and cover the link in the E2E test.

## Migration Plan

Additive only. No data, API, or solver migration. Rollback is deleting the new explainer files and reverting the registry, route, docs-frontmatter, and test edits.

## Open Questions

None. The scope is deliberately limited to a fixed-orientation, input-order, `p <= 8` teaching simulation.

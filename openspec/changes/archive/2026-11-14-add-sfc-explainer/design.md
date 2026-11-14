# Design

## Context

See `proposal.md` — Why. The first implementation exposed the final state: the tour was complete on load, the order panel was a flat `visit · city` list, and the curve was a dense wash at high `p`. User review found three failures: the construction was invisible, the order changes looked arbitrary, and `p = 8` reduced the explainer to a permutation viewer.

Relevant current state:

- The solver already exists in `src/tsp/space_filling_curve.rs`; this change does not touch it.
- All explainers are self-contained Preact components with a pure-TypeScript `*-algo.ts` simulation module. None import `teeline-wasm`.
- The Astro route needs a static import and a literal `id === ...` branch.
- The docs CTA is driven by `hasExplainer: true`; the index/sidebar are driven by `EXPLAINER_META`.
- The first implementation is on `feat/sfc-explainer` / PR #660. This revision is a follow-up delta, not a new capability.

## Goals / Non-Goals

**Goals:**

- Make the construction — not the final tour — the primary narrative.
- Show explicitly that the curve is a 1D sort key and the tour connects the sorted cities.
- Make order changes explainable through cell buckets, the 1D index strip, and a changed-city diff.
- Keep the visualization legible at every `p`, especially `p = 8`.
- Provide reversible, inspectable playback (Back / Step / Run-Pause / Reset) for construction and subdivision.
- Make the seam/2-opt lesson tangible with a real, computed preview.

**Non-Goals:**

- A one-to-one port of `SfcOptions` (rotation, mirroring, multi-orientation, aspect scaling, alternate tie-breakers).
- Calling the WASM solver or adding a WASM debug export.
- A general-purpose SFC playground for arbitrary uploaded instances.
- A full 2-opt pass or optimizer; the preview shows one best seam-removing move.

## Decisions

### 1. Pure-TypeScript pedagogical simulation

`sfc-algo.ts` keeps the small solver subset: uniform projection, Hilbert/Morton indices for `p <= 8`, input-order sorting. It adds only the data the new lesson needs:

- `rows`: city, cell, index, visit for each city in visit order,
- `buckets`: cities grouped by cell, in curve-index order, with input order inside a bucket,
- `orderDiff(previous, next)`: cities whose visit position changed,
- `twoOptSeamPreview(cities, order)`: the best 2-opt move that removes the closing seam.

This is a visualization, not a second solver path: it never produces solver output, and the production web solve path remains `Solvers -> tsp::solve` through the WASM bridge.

### 2. Canvas for the dense curve, SVG for everything else

The full curve is canvas-rendered (65,536 segments at `p = 8`). Cities, occupied cells, tour edges, the 1D strip, and labels are SVG. The canvas is decorative (`aria-hidden`); the table and strip provide the accessible data.

Rendering policy:

- `p <= 4`: full curve crisp (teal), grid lines faint, occupied cells tinted.
- `p > 4`: fade the full curve exponentially and draw the **occupied-cell spine** — the polyline through the 24 occupied cells in visit order. The spine is the legible curve signal at high `p`.
- The longest Morton quadrant-crossing jumps are drawn dashed red even when the rest of the curve is faded.
- The tour is drawn above the curve with a contrast halo.

### 3. Fixed 24-city illustrative layout

Keep the pinned `256 × 254` four-cluster fixture. `p = 3` (8×8) gives 20 occupied cells and four shared buckets of two, so the curve is traceable and the tie rule is visible on load. `p = 1..8` remains available.

### 4. Staged construction is the primary narrative

The explainer is a four-stage state machine:

| Stage | Canvas | Table / strip | Copy |
| --- | --- | --- | --- |
| Project | cities + occupied cells + bucket counts; no curve/tour | city + cell; grouped by cell | “Each city falls into one cell…” |
| Index | curve + occupied-cell indices | add index column; strip appears | “The curve gives every cell a 1D index…” |
| Sort | city visit-order badges; no tour | sorted by index; strip ticks labelled by visit | “Sort by index; same-cell ties use input order.” |
| Connect | tour edges in visit order; seam last | current row/edge; strip playhead | “The tour connects the sorted cities; the curve is the sort key.” |

The initial state is Project at `p = 3`. The visitor can jump to a stage, Step/Back, or Run through the sequence. Clicking Connect jumps to the completed tour; Step/Run builds it edge by edge.

### 5. 1D strip, sort table, and projection overlay

- **1D strip** below the canvas: x-axis is the curve index `[0, 4^p − 1]`; each city is a tick at its index; same-index ticks stack; visit labels appear at Sort/Connect. A playhead follows the current Connect edge. The seam is the wrap from the rightmost tick to the leftmost.
- **Sort table** replaces the flat order list: columns visit, city, cell, index. Shared cells get a group header labelled “same cell · input order”. The table is a real `<table>` with a caption and scoped headers.
- **Bidirectional hover**: a shared `hoveredCity` state links the canvas city, strip tick, and table row.
- **Occupied-cell overlay**: distinct cells are tinted; shared cells are outlined with a count badge; cell index labels appear from Index onward. For `p > 5` the cells are sub-pixel, so the table/strip carry the mapping and only the hovered cell is outlined.

### 6. Order changes are explained

On `p` or curve-type change, keep the previous layout, compute `orderDiff`, and:

- highlight changed cities on the canvas and in the table,
- add a “N cities changed order since p = X” callout,
- keep the shared-cell groups visible so the visitor sees that finer grids split cells and same-cell cities retain input order.

No fake explanation: the diff is derived from the actual visit positions.

### 7. Reversible playback

Construction and subdivision are separate playback modes; starting one pauses the other.

- **Construction**: Back, Step, Run/Pause, Reset, plus stage buttons. Step advances one stage or one tour edge; Back reverses. Run uses ~700 ms per stage and ~180 ms per edge. Reset returns to Project, `p = 3`, Hilbert.
- **Subdivision**: Play/Pause plus `p−`/`p+` discrete stepping; the slider remains live.
- Changing `p`, curve type, or stage stops playback.
- `prefers-reduced-motion`: construction Run jumps to the completed Connect stage; subdivision Play jumps to `p = 8`.

### 8. Labels, colours, and the seam/longest overlap

- Primary node badge = visit order at Sort/Connect; city index (`#13`) at Project/Index and in the table/hover.
- Tour edges: indigo (`#4f46e5`); curve: teal; seam: amber; longest: magenta; long jumps: orange; Morton quadrant crossings: dashed red.
- When the seam is also the longest edge, draw a dashed magenta overlay on the amber seam and say “seam (also longest edge)”.
- The seam callout uses visit positions (“visit 24 → visit 1”) and highlights both endpoints.

### 9. Interactive 2-opt preview

`twoOptSeamPreview` iterates the valid partner edges (excluding the two degenerate neighbours), computes the new length for removing the seam + partner edge and adding the two reconnections, and returns the best move:

- removed edges: the seam and the partner,
- added edges: `(last, order[j])` and `(first, order[j+1])`,
- old/new lengths and delta.

The Connect callout gets a `[Preview 2-opt fix]` toggle. When on, the removed edges are ghosted, the two replacement edges are drawn dashed green, and the callout shows `old → new (delta%)`, computed from fixture coordinates. If no move improves, show the least-worse delta and say so; never invent numbers.

### 10. Registration and docs

Keep the existing registry route and docs wiring. Update the registry description to mention the staged construction and 1D order. The algorithms-index count stays 22.

### 11. Tests

- Vitest: existing curve/normalization tests, plus `rows`/`buckets` grouping at `p = 1, 3`, input-order ties, `orderDiff` on a known change, `twoOptSeamPreview` (removed/added edges, exact delta, best move), and determinism.
- Playwright: initial Project/no tour at `p = 3`; stage stepping and Back; Run/Pause/Reset; `p−`/`p+`; strip/table counts and headers; shared-cell group; changed-city diff after `p` change; 2-opt preview edges and real delta; Morton quadrant crossing; keyboard; narrow viewport; docs/index links.

## Risks / Trade-offs

- **More UI state → more interactions.** → Keep one `stage` enum, one `connectStep`, one playback mode; unit-test the pure data transitions; stop playback on every manual control change.
- **The table and strip can duplicate information.** → The strip carries position/intuition; the table carries exact cell/index/visit. Shared hover binds them.
- **`p = 8` cells are sub-pixel.** → Fade the curve, draw the occupied-cell spine, and rely on the table/strip for exact cells; outline only the hovered cell.
- **Live-region noise during playback.** → Announce stage changes and changed-city counts, not every edge tick; the table is the stable detailed view.
- **The 2-opt preview could imply a full pass.** → Label it “one best seam-removing move” and show the exact delta.
- **PR #660 is open.** → Push the revision as a follow-up commit to `feat/sfc-explainer` while the PR is open; re-run the full gate.

## Migration Plan

Additive only. No data/API/solver migration. Rollback is reverting the explainer component and tests to the first version.

## Open Questions

None. Decisions locked: default `p = 3`; strip + compact table; indigo tour edges; best seam-removing 2-opt preview; Back/Step/Run-Pause/Reset.

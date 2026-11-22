# Design

## Context

See `proposal.md` — Why, and `specs/web-explainers/spec.md` for the behavior contract. The
current explainer architecture:

- Every explainer is a self-contained Preact component plus a pure-TypeScript `*-algo.ts`
  simulation module; none import `teeline-wasm`. The production solve path is untouched.
- `teeline-web/src/explainers/registry.ts` holds `EXPLAINER_META`; the Astro route needs a
  static import and a literal `id === ...` branch. `nav-data.ts` holds `EXPLAINER_SOLVERS`,
  and the docs page opts in with `hasExplainer: true`.
- Tests are Vitest for the pure module and Playwright for interaction/accessibility.
- The web design rules live in `docs/web-design.md`; explainers have their own scoped palette
  (the GA explainer already uses blue for parent A, amber for parent B, green for the best
  tour). Numeric values use the monospace stack.

## Goals / Non-Goals

**Goals:**

- Make the *why* of EAX visible: shared edges survive, only the symmetric difference changes,
  subtours are patched, and local search polishes the child.
- One crossover event, scrubbable in both directions through six legible stages.
- A pinned 16-city fixture: two crossing-free parent tours sharing 68.75% of their edges, whose
  symmetric difference is exactly two even AB-cycles, with one cycle that splits into subtours
  and one that does not. Strict local-optimality of the parents is a preference, not a hard
  invariant (see Decision 1).
- Honest statistics: every length, delta, and inheritance count comes from the fixture.

**Non-Goals:**

- A one-to-one port of the Rust EAX (candidate lists, family replacement, full population
  loop, epochs, seeding).
- Calling the WASM solver or adding a debug export.
- Arbitrary uploaded instances, or a general EAX playground.
- A full 2-opt pass; the preview is one best move.
- Changing the solver, CLI, API, or benchmark code.

## Decisions

### 1. Pinned 16-city fixture (verified, hand-off ready)

The fixture is **pinned, not searched**. It lives as constants in `eax-algo.ts`; the values
below are the source of truth, so implementation needs no fixture generation or layout tuning.

- **Coordinates** (SVG units; each city index is its position in this list):

  ```text
  0:(179.7, 55.1)   1:(141.3, 97.7)   2:(123.3, 43.5)   3:(85.9, 85.7)
  4:(102.2,140.4)   5:(65.6, 30.0)    6:(30.0, 74.1)    7:(79.2,194.5)
  8:(61.3,248.2)    9:(119.5,234.6)  10:(172.5,214.3)  11:(211.1,255.9)
  12:(248.7,212.8)  13:(270.0,160.2) 14:(223.9,126.8)  15:(171.2,148.3)
  ```

- **Parent A:** `[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]` (cost ≈ 1082.231)
- **Parent B:** `[0,5,6,2,1,4,3,8,7,9,10,11,12,13,14,15]` (cost ≈ 1156.468)
- **Shared edges (11 = 68.75%):** `1-2, 3-4, 5-6, 7-8, 9-10, 10-11, 11-12, 12-13, 13-14,
  14-15, 15-0`; **A-only (5):** `0-1, 2-3, 4-5, 6-7, 8-9`; **B-only (5):** `0-5, 1-4, 2-6,
  3-8, 7-9`.
- **AB-cycle C1 (length 4):** `[0,1,4,5]`. Applying it alone yields two subtours
  `[0,5,6,7,8,9,10,11,12,13,14,15]` (≈854.822) and `[1,4,3,2]` (≈228.473).
- **AB-cycle C2 (length 6):** `[2,3,8,9,7,6]`. Applying it alone yields one tour (≈1155.403).
- **Both cycles:** exactly Parent B.
- Every city in the difference has **difference degree 2** (one A-only and one B-only edge), so
  the AB-cycle decomposition is unique (no degree-4 pairing ambiguity). Cities 0–9 each have one
  shared edge (union degree 3); cities 10–15 have two shared edges each (union degree 2). There
  is **no degree-4 city**, so the union graph exercises degrees 2 and 3 only.
- The default patch merges C1's small subtour by removing `3-4` and `6-7`, adding `3-6` and
  `4-7` (Δ ≈ −71.265); the child is `[0,5,6,3,2,1,4,7,8,9,10,11,12,13,14,15]` at cost
  ≈1012.031 — cheaper than both parents. Best 2-opt move on the child: Δ ≈ −57.328 (remove
  `0-5` and `1-2`, add `0-1` and `5-2`).

Both parents are crossing-free (0 crossings each). Strict 2-opt local-optimality of the parents
is **not** asserted: searches could not find a pair satisfying it together with the cycle
structure, so the fixture prioritizes the crossover lesson (the one relaxed preference). If a
future implementation finds a both-local layout, it may replace the coordinates only after the
Vitest invariants and costs are updated together.

A Vitest test asserts every invariant above (permutations, shared ratio, difference degree 2,
two even cycles, C1→two subtours, C2→one tour, all-cycles→B, exact costs and patch delta), so a
future edit cannot silently turn the lesson into an accident. No runtime search or random
layout.

**Two cycles, not three.** The two-cycle fixture makes the EAX identity visible: selecting
*every* AB-cycle is exactly `E_A Δ (E_A Δ E_B) = E_B`, i.e. Parent B. The explainer names that
identity rather than presenting it as a patched child. A three-cycle fixture was considered so
`Rand`/multi-select could still show patching, but it complicates the legend and still has the
same all-cycles identity; the pinned C1 already demonstrates the patch path.

### 2. Pure-TypeScript simulation, no WASM

`eax-algo.ts` owns the fixture, union graph, AB-cycle decomposition, E-set selection, symmetric
difference, component detection, bridge patching, one-step 2-opt preview, and statistics. It is
a teaching model, not a solver: it may be simpler than the Rust (exhaustive bridge search
instead of candidate lists, a single event instead of a population). The explainer never claims
to reproduce a production run.

### 3. Six-stage storyboard with an explicit frame sequence

The component derives a flat `Frame[]` from the fixture and the current selection. Each frame
carries the stage, the visible edge layers, the highlighted cycle/subtour/bridge, and the copy
for that moment. Back and Step move one frame; stage buttons jump to a stage's first frame.
This keeps playback, stage jumps, and tests on one deterministic data structure instead of a
web of booleans.

Frame outline:

| Stage | Frames |
| --- | --- |
| 1 Compare Parents | both tours, lengths, shared-edge count |
| 2 Union Graph | shared edges fade in; A-only and B-only edges highlight |
| 3 AB-Cycles | one frame per cycle (alternating walk traced), then a summary |
| 4 Select E-set | 1-AB / Rand / click; selected cycles pulse, others dim |
| 5 Apply E-set | remove A-edges in the E-set, add B-edges, reveal components |
| 6 Patch Subtours | one frame per bridge (smallest component, cheapest pair), optional 2-opt, final child |

Stage 5 is two frames (remove the A-edges, then add the B-edges and reveal components); stage 6
adds a frame per bridge and a "no patching needed" frame when the E-set already forms one tour.
`Run` uses the current selection and does not block at stage 4; changing a cycle or strategy
pauses and rebuilds the frames after the selection. A cycle click on a later stage returns to
stage 4.

### 4. SVG, not canvas

Sixteen cities and at most ~27 union edges fit comfortably in SVG, which keeps hit targets,
focus rings, and screen-reader structure simple. The decorative graph is `aria-hidden`; a
visually hidden summary and a cycle/subtour table carry the text alternative.

### 5. Selection defaults and deterministic randomness

- Default strategy: `1-AB` (the label maps to the solver's `single_ab`), selecting **exactly one
  cycle — the pinned C1**, which produces two subtours. The solver picks the cycle at random; the
  explainer pins it for reproducibility and says so in the header/notes ("simplified for
  teaching").
- `Rand` (maps to the solver's `rand`) uses a small seeded PRNG (mulberry32) with a visible seed
  and a `Reshuffle` button. It selects each cycle with probability 1/2 and, if none is selected,
  picks one uniformly — matching `select_e_set`. With only two cycles this means each cycle is
  selected with probability 5/8 and the both-cycles result (Parent B) appears 1/4 of the time.
  No unseeded `Math.random`.
- Clicking a cycle or legend row toggles it and switches the strategy to a manual selection.
  The last active cycle cannot be deselected: an empty E-set would make the child identical to
  Parent A, which is not a crossover worth presenting as one.
- A stage or strategy change pauses playback and resets to that stage's first frame.
- Selecting every cycle has its own frame: the result is Parent B exactly, so the copy says
  "all AB-cycles selected: A becomes B, nothing to patch" instead of pretending a crossover
  happened.

### 6. Exhaustive patching for the 16-city fixture

The bridge search iterates every edge of the smallest component against every edge of the other
components and keeps the cheapest reconnection (the Rust uses candidate lists for n = 1000; the
explainer does not need them). Each accepted bridge removes one edge from each component and adds
two, so degree two is preserved and the component count drops by one. The exact delta is shown.
Each edge pair admits two rewirings; both are scored, and exact ties break on the canonical
vertex tuple `(min_u, max_v, min_x, max_y)` so Node/Vitest and browser runs never diverge.

### 7. One-move 2-opt preview

After patching, compute the best single 2-opt move over all edge pairs (small fixture), draw the
removed and added edges, and report the exact old/new lengths. A toggle enables it; the default
final state is the unpolished child so the EAX result is never confused with the polish. If no
improving move exists, the preview says the child is already 2-opt-local and draws no
replacement edges. The preview never applies a worsening or zero-delta swap; the toggle either
disables or reports "already 2-opt-local (Δ = 0)".

### 8. Teaching statistics are the quality-preservation lesson

The final panel reports shared edges and ratio, AB-cycle count/lengths, E-set size, subtours,
bridges added, child vs A vs B cost, and edge inheritance. Inheritance is a partition:
`shared` (in both parents), `from A` (A-only), `from B` (B-only), and `new` (in neither). The copy
states the point: EAX keeps most of the parents' good edges and only rewires the difference,
which is why a child is rarely much worse and often better than its parents.

### 9. Visual language

- Parent A: blue (`#3b82f6`); Parent B: amber (`#d97706`); bridge/polished: green
  (`#16a34a`) — the GA explainer's existing scoped palette. Shared edges use the explainer's
  muted text colour; removed/dissolving uses the site `--color-negative` (`#f87171`); the final
  child uses the site `--color-accent` (`#6366f1`). No new global tokens are introduced; red and
  indigo are existing site tokens used in a new role.
- **Keep cycle identity and component identity on separate visual channels.** AB-cycles are
  identified by badges and legend rows (not by recolouring edges); connected components in
  Stages 5–6 are shown with tinted city halos/rings keyed to `component_id`, while edges keep
  their A/B/removed/bridge provenance. This prevents `Cycle #1` and `Subtour #1` from reading as
  the same thing.
- **Stage 1 overlap:** draw Parent A and Parent B with a ±1.5 px perpendicular offset; on shared
  edges use an alternating blue/amber dashed stroke so both tours remain visibly complete before
  Stage 2 fades the shared backbone.
- Cycle badges use four categorical tints and a legend; selected cycles pulse once (functional
  state, not decorative motion). Cycle toggles are native buttons with `aria-pressed`, mirrored
  by legend rows; released keys and focus rings use the explainer's existing styles.
- The cycle/subtour summary is a real table with a caption and column headers. Stage changes are
  announced with `aria-live="polite"`, and a stage jump moves focus to the stage heading.
- Numeric values use the monospace stack; the surrounding copy uses the explainer's existing
  body stack.
- `prefers-reduced-motion`: Run jumps to the next stage/frame instead of animating; the pulse is
  replaced by a static outline.

### 10. Registration

Add `eax` to `EXPLAINER_META` (title, description, `backLabel: 'EAX'`), `EXPLAINER_SOLVERS`
in `nav-data.ts`, the static import and literal branch in the explainer route, and
`hasExplainer: true` in `docs/algorithms/memetic-eax.md`. The algorithms index and sidebar
derive from `EXPLAINER_META`, so they need no per-solver edit. `nav-data.test.ts` must stay green
(`solver-index.test.ts` only checks benchmark data and is unaffected). Bump the `registry.ts`
header comment from 22 to 23 explainers.

### 11. Tests

- **Vitest (`eax-algo.test.ts`)**: fixture invariants (permutations, shared ratio, difference
  degree 2 at every active city, exactly two even cycles, C1→two subtours, C2→one tour,
  all-cycles→Parent B), difference-graph degree 2 at active cities (union-graph degree 2/3/4),
  AB-cycle alternation, `1-AB`/`Rand` selection (pinned C1, seeded determinism, non-empty),
  symmetric difference preserves degree two, component detection, smallest-first patching, exact
  bridge delta, one-tour result, 2-opt preview, and statistics.
- **Playwright**: initial parents, six stage jumps, Step/Back, Run/Pause/Reset, strategy and
  cycle toggles, the empty-selection guard, seeded Rand reload, subtour labels, bridge stepping,
  2-opt preview and its no-improvement state, live-region/focus behavior, keyboard reachability,
  narrow viewport, docs/index links.
- **Registration guards**: the existing `nav-data` cross-check plus the explainer route build.

## Risks / Trade-offs

- **The lesson is only as good as the fixture.** → Pin the tours and assert every invariant
  (difference degree 2, two even cycles, C1→two subtours, C2→one tour, exact costs and patch
  delta) in Vitest; the default selection is tested to produce two subtours.
- **Too many stages/controls for one screen.** → One frame sequence, one stage bar, one legend;
  auto-play is off by default and each stage has a one-sentence lesson.
- **The explainer can be mistaken for the Rust solver.** → The header says "one crossover event,
  simplified for teaching"; no WASM call and no claim of reproducing production results.
- **Colour overload with two parents, cycles, subtours, and patches.** → One colour per role,
  selected/unselected shown by opacity and a static outline, cycle badges limited to four.
- **Playback and manual selection can disagree.** → Any manual control pauses and rebuilds the
  frame sequence from the selection; the pure module is the single source of truth.
- **Accessibility of an edge-heavy diagram.** → The graph is `aria-hidden`; the summary and
  cycle/subtour table carry the content, and every control is a native button/checkbox.

## Migration Plan

Additive only: a new explainer component/module, registry entry, route branch, and docs flag.
Rollback is deleting those entries and reverting the docs flag.

## Open Questions

None. Decisions locked: pinned 16-city fixture (68.75% shared, difference degree 2, C1→two
subtours, C2→one tour, child cheaper than both parents); default `1-AB` selects the pinned C1
(the solver's `single_ab` is random; the explainer pins for determinism); seeded `Rand` with the
uniform empty-fallback; SVG; exhaustive patching; one-move 2-opt preview; six-stage frame
sequence; existing explainer palette.

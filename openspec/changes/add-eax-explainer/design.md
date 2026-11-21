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
- A fixture where both parents are local optima, share a large edge fraction, and differ in
  exactly two non-trivial AB-cycles.
- Honest statistics: every length, delta, and inheritance count comes from the fixture.

**Non-Goals:**

- A one-to-one port of the Rust EAX (candidate lists, family replacement, full population
  loop, epochs, seeding).
- Calling the WASM solver or adding a debug export.
- Arbitrary uploaded instances, or a general EAX playground.
- A full 2-opt pass; the preview is one best move.
- Changing the solver, CLI, API, or benchmark code.

## Decisions

### 1. Pinned 24-city fixture with two locally optimal parents

A fixed synthetic layout plus two pinned tours are committed as constants in
`eax-algo.ts`. During implementation, generate candidates offline with a deterministic
2-opt search from two starts, then pin the pair that satisfies all of:

- both tours are permutations of the 24 cities;
- neither has an improving 2-opt move (they are local optima);
- they share at least 60% of their undirected edges;
- the symmetric difference decomposes into exactly **two** non-trivial AB-cycles, each of
  even length and at least four.

A Vitest test asserts every invariant, so a future edit cannot silently turn the lesson into
an accident. No runtime search or random layout.

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

### 4. SVG, not canvas

Twenty-four cities and at most ~48 union edges fit comfortably in SVG, which keeps hit targets,
focus rings, and screen-reader structure simple. The decorative graph is `aria-hidden`; a
visually hidden summary and a cycle/subtour table carry the text alternative.

### 5. Selection defaults and deterministic randomness

- Default strategy: `1-AB`, selecting the larger cycle. The fixture is chosen so this selection
  produces at least two subtours (the patching lesson is the point).
- `Rand` uses a small seeded PRNG (mulberry32) with a visible seed and a `Reshuffle` button.
  The same seed always selects the same cycles; no unseeded `Math.random`.
- Clicking a cycle or legend row toggles it and switches the strategy to a manual selection.
  The last active cycle cannot be deselected: an empty E-set would make the child identical to
  Parent A, which is not a crossover worth presenting as one.
- A stage or strategy change pauses playback and resets to that stage's first frame.

### 6. Exhaustive patching for the 24-city fixture

The bridge search iterates every edge of the smallest component against every edge of the other
components and keeps the cheapest reconnection (the Rust uses candidate lists for n = 1000; the
explainer does not need them). Each accepted bridge removes one edge from each component and adds
two, so degree two is preserved and the component count drops by one. The exact delta is shown.

### 7. One-move 2-opt preview

After patching, compute the best single 2-opt move over all edge pairs (small fixture), draw the
removed and added edges, and report the exact old/new lengths. A toggle enables it; the default
final state is the unpolished child so the EAX result is never confused with the polish.

### 8. Teaching statistics are the quality-preservation lesson

The final panel reports shared edges and ratio, AB-cycle count/lengths, E-set size, subtours,
bridges added, child vs A vs B cost, and edge inheritance (`from A`, `from B`, `new`). The copy
states the point: EAX keeps most of the parents' good edges and only rewires the difference,
which is why a child is rarely much worse and often better than its parents.

### 9. Visual language

- Parent A: blue (`#3b82f6`); Parent B: amber (`#d97706`); bridge/polished: green
  (`#16a34a`) — the GA explainer's existing scoped palette. Shared edges use the explainer's
  muted text colour; removed/dissolving uses the site `--color-negative` (`#f87171`); the final
  child uses the site `--color-accent` (`#6366f1`). No new global tokens are introduced; red and
  indigo are existing site tokens used in a new role.
- Cycle badges use four categorical tints and a legend; selected cycles pulse once (functional
  state, not decorative motion).
- Numeric values use the monospace stack; the surrounding copy uses the explainer's existing
  body stack.
- `prefers-reduced-motion`: Run jumps to the next stage/frame instead of animating; the pulse is
  replaced by a static outline.

### 10. Registration

Add `eax` to `EXPLAINER_META` (title, description, `backLabel: 'EAX'`), `EXPLAINER_SOLVERS`
in `nav-data.ts`, the static import and literal branch in the explainer route, and
`hasExplainer: true` in `docs/algorithms/memetic-eax.md`. The algorithms index and sidebar
derive from `EXPLAINER_META`, so they need no per-solver edit. `nav-data.test.ts` and
`solver-index.test.ts` must stay green.

### 11. Tests

- **Vitest (`eax-algo.test.ts`)**: fixture invariants (permutation, local optimality, shared
  ratio, two even cycles), union graph degree 2/4, AB-cycle alternation, `1-AB`/`Rand` selection
  (seeded determinism, non-empty), symmetric difference preserves degree two, component
  detection, smallest-first patching, exact bridge delta, one-tour result, 2-opt preview, and
  statistics.
- **Playwright**: initial parents, six stage jumps, Step/Back, Run/Pause/Reset, strategy and
  cycle toggles, seeded Rand reload, subtour labels, bridge stepping, 2-opt preview, keyboard
  reachability, narrow viewport, docs/index links.
- **Registration guards**: the existing `nav-data`/`solver-index` cross-checks plus the explainer
  route build.

## Risks / Trade-offs

- **The lesson is only as good as the fixture.** → Pin the tours, assert local optimality, the
  shared-edge ratio, and exactly two cycles in Vitest; reject a fixture that produces one
  component at the default selection.
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

None. Decisions locked: pinned 24-city fixture with two local optima; default `1-AB` with the
larger cycle; seeded `Rand`; SVG; exhaustive patching; one-move 2-opt preview; six-stage frame
sequence; existing explainer palette.

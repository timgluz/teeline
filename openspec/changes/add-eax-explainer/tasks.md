# Tasks

## 1. Fixture and pure simulation

- [ ] 1.1 Pin the fixture from `design.md` Decision 1 in `teeline-web/src/explainers/eax-algo.ts` (16 coordinates, Parent A, Parent B, C1 `[0,1,4,5]`, C2 `[2,3,8,9,7,6]`); no search at implementation time. Verify in `eax-algo.test.ts`: both are permutations; shared edges = 11 (68.75%); every active city has difference degree 2; exactly two even non-trivial cycles; applying C1 alone yields two subtours; applying C2 alone yields one tour; applying both yields Parent B.
- [ ] 1.2 Implement the union graph (shared, A-only, B-only edges) and expose per-city degrees; verify the test asserts difference degree 2 at every active city (union degree 2/3/4) and that shared edges are excluded from the difference graph.
- [ ] 1.3 Implement AB-cycle decomposition over the difference graph; verify every cycle alternates origin, has even length, the fixture yields exactly two cycles, and the decomposition is unique (difference degree 2 everywhere).
- [ ] 1.4 Implement E-set selection (`1-AB` → pinned C1, seeded `Rand` with 1/2 per cycle plus a uniform empty-fallback, manual cycle list); verify `1-AB` selects C1, `Rand` is reproducible for a seed and never empty, and manual selection toggles a cycle.
- [ ] 1.5 Implement the symmetric difference and connected components; verify every city keeps degree two, the default (`1-AB` = C1) produces exactly two subtours, applying C2 alone produces exactly one tour, and selecting every cycle reproduces Parent B exactly.
- [ ] 1.6 Implement smallest-first bridge patching with an exhaustive cheapest-pair search, including both rewirings and canonical tie-breaking; verify each step preserves degree two, reports the exact delta, ends in one Hamiltonian tour, and the pinned fixture's single bridge removes `3-4`/`6-7` and adds `3-6`/`4-7` (Δ ≈ −71.265).
- [ ] 1.7 Implement a one-move 2-opt preview over the patched child, including the no-improving-move state; verify the reported removed/added edges and delta are computed from fixture coordinates, and that no move is fabricated.
- [ ] 1.8 Implement teaching statistics (shared edges/ratio, cycle lengths, E-set size, subtours, bridges, child vs parent costs, shared/A/B/new edge inheritance); verify the test covers each field on the fixture.

## 2. Storyboard component

- [ ] 2.1 Render the SVG graph, city labels, parent/cycle legends, and cycle badges, with the Stage 1 ±1.5 px/alternating-shared-edge treatment and component halos in Stages 5–6; verify the graph is `aria-hidden`, the two identities use separate visual channels, and a text summary is present.
- [ ] 2.2 Add the six-stage bar, the derived frame sequence, Back/Step/Run-Pause/Reset, stage jumps, and reduced-motion handling; verify stepping forward/back reverses the same frames.
- [ ] 2.3 Wire cycle clicking, `1-AB`/`Rand` toggles, the seed/Reshuffle control, and selected/unselected styling; verify manual changes pause playback and rebuild the frames.
- [ ] 2.4 Add subtour colouring and labels, step-by-step bridge application, the final child tour, and the optional 2-opt preview; verify each bridge step advances one component.
- [ ] 2.5 Add the statistics panel, the screen-reader summary/table, keyboard focus styles, and the narrow-viewport layout; verify the panel reports edge inheritance and child/parent costs.
- [ ] 2.6 Verify the component with Vitest where practical and with Playwright for the full interaction flow.

## 3. Registration and docs

- [ ] 3.1 Register `eax` in `EXPLAINER_META`, `EXPLAINER_SOLVERS`, the static import/branch in `pages/algorithms/[id]/explainer/index.astro`, and set `hasExplainer: true` in `docs/algorithms/memetic-eax.md`; bump the `registry.ts` header comment from 22 to 23 explainers; verify `nav-data.test.ts` and `task web:test` pass.
- [ ] 3.2 Add Playwright coverage for the explainer route, the docs CTA, the algorithms-index link, keyboard operation, the empty-selection guard, the all-cycles→Parent-B state, the no-improving 2-opt state, live-region/focus behavior, seeded `Rand` reload, and a narrow viewport; verify the spec file passes.

## 4. Integration gate

- [ ] 4.1 Run `npx tsc --noEmit`, `task web:test`, `task web:build`, the new Playwright spec, and `markdownlint` on the docs page; verify every command exits 0 and the explainer route renders in the built site.

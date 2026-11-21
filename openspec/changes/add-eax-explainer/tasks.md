# Tasks

## 1. Fixture and pure simulation

- [ ] 1.1 Pin a 24-city fixture and two parent tours in `teeline-web/src/explainers/eax-algo.ts`; verify both are permutations, both are 2-opt local optima, shared edges ≥60%, and the symmetric difference has exactly two even non-trivial cycles in `eax-algo.test.ts`.
- [ ] 1.2 Implement the union graph (shared, A-only, B-only edges) and expose per-city degrees; verify the test asserts degree two or four and that shared edges are excluded from the difference graph.
- [ ] 1.3 Implement AB-cycle decomposition over the difference graph; verify every cycle alternates origin, has even length, and the fixture yields two cycles.
- [ ] 1.4 Implement E-set selection (`1-AB`, seeded `Rand`, manual cycle list); verify `1-AB` selects one cycle, `Rand` is reproducible for a seed and never empty, and manual selection toggles a cycle.
- [ ] 1.5 Implement the symmetric difference and connected components; verify every city keeps degree two and the default selection produces at least two subtours.
- [ ] 1.6 Implement smallest-first bridge patching with an exhaustive cheapest-pair search; verify each step preserves degree two, reports the exact delta, and ends in one Hamiltonian tour.
- [ ] 1.7 Implement a one-move 2-opt preview over the patched child; verify the reported removed/added edges and delta are computed from fixture coordinates.
- [ ] 1.8 Implement teaching statistics (shared edges/ratio, cycle lengths, E-set size, subtours, bridges, child vs parent costs, A/B/new edge inheritance); verify the test covers each field on the fixture.

## 2. Storyboard component

- [ ] 2.1 Render the SVG graph, city labels, parent/cycle legends, and cycle badges; verify the graph is `aria-hidden` and a text summary is present.
- [ ] 2.2 Add the six-stage bar, the derived frame sequence, Back/Step/Run-Pause/Reset, stage jumps, and reduced-motion handling; verify stepping forward/back reverses the same frames.
- [ ] 2.3 Wire cycle clicking, `1-AB`/`Rand` toggles, the seed/Reshuffle control, and selected/unselected styling; verify manual changes pause playback and rebuild the frames.
- [ ] 2.4 Add subtour colouring and labels, step-by-step bridge application, the final child tour, and the optional 2-opt preview; verify each bridge step advances one component.
- [ ] 2.5 Add the statistics panel, the screen-reader summary/table, keyboard focus styles, and the narrow-viewport layout; verify the panel reports edge inheritance and child/parent costs.
- [ ] 2.6 Verify the component with Vitest where practical and with Playwright for the full interaction flow.

## 3. Registration and docs

- [ ] 3.1 Register `eax` in `EXPLAINER_META`, `EXPLAINER_SOLVERS`, the static import/branch in `pages/algorithms/[id]/explainer/index.astro`, and set `hasExplainer: true` in `docs/algorithms/memetic-eax.md`; verify `nav-data.test.ts` and `task web:test` pass.
- [ ] 3.2 Add Playwright coverage for the explainer route, the docs CTA, the algorithms-index link, keyboard operation, seeded `Rand` reload, and a narrow viewport; verify the spec file passes.

## 4. Integration gate

- [ ] 4.1 Run `npx tsc --noEmit`, `task web:test`, `task web:build`, the new Playwright spec, and `markdownlint` on the docs page; verify every command exits 0 and the explainer route renders in the built site.

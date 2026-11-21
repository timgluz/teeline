# Proposal

## Why

Memetic EAX is Teeline's strongest metaheuristic on the TSPLIB benchmark set, yet its
mechanism is invisible on the website. A visitor sees a good tour but not *why* edge-based
crossover preserves quality: shared parent edges survive, only the symmetric difference
changes, subtours are patched, and local search polishes the child. Existing explainers show
permutation crossovers (OX) that visibly scramble tours; reusing that story for EAX would
teach the opposite of what the solver does.

## What Changes

- Add `/algorithms/eax/explainer/`, a six-stage interactive storyboard:
  1. compare Parent A and Parent B;
  2. build the union graph (shared edges muted, A/B-only edges highlighted);
  3. partition into non-trivial AB-cycles;
  4. select the E-set (`1-AB`, `Rand`, or click a cycle);
  5. apply the symmetric difference and show the resulting subtours;
  6. merge subtours with cheapest-bridge patching, preview one 2-opt move, and settle on the
     final child tour.
- Add a pure-TypeScript `eax-algo.ts` simulation for a pinned 24-city fixture: union graph,
  AB-cycle decomposition, E-set selection, symmetric difference, component detection, bridge
  patching, one-step 2-opt preview, and teaching statistics. It is a visualization, not a
  second solver path; the production solve path is unchanged.
- Add reversible playback (Back, Step, Run/Pause, Reset), stage jumps, cycle selection, a
  seeded `Rand` shuffle, and an optional 2-opt preview.
- Add teaching statistics that explain quality preservation: shared edges before/after, AB
  cycle count and lengths, E-set size, subtours, bridges added, child vs parent costs, and
  edge inheritance (A / B / new).
- Register the explainer in `EXPLAINER_META` / `EXPLAINER_SOLVERS`, set `hasExplainer: true`
  on the EAX docs page, and add the static Astro import/branch.
- Keep the explainer accessible (keyboard, screen-reader summary/table, reduced motion) and
  responsive.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-explainers`: add requirements for an EAX explainer that is discoverable, staged in six
  steps, exposes the union graph and AB-cycles, lets the visitor select the E-set, shows the
  symmetric-difference subtours and greedy patching, exposes teaching statistics, and remains
  accessible, responsive, and deterministic.

## Impact

- `teeline-web/src/explainers/eax-algo.ts` (new pure simulation) and `eax.tsx` (new Preact
  component), plus Vitest/Playwright tests.
- `teeline-web/src/explainers/registry.ts`, `nav-data.ts`, the explainer route, and
  `docs/algorithms/memetic-eax.md` (`hasExplainer: true`).
- No solver, WASM, API, CLI, or benchmark changes. Additive only.

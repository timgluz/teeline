// Metadata registry for the 10 interactive explainer pages. Only metadata
// lives here — the components themselves must be statically imported
// directly in [id]/explainer/index.astro, because Astro's client:* hydration
// directives require the compiler to see a literal top-level `import`
// statement for whatever component gets hydrated; a dynamic import() hidden
// behind a loader function (the first approach tried here) compiles fine but
// fails at render time with "NoMatchingImport" since Astro can't statically
// resolve which client bundle to ship.
export interface ExplainerMeta {
  title: string
  description: string
  backLabel: string
}

export const EXPLAINER_META: Record<string, ExplainerMeta> = {
  pso: {
    title: 'Particle Swarm Optimisation — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Particle Swarm Optimisation: watch a swarm of tours evolve via velocity-capped swap sequences with decaying inertia.',
    backLabel: 'PSO',
  },
  gsa: {
    title: 'Gravitational Search Algorithm — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of the Gravitational Search Algorithm: watch agents (tours) attract each other by mass (fitness), apply swap-move velocities, and converge as G decays.',
    backLabel: 'GSA',
  },
  tabu: {
    title: 'Tabu Search — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Tabu Search: watch a tour evolve via best-admissible 2-opt moves, forbidden moves age off the tabu list, and aspiration overrides unlock new global bests.',
    backLabel: 'Tabu Search',
  },
  ga: {
    title: 'Genetic Algorithm — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of the Genetic Algorithm: watch selection, ordered crossover, and mutation evolve a population of tours generation by generation.',
    backLabel: 'GA',
  },
  cs: {
    title: 'Cuckoo Search — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Cuckoo Search: watch nests evolve via Lévy-flight 2-opt reversals, host competition, and Bernoulli nest abandonment.',
    backLabel: 'CS',
  },
  fpa: {
    title: 'Flower Pollination — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of the Flower Pollination Algorithm: watch a population of tours evolve via Lévy-flight global pollination and ε-scaled local cross-pollination.',
    backLabel: 'FPA',
  },
  lk: {
    title: 'Lin-Kernighan ILS — Interactive Explainer — Teeline',
    description: 'Watch simplified 2-opt local search and double-bridge perturbation run step-by-step in your browser.',
    backLabel: 'LK',
  },
  sa: {
    title: 'Simulated Annealing — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Simulated Annealing: watch temperature cooling drive the shift from broad exploration to fine exploitation via Metropolis acceptance.',
    backLabel: 'SA',
  },
  som: {
    title: 'Kohonen SOM — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of the Kohonen Self-Organizing Map: watch the neuron ring expand, order itself around cities, and extract a tour.',
    backLabel: 'SOM',
  },
  fourier: {
    title: 'Fourier Solver — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of the Fourier-basis constructive TSP solver: watch gradient descent shape a closed curve into a valid tour.',
    backLabel: 'Fourier',
  },
}

export const EXPLAINER_IDS = Object.keys(EXPLAINER_META)

// Metadata registry for the 15 interactive explainer pages. Only metadata
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
  greedy_edge: {
    title: 'Greedy Edge Construction — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Greedy Edge Construction: watch every pairwise edge scan shortest-first, accepted unless it would create degree 3+ or a premature sub-cycle, with union-find components merging until one Hamiltonian cycle remains.',
    backLabel: 'Greedy Edge',
  },
  savings: {
    title: 'Savings Construction — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Savings Construction: watch every pairwise edge scan highest-savings-first, ranked by the Clarke-Wright savings formula s(i,j) = d(hub,i) + d(hub,j) − d(i,j), with a centroid-nearest hub city bias and the same Kruskal-style accept/reject guards as Greedy Edge.',
    backLabel: 'Savings',
  },
  aco: {
    title: 'Ant Colony Optimization — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Ant Colony Optimization: watch a colony of ants construct tours probabilistically, biased by a shared pheromone matrix that strengthens on short edges and decays over epochs. Adjust α (pheromone influence), β (heuristic weight), evaporation rate, and colony size.',
    backLabel: 'ACO',
  },
  stochastic_hill: {
    title: 'Stochastic Hill Climbing — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Stochastic Hill Climbing: watch random 2-opt candidates get accepted only when they beat the best tour so far, with random restarts when the search goes stale.',
    backLabel: 'Stochastic Hill Climbing',
  },
  or_opt: {
    title: 'Or-opt — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of Or-opt: watch segments of 1–3 consecutive cities get cut from one part of the tour and pasted into a better position, with reversed insertions for Or-2/Or-3 and best-improvement passes running to a local optimum.',
    backLabel: 'Or-opt',
  },
  '2opt': {
    title: '2-opt — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of the 2-opt local-search algorithm: watch edge swaps eliminate crossings one pass at a time, with removed edges highlighted red and new edges green. Step through best-improvement scans until the tour reaches a local optimum.',
    backLabel: '2-opt',
  },
  nn: {
    title: 'Nearest Neighbor — Interactive Explainer — Teeline',
    description:
      'Interactive walkthrough of the Nearest Neighbor constructive TSP heuristic: watch the tour grow one city at a time as the algorithm greedily picks the closest unvisited city. Step through each decision and see how early choices shape the final tour.',
    backLabel: 'NN',
  },
}

export const EXPLAINER_IDS = Object.keys(EXPLAINER_META)

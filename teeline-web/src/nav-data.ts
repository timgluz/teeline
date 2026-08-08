// Plain data module — no framework/build-tool coupling. Consumed by
// Topbar.astro / Sidebar.astro (rendering) and src/content/config.ts
// (hasExplainer cross-check in getStaticPaths).

export interface SolverMeta {
  id: string
  name: string
}

export const SOLVER_META: Record<string, SolverMeta> = {
  bhk: { id: 'bhk', name: 'Bellman-Held-Karp' },
  branch_bound: { id: 'branch_bound', name: 'Branch & Bound' },
  nn: { id: 'nn', name: 'Nearest Neighbor' },
  fourier: { id: 'fourier', name: 'Fourier' },
  christofides: { id: 'christofides', name: 'Christofides' },
  greedy_edge: { id: 'greedy_edge', name: 'Greedy Edge' },
  '2opt': { id: '2opt', name: '2-opt' },
  '3opt': { id: '3opt', name: '3-opt' },
  or_opt: { id: 'or_opt', name: 'Or-opt' },
  stochastic_hill: { id: 'stochastic_hill', name: 'Stochastic Hill Climbing' },
  lk: { id: 'lk', name: 'Lin-Kernighan' },
  sa: { id: 'sa', name: 'Simulated Annealing' },
  tabu: { id: 'tabu', name: 'Tabu Search' },
  ga: { id: 'ga', name: 'Genetic Algorithm' },
  pso: { id: 'pso', name: 'Particle Swarm' },
  cs: { id: 'cs', name: 'Cuckoo Search' },
  fpa: { id: 'fpa', name: 'Flower Pollination' },
  gsa: { id: 'gsa', name: 'Gravitational Search' },
  som: { id: 'som', name: 'Kohonen SOM' },
}

export interface SolverGroup {
  label: string
  ids: string[]
}

export const SOLVER_GROUPS: SolverGroup[] = [
  { label: 'Exact', ids: ['bhk', 'branch_bound'] },
  { label: 'Constructive', ids: ['nn', 'fourier', 'christofides', 'greedy_edge', 'som'] },
  { label: 'Local search', ids: ['2opt', '3opt', 'or_opt', 'stochastic_hill', 'lk'] },
  { label: 'Metaheuristic', ids: ['sa', 'tabu', 'ga', 'pso', 'cs', 'fpa', 'gsa'] },
]

// Every id in SOLVER_META has a generated doc page under /algorithms/<id>/.
export const PAGED_SOLVERS = new Set(Object.keys(SOLVER_META))

// The subset of ids with an interactive explainer under /algorithms/<id>/explainer/.
export const EXPLAINER_SOLVERS = new Set([
  'pso', 'gsa', 'tabu', 'ga', 'cs', 'fpa', 'lk', 'sa', 'som', 'fourier', 'greedy_edge',
])

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
  aco: { id: 'aco', name: 'Ant Colony Optimization' },
  savings: { id: 'savings', name: 'Savings' },
}

export interface SolverGroup {
  label: string
  ids: string[]
}

export const SOLVER_GROUPS: SolverGroup[] = [
  { label: 'Exact', ids: ['bhk', 'branch_bound'] },
  { label: 'Constructive', ids: ['nn', 'fourier', 'christofides', 'greedy_edge', 'savings', 'som'] },
  { label: 'Local search', ids: ['2opt', '3opt', 'or_opt', 'stochastic_hill', 'lk'] },
  { label: 'Metaheuristic', ids: ['sa', 'tabu', 'ga', 'pso', 'cs', 'fpa', 'gsa', 'aco'] },
]

// Every id in SOLVER_META has a generated doc page under /algorithms/<id>/.
export const PAGED_SOLVERS = new Set(Object.keys(SOLVER_META))

// The subset of ids with an interactive explainer under /algorithms/<id>/explainer/.
export const EXPLAINER_SOLVERS = new Set([
  'pso', 'gsa', 'tabu', 'ga', 'cs', 'fpa', 'lk', 'sa', 'som', 'fourier', 'greedy_edge', 'savings', 'aco',
])

// ── Problem (dataset) navigation ───────────────────────────────────────

export interface ProblemMeta {
  id: string
  name: string
  cities: number
}

export const PROBLEM_META: Record<string, ProblemMeta> = {
  att48:          { id: 'att48', name: 'ATT 48', cities: 48 },
  bayg29:         { id: 'bayg29', name: 'Bayg 29', cities: 29 },
  bays29:         { id: 'bays29', name: 'Bays 29', cities: 29 },
  berlin52:       { id: 'berlin52', name: 'Berlin 52', cities: 52 },
  brazil58:       { id: 'brazil58', name: 'Brazil 58', cities: 58 },
  burma14:        { id: 'burma14', name: 'Burma 14', cities: 14 },
  dantzig42:      { id: 'dantzig42', name: 'Dantzig 42', cities: 42 },
  eil51:          { id: 'eil51', name: 'Eil 51', cities: 51 },
  eil76:          { id: 'eil76', name: 'Eil 76', cities: 76 },
  fri26:          { id: 'fri26', name: 'Fri 26', cities: 26 },
  gr17:           { id: 'gr17', name: 'Gr 17', cities: 17 },
  gr21:           { id: 'gr21', name: 'Gr 21', cities: 21 },
  gr24:           { id: 'gr24', name: 'Gr 24', cities: 24 },
  gr48:           { id: 'gr48', name: 'Gr 48', cities: 48 },
  gr96:           { id: 'gr96', name: 'Gr 96', cities: 96 },
  hk48:           { id: 'hk48', name: 'HK 48', cities: 48 },
  kroA100:        { id: 'kroA100', name: 'KroA 100', cities: 100 },
  kroB100:        { id: 'kroB100', name: 'KroB 100', cities: 100 },
  kroC100:        { id: 'kroC100', name: 'KroC 100', cities: 100 },
  kroD100:        { id: 'kroD100', name: 'KroD 100', cities: 100 },
  kroE100:        { id: 'kroE100', name: 'KroE 100', cities: 100 },
  pr76:           { id: 'pr76', name: 'Pr 76', cities: 76 },
  rat99:          { id: 'rat99', name: 'Rat 99', cities: 99 },
  rd100:          { id: 'rd100', name: 'Rd 100', cities: 100 },
  st70:           { id: 'st70', name: 'St 70', cities: 70 },
  swiss42:        { id: 'swiss42', name: 'Swiss 42', cities: 42 },
  ulysses16:      { id: 'ulysses16', name: 'Ulysses 16', cities: 16 },
  ulysses22:      { id: 'ulysses22', name: 'Ulysses 22', cities: 22 },
}

export const PROBLEM_GROUPS = [
  { label: 'Small (≤100)', ids: Object.keys(PROBLEM_META) },
]

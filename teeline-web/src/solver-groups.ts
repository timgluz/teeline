export interface SolverMeta {
  id: string
  name: string
}

export const SOLVER_META: Record<string, SolverMeta> = {
  'bhk':          { id: 'bhk',          name: 'Bellman-Held-Karp' },
  'branch_bound': { id: 'branch_bound', name: 'Branch & Bound' },
  'nn':           { id: 'nn',           name: 'Nearest Neighbor' },
  'fourier':      { id: 'fourier',      name: 'Fourier' },
  'christofides': { id: 'christofides', name: 'Christofides' },
  '2opt':         { id: '2opt',         name: '2-opt' },
  '3opt':         { id: '3opt',         name: '3-opt' },
  'or_opt':       { id: 'or_opt',       name: 'Or-opt' },
  'lk':           { id: 'lk',           name: 'Lin-Kernighan' },
  'sa':           { id: 'sa',           name: 'Simulated Annealing' },
  'tabu':         { id: 'tabu',         name: 'Tabu Search' },
  'ga':           { id: 'ga',           name: 'Genetic Algorithm' },
  'pso':          { id: 'pso',          name: 'Particle Swarm' },
  'cs':           { id: 'cs',           name: 'Cuckoo Search' },
  'fpa':          { id: 'fpa',          name: 'Flower Pollination' },
  'gsa':          { id: 'gsa',          name: 'Gravitational Search' },
}

export interface SolverGroup {
  label: string
  ids: string[]
}

export const SOLVER_GROUPS: SolverGroup[] = [
  { label: 'Exact',         ids: ['bhk', 'branch_bound'] },
  { label: 'Constructive',  ids: ['nn', 'fourier', 'christofides'] },
  { label: 'Local search',  ids: ['2opt', '3opt', 'or_opt', 'lk'] },
  { label: 'Metaheuristic', ids: ['sa', 'tabu', 'ga', 'pso', 'cs', 'fpa', 'gsa'] },
]

export const PAGED_SOLVERS = new Set<string>([
  'bhk', 'branch_bound', 'nn', 'fourier', 'christofides',
  '2opt', '3opt', 'or_opt', 'lk', 'sa', 'tabu', 'ga', 'pso', 'cs', 'fpa', 'gsa',
])

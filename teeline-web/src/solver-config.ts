import type { SolveOptions } from './solver-options'

export type SolverKind = 'exact' | 'constructive' | 'local-search' | 'metaheuristic' | 'utility'

export interface SolverParam {
  key: keyof SolveOptions
  label: string
  type: 'int' | 'float'
  min?: number
  max?: number
  step?: number
  description?: string
}

export interface SolverMeta {
  name: string
  alias: string
  kind: SolverKind
  description: string
  params: SolverParam[]
}

const SHARED_HEURISTIC_PARAMS: SolverParam[] = [
  {
    key: 'epochs',
    label: 'Epochs',
    type: 'int',
    min: 1,
    description: 'Number of iterations to run',
  },
  {
    key: 'platooEpochs',
    label: 'Plateau epochs',
    type: 'int',
    min: 0,
    description: 'Stop early after this many epochs without improvement (0 = disabled)',
  },
  {
    key: 'nNearest',
    label: 'Nearest neighbours',
    type: 'int',
    min: 1,
    description: 'Candidate list size for neighbourhood search',
  },
]

const MUTATION_PARAM: SolverParam = {
  key: 'mutationProbability',
  label: 'Mutation probability',
  type: 'float',
  min: 0,
  max: 1,
  step: 0.001,
  description: 'Probability of applying a random mutation per individual',
}

export const SOLVERS: SolverMeta[] = [
  {
    name: 'Bellman-Held-Karp',
    alias: 'bhk',
    kind: 'exact',
    description: 'Exact dynamic programming. Practical limit ~20 cities.',
    params: [],
  },
  {
    name: 'Branch & Bound',
    alias: 'branch_bound',
    kind: 'exact',
    description: 'Exact branch-and-bound search. Practical limit ~15 cities.',
    params: [],
  },
  {
    name: 'Nearest Neighbour',
    alias: 'nn',
    kind: 'constructive',
    description: 'Greedy construction heuristic. Fast, deterministic.',
    params: [],
  },
  {
    name: '2-opt',
    alias: '2opt',
    kind: 'local-search',
    description: 'Local search: iteratively reverses edges to reduce tour length.',
    params: SHARED_HEURISTIC_PARAMS,
  },
  {
    name: '3-opt',
    alias: '3opt',
    kind: 'local-search',
    description: 'Local search: considers triple-edge reconnections.',
    params: SHARED_HEURISTIC_PARAMS,
  },
  {
    name: 'Simulated Annealing',
    alias: 'sa',
    kind: 'metaheuristic',
    description: 'Probabilistically accepts worse solutions to escape local optima.',
    params: [
      ...SHARED_HEURISTIC_PARAMS,
      {
        key: 'coolingRate',
        label: 'Cooling rate',
        type: 'float',
        min: 0.00001,
        max: 0.9999,
        step: 0.00001,
        description: 'Temperature reduction per epoch (0 < α < 1)',
      },
      {
        key: 'maxTemperature',
        label: 'Max temperature',
        type: 'float',
        min: 0.01,
        step: 1,
        description: 'Starting temperature (must be > min temperature)',
      },
      {
        key: 'minTemperature',
        label: 'Min temperature',
        type: 'float',
        min: 0,
        step: 0.001,
        description: 'Temperature floor; solver stops when reached',
      },
    ],
  },
  {
    name: 'Genetic Algorithm',
    alias: 'ga',
    kind: 'metaheuristic',
    description: 'Evolves a population of tours via crossover and mutation.',
    params: [
      ...SHARED_HEURISTIC_PARAMS,
      MUTATION_PARAM,
      {
        key: 'nElite',
        label: 'Elite count',
        type: 'int',
        min: 1,
        description: 'Number of top individuals preserved unchanged each generation',
      },
    ],
  },
  {
    name: 'Particle Swarm',
    alias: 'pso',
    kind: 'metaheuristic',
    description: 'Discrete PSO with velocity-capped, linearly decaying inertia.',
    params: SHARED_HEURISTIC_PARAMS,
  },
  {
    name: 'Cuckoo Search',
    alias: 'cs',
    kind: 'metaheuristic',
    description: 'Lévy-flight nest search with random nest abandonment.',
    params: [...SHARED_HEURISTIC_PARAMS, MUTATION_PARAM],
  },
  {
    name: 'Flower Pollination',
    alias: 'fpa',
    kind: 'metaheuristic',
    description: 'Global Lévy-flight toward best; local cross-pollination.',
    params: [...SHARED_HEURISTIC_PARAMS, MUTATION_PARAM],
  },
  {
    name: 'Stochastic Hill Climb',
    alias: 'stochastic_hill',
    kind: 'metaheuristic',
    description: 'Hill climbing with random restarts.',
    params: SHARED_HEURISTIC_PARAMS,
  },
  {
    name: 'Tabu Search',
    alias: 'tabu_search',
    kind: 'metaheuristic',
    description: 'Local search with a short-term memory of recently visited solutions.',
    params: SHARED_HEURISTIC_PARAMS,
  },
  {
    name: 'Random Shuffle',
    alias: 'shuffle',
    kind: 'utility',
    description: 'Baseline: random tour order.',
    params: [],
  },
]

export function solverByAlias(alias: string): SolverMeta | undefined {
  return SOLVERS.find((s) => s.alias === alias)
}

export function paramsFor(alias: string): SolverParam[] {
  return solverByAlias(alias)?.params ?? []
}

// Canonical benchmark solver index for the website.
//
// Maps the algorithm doc id — the join key shared by the `docs` content
// collection, `nav-data.ts` and the `/algorithms/{id}/` route — to the
// canonical CLI name that the benchmark harness invokes and to the shard
// filename used under `bench/v1/algorithms/` on R2.
//
// Kept in sync with the repo-root `bench/solvers.json` (the same data, read by
// the shell scripts) by `solver-index.test.ts`, which fails if the two lists
// drift. Display names live in nav-data.ts's SOLVER_META; this module
// deliberately carries only the fields the benchmark pipeline needs, so the two
// do not duplicate each other.

export type SolverFamily =
  'exact' | 'constructive' | 'local' | 'meta' | 'utility'

export interface SolverIndexEntry {
  /** Algorithm doc id — also the `/algorithms/{id}/` route param and R2 shard name. */
  id: string
  /** Canonical CLI name (`teeline solvers` NAME column, `Solvers::variants()`). */
  cli: string
  family: SolverFamily
  /**
   * Whether repeated runs of the same command produce the identical tour.
   * Measured, not inferred from the source: see `bench/solvers.json`.
   * A `false` here means the published gap is a distribution (median/best/worst),
   * never a single authoritative number.
   */
  deterministic: boolean
  /** Filename of the algorithm doc under docs/algorithms/, without extension. Null for utilities with no page. */
  doc: string | null
}

export const SOLVER_INDEX: readonly SolverIndexEntry[] = [
  {
    id: 'bhk',
    cli: 'bellman_karp',
    family: 'exact',
    deterministic: true,
    doc: 'bellman-held-karp',
  },
  {
    id: 'branch_bound',
    cli: 'branch_bound',
    family: 'exact',
    deterministic: true,
    doc: 'branch-bound',
  },
  {
    id: 'nn',
    cli: 'nearest_neighbor',
    family: 'constructive',
    deterministic: false,
    doc: 'nearest-neighbor',
  },
  {
    id: 'christofides',
    cli: 'christofides',
    family: 'constructive',
    deterministic: true,
    doc: 'christofides',
  },
  {
    id: 'greedy_edge',
    cli: 'greedy_edge',
    family: 'constructive',
    deterministic: true,
    doc: 'greedy-edge',
  },
  {
    id: 'savings',
    cli: 'savings',
    family: 'constructive',
    deterministic: true,
    doc: 'savings',
  },
  {
    id: 'fourier',
    cli: 'fourier',
    family: 'constructive',
    deterministic: false,
    doc: 'fourier',
  },
  {
    id: 'som',
    cli: 'kohonen_som',
    family: 'constructive',
    deterministic: false,
    doc: 'som',
  },
  {
    id: '2opt',
    cli: 'two_opt',
    family: 'local',
    deterministic: true,
    doc: 'two-opt',
  },
  {
    id: '3opt',
    cli: 'three_opt',
    family: 'local',
    deterministic: true,
    doc: 'three-opt',
  },
  {
    id: 'or_opt',
    cli: 'or_opt',
    family: 'local',
    deterministic: true,
    doc: 'or-opt',
  },
  {
    id: 'lk',
    cli: 'lin_kernighan',
    family: 'local',
    deterministic: false,
    doc: 'lin-kernighan',
  },
  {
    id: 'stochastic_hill',
    cli: 'stochastic_hill',
    family: 'local',
    deterministic: false,
    doc: 'stochastic-hill',
  },
  {
    id: 'sa',
    cli: 'simulated_annealing',
    family: 'meta',
    deterministic: false,
    doc: 'simulated-annealing',
  },
  {
    id: 'tabu',
    cli: 'tabu_search',
    family: 'meta',
    deterministic: false,
    doc: 'tabu-search',
  },
  {
    id: 'ga',
    cli: 'genetic_algorithm',
    family: 'meta',
    deterministic: false,
    doc: 'genetic-algorithm',
  },
  {
    id: 'pso',
    cli: 'particle_swarm',
    family: 'meta',
    deterministic: false,
    doc: 'particle-swarm',
  },
  {
    id: 'cs',
    cli: 'cuckoo_search',
    family: 'meta',
    deterministic: false,
    doc: 'cuckoo-search',
  },
  {
    id: 'fpa',
    cli: 'flower_pollination',
    family: 'meta',
    deterministic: false,
    doc: 'flower-pollination',
  },
  {
    id: 'gsa',
    cli: 'gravitational_search',
    family: 'meta',
    deterministic: false,
    doc: 'gravitational-search',
  },
  {
    id: 'aco',
    cli: 'ant_colony',
    family: 'meta',
    deterministic: false,
    doc: 'ant-colony',
  },
  {
    id: 'shuffle',
    cli: 'random_shuffle',
    family: 'utility',
    deterministic: false,
    doc: null,
  },
]

const BY_ID = new Map(SOLVER_INDEX.map((s) => [s.id, s]))

/** Looks up a solver by doc id. Returns undefined for ids with no benchmark entry. */
export function solverById(id: string): SolverIndexEntry | undefined {
  return BY_ID.get(id)
}

/**
 * Whether every solver in the index is marked deterministic. Used by the UI to
 * decide whether to caption a table as "single run" or "median of N runs" —
 * a page whose solver is non-deterministic must never present one number as
 * *the* result.
 */
export function isDeterministic(id: string): boolean {
  return BY_ID.get(id)?.deterministic ?? false
}

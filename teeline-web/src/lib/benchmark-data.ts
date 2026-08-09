export type Family = 'exact' | 'constructive' | 'local' | 'meta'

export interface BenchmarkPoint {
  id: string
  family: Family
  gapPct: number
  runtimeMs: number
}

// Data from docs/benchmarks.md — berlin52, release build v1.0.1/v1.0.11.
// gapPct = gap from optimal (7544.37); runtimeMs = wall time.
// Points marked as "estimated" are interpolated or from the quality-vs-speed
// observations section and will be replaced with measured runs later.
export const BERLIN52_BENCHMARKS: BenchmarkPoint[] = [
  { id: 'hk', family: 'exact', gapPct: 0, runtimeMs: 15000 },
  { id: 'branch_bound', family: 'exact', gapPct: 0, runtimeMs: 20000 },
  { id: 'nn', family: 'constructive', gapPct: 19.0, runtimeMs: 10 },
  { id: 'fourier', family: 'constructive', gapPct: 13.3, runtimeMs: 2500 },
  { id: 'christofides', family: 'constructive', gapPct: 15.4, runtimeMs: 10 },
  { id: 'greedy_edge', family: 'constructive', gapPct: 31.9, runtimeMs: 10 },
  { id: 'savings', family: 'constructive', gapPct: 11.5, runtimeMs: 10 },
  { id: 'som', family: 'constructive', gapPct: 13.5, runtimeMs: 620 },
  { id: '2opt', family: 'local', gapPct: 24.2, runtimeMs: 10 },
  { id: '3opt', family: 'local', gapPct: 2.6, runtimeMs: 300 },
  { id: 'or_opt', family: 'local', gapPct: 7.3, runtimeMs: 30 },
  { id: 'lk', family: 'local', gapPct: 0, runtimeMs: 150 },
  { id: 'sa', family: 'meta', gapPct: 6.8, runtimeMs: 340 },
  { id: 'ga', family: 'meta', gapPct: 7.5, runtimeMs: 3170 },
  { id: 'pso', family: 'meta', gapPct: 17.6, runtimeMs: 840 },
  { id: 'cs', family: 'meta', gapPct: 4.4, runtimeMs: 720 },
  { id: 'fpa', family: 'meta', gapPct: 17.5, runtimeMs: 530 },
  { id: 'gsa', family: 'meta', gapPct: 145, runtimeMs: 1200 },
  { id: 'stochastic_hill', family: 'meta', gapPct: 11.2, runtimeMs: 20 },
  { id: 'tabu', family: 'meta', gapPct: 23.8, runtimeMs: 40 },
]

export const FAMILY_COLORS: Record<Family, string> = {
  exact: '#F87171',
  constructive: '#22D3A5',
  local: '#6366F1',
  meta: '#F5A524',
}

export const FAMILY_LABELS: Record<Family, string> = {
  exact: 'Exact',
  constructive: 'Constructive',
  local: 'Local search',
  meta: 'Metaheuristic',
}

// Solver ids to label explicitly on the chart (notable points).
export const LABELED_IDS = new Set(['hk', 'nn', 'christofides', '2opt', 'lk', 'cs'])

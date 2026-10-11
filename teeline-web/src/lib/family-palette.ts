// Family colours and labels, shared by the landing chart and the algorithm page.
//
// This module used to hold BERLIN52_BENCHMARKS as well: a hand-maintained array that was 20 of
// 22 solvers, carried an `hk` entry corresponding to no solver, and omitted `aco`, `bhk` and
// `shuffle`. Published shards replaced it, and only the palette was worth keeping.
//
// `utility` exists because `shuffle` is measured and published as a baseline: a family with no
// colour would silently render as a missing point.

import type { SolverFamily } from './solver-index'

export const FAMILY_COLORS: Record<SolverFamily, string> = {
  exact: '#F87171',
  constructive: '#22D3A5',
  local: '#6366F1',
  meta: '#F5A524',
  utility: '#8A8A92',
}

export const FAMILY_LABELS: Record<SolverFamily, string> = {
  exact: 'Exact',
  constructive: 'Constructive',
  local: 'Local search',
  meta: 'Metaheuristic',
  utility: 'Baseline',
}

/** A colour for a solver with no family, so an unknown id still plots as something. */
export const UNKNOWN_FAMILY_COLOR = '#6B7280'

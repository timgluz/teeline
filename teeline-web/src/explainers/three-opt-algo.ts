// Pure simulation logic for the 3-opt interactive explainer.
// Rust-faithful model (src/tsp/three_opt.rs):
//   - each pass scans all O(n³) triples of edges (A,B),(C,D),(E,F) and keeps
//     the single best improving reconnection across the 7 non-identity cases
//   - cases 1–3 are segment reversals; cases 4–7 swap/reverse the two middle
//     segments (the moves 2-opt cannot express)
//   - repeats until no triple yields an improvement (local optimum)
// Edge sets, case numbering, tie-breaks (first best case per triple, first
// max-savings move overall) and the degenerate (i==0, k==n-1) skip all mirror
// the solver. Deterministic — no RNG, so Back/Step replay identically.

import { CITIES, N_CITIES, dist, tourLength } from './explainer-cities'
export { CITIES, N_CITIES, dist, tourLength }

export type Phase = 'idle' | 'candidate' | 'swap_applied' | 'local_optimum'

// ---------------------------------------------------------------
// The candidate move (Rust find_best_move + apply_3opt port)
// ---------------------------------------------------------------
export type CaseNo = 1 | 2 | 3 | 4 | 5 | 6 | 7

// Case labels (s1 = [B..C], s2 = [D..E]; rev = reversed):
//   1: rev(s1)        2: rev(s2)        3: rev(s1)+rev(s2)
//   4: s2+s1          5: s2+rev(s1)     6: rev(s2)+s1    7: rev(s2)+rev(s1)
export const CASE_LABELS: Record<CaseNo, string> = {
  1: 'rev s1',
  2: 'rev s2',
  3: 'rev s1 · rev s2',
  4: 'swap s2+s1',
  5: 's2 + rev s1',
  6: 'rev s2 + s1',
  7: 'rev s2 · rev s1',
}

export interface MoveEvent {
  i: number             // A = path[i]
  j: number             // C = path[j]
  k: number             // E = path[k]
  caseNo: CaseNo
  delta: number         // new tour length − old tour length (negative = improvement)
  removedEdges: [number, number][]  // [A,B], [C,D], [E,F] — the three cut edges
  addedEdges: [number, number][]    // the three reconnection edges for this case
}

export interface SimState {
  phase: Phase
  tour: number[]
  bestTour: number[]
  bestCost: number
  pass: number          // completed scans (one per applied swap + final scan)
  swaps: number         // applied 3-opt moves
  pending: MoveEvent | null
  lastMove: MoveEvent | null
  costHistory: number[] // best cost after each pass (sparkline)
  step: number
}

// New edge sets for the 7 reconnection cases (Rust reconnection_costs table).
function addedEdgesForCase(a: number, b: number, c: number, d: number, e: number, f: number, caseNo: CaseNo): [number, number][] {
  switch (caseNo) {
    case 1: return [[a, c], [b, d], [e, f]]
    case 2: return [[a, b], [c, e], [d, f]]
    case 3: return [[a, c], [b, e], [d, f]]
    case 4: return [[a, d], [e, b], [c, f]]
    case 5: return [[a, d], [e, c], [b, f]]
    case 6: return [[a, e], [d, b], [c, f]]
    case 7: return [[a, e], [d, c], [b, f]]
  }
}

// Rust find_best_move: scan all C(n,3) triples, return the globally best
// improving 3-opt move or null. Tie-breaks match the solver: per triple the
// lowest-index best case; overall the first move with the max savings.
export function scanBestMove(tour: number[]): MoveEvent | null {
  const n = tour.length
  let bestMove: { i: number; j: number; k: number; caseNo: CaseNo; delta: number } | null = null
  let bestSavings = 0

  for (let i = 0; i < n - 2; i++) {
    const a = tour[i]
    const b = tour[i + 1]
    for (let j = i + 1; j < n - 1; j++) {
      const c = tour[j]
      const dt = tour[j + 1]
      for (let k = j + 1; k < n; k++) {
        // Degenerate triple: the wrap-around edge makes F == A.
        if (i === 0 && k === n - 1) continue
        const e = tour[k]
        const f = tour[(k + 1) % n]

        const orig = dist(a, b) + dist(c, dt) + dist(e, f)
        const costs: number[] = [
          dist(a, c) + dist(b, dt) + dist(e, f), // 1
          dist(a, b) + dist(c, e) + dist(dt, f), // 2
          dist(a, c) + dist(b, e) + dist(dt, f), // 3
          dist(a, dt) + dist(e, b) + dist(c, f), // 4
          dist(a, dt) + dist(c, e) + dist(b, f), // 5
          dist(a, e) + dist(b, dt) + dist(c, f), // 6
          dist(a, e) + dist(c, dt) + dist(b, f), // 7
        ]

        // Best case for this triple: lowest-index minimum below orig.
        let bestCase = -1
        let bestCost = Infinity
        for (let ci = 0; ci < 7; ci++) {
          if (costs[ci] < orig && costs[ci] < bestCost) {
            bestCost = costs[ci]
            bestCase = ci
          }
        }
        if (bestCase >= 0) {
          const savings = orig - bestCost
          if (savings > bestSavings) {
            bestSavings = savings
            const caseNo = (bestCase + 1) as CaseNo
            if (caseNo < 1 || caseNo > 7) {
              throw new Error(`3-opt: invalid caseNo ${caseNo}`)
            }
            bestMove = { i, j, k, caseNo, delta: bestCost - orig }
          }
        }
      }
    }
  }

  if (!bestMove) return null
  const { i, j, k, caseNo, delta } = bestMove
  const a = tour[i]
  const b = tour[i + 1]
  const c = tour[j]
  const dt = tour[j + 1]
  const e = tour[k]
  const f = tour[(k + 1) % n]
  return {
    i, j, k, caseNo, delta,
    removedEdges: [[a, b], [c, dt], [e, f]],
    addedEdges: addedEdgesForCase(a, b, c, dt, e, f, caseNo),
  }
}

// Rust apply_3opt: cases 1–3 reverse segments, cases 4–7 swap/reverse the two
// middle segments into [i+1..=k].
export function apply3Opt(tour: number[], i: number, j: number, k: number, caseNo: CaseNo): number[] {
  const rev = (arr: number[]) => [...arr].reverse()
  const s1 = tour.slice(i + 1, j + 1) // [i+1..=j]
  const s2 = tour.slice(j + 1, k + 1) // [j+1..=k]
  const head = tour.slice(0, i + 1)
  const tail = tour.slice(k + 1)
  switch (caseNo) {
    case 1: return [...head, ...rev(s1), ...s2, ...tail]
    case 2: return [...head, ...s1, ...rev(s2), ...tail]
    case 3: return [...head, ...rev(s1), ...rev(s2), ...tail]
    case 4: return [...head, ...s2, ...s1, ...tail]
    case 5: return [...head, ...s2, ...rev(s1), ...tail]
    case 6: return [...head, ...rev(s2), ...s1, ...tail]
    case 7: return [...head, ...rev(s2), ...rev(s1), ...tail]
  }
}

// ---------------------------------------------------------------
// Scenarios (crafted tours, tuned offline — see three-opt.test.ts pins)
// ---------------------------------------------------------------
export interface Scenario {
  label: string
  desc: string
  tour: number[]
}

export const SCENARIOS: Record<string, Scenario> = {
  single_3opt: {
    label: 'Single 3-opt',
    desc: 'Two blocks are swapped — one 3-opt segment swap (case 4) puts them back',
    tour: [0, 3, 4, 1, 2, 5, 8, 9, 6, 7],
  },
  beyond_2opt: {
    label: 'Beyond 2-opt',
    desc: '2-opt calls this a local optimum — a 3-opt segment swap (case 6) still improves it',
    tour: [0, 7, 6, 5, 4, 3, 2, 1, 8, 9],
  },
  already_3optimal: {
    label: 'Already 3-optimal',
    desc: 'No improving 3-opt move — the search is stuck 0.1% above the global optimum',
    tour: [8, 2, 1, 0, 7, 6, 9, 5, 4, 3],
  },
  deep_sweep: {
    label: 'Deep sweep',
    desc: 'A tangled tour that needs several 3-opt passes to untangle',
    tour: [4, 0, 9, 3, 6, 1, 8, 2, 5, 7],
  },
}

export function makeInitState(tour: number[]): SimState {
  const cost = tourLength(tour)
  return {
    phase: 'idle',
    tour: [...tour],
    bestTour: [...tour],
    bestCost: cost,
    pass: 0,
    swaps: 0,
    pending: null,
    lastMove: null,
    costHistory: [cost],
    step: 0,
  }
}

// One Step press. Two-click model like the 2-opt/or-opt explainers:
//   idle | swap_applied → 'candidate'     (scan; show the best triple + case)
//   'candidate'          → 'swap_applied' (apply the pending reconnection)
// Either scan may instead end in 'local_optimum'. Pure — no mutation.
export function stepOnce(state: SimState): SimState {
  if (state.phase === 'local_optimum') return { ...state, step: state.step + 1 }

  // Second click — apply the pending move
  if (state.phase === 'candidate' && state.pending) {
    const m = state.pending
    const newTour = apply3Opt(state.tour, m.i, m.j, m.k, m.caseNo)
    const cost = tourLength(newTour)

    return {
      ...state,
      phase: 'swap_applied',
      tour: newTour,
      bestTour: cost < state.bestCost ? [...newTour] : state.bestTour,
      bestCost: Math.min(cost, state.bestCost),
      pass: state.pass + 1,
      swaps: state.swaps + 1,
      lastMove: m,
      costHistory: [...state.costHistory, cost],
      step: state.step + 1,
    }
  }

  // First click — scan for the best 3-opt move
  const move = scanBestMove(state.tour)

  if (!move) {
    return {
      ...state,
      phase: 'local_optimum',
      pass: state.pass + 1,
      costHistory: [...state.costHistory, state.bestCost],
      pending: null,
      step: state.step + 1,
    }
  }

  return {
    ...state,
    phase: 'candidate',
    pending: move,
    step: state.step + 1,
  }
}

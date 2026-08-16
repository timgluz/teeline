// Pure simulation logic for the Or-opt interactive explainer.
// Rust-faithful model (src/tsp/or_opt.rs):
//   - each pass scans ALL relocations of segments of length 1, 2 and 3
//     (Or-1/Or-2/Or-3) and applies the single best-improving move
//   - reversed insertions are also tried for Or-2/Or-3 (symmetric distances)
//   - repeats until no relocation improves the tour (local optimum)
// Move encoding follows the Rust solver exactly: (i, seg_len, j, reversed)
// with `i` = segment start, `j` = insert after original position j,
// the forbidden window {prev} ∪ [i, i+seg_len), the −1e-3 float threshold,
// and the drain/splice index adjustment in applyRelocation.
// Deterministic — no RNG, so Back/Step replay identically and tests can
// assert exact sequences. No DOM, fully testable.

export const CITIES: [number, number][] = [
  [150, 20],   // 0 — top
  [270, 70],   // 1 — top-right
  [260, 180],  // 2 — right
  [180, 280],  // 3 — bottom-right
  [120, 290],  // 4 — bottom
  [35, 220],   // 5 — bottom-left
  [25, 80],    // 6 — left
  [80, 25],    // 7 — top-left
  [155, 155],  // 8 — centre
  [90, 140],   // 9 — inner-left
]
export const N_CITIES = CITIES.length

export type Phase = 'idle' | 'candidate' | 'move_applied' | 'local_optimum'

export function dist(i: number, j: number): number {
  const dx = CITIES[i][0] - CITIES[j][0]
  const dy = CITIES[i][1] - CITIES[j][1]
  return Math.sqrt(dx * dx + dy * dy)
}

export function tourLength(tour: number[]): number {
  let d = 0
  for (let k = 0; k < tour.length; k++) {
    d += dist(tour[k], tour[(k + 1) % tour.length])
  }
  return d
}

// ---------------------------------------------------------------
// The candidate move (Rust find_best_move + apply_relocation port)
// ---------------------------------------------------------------
export interface MoveEvent {
  i: number             // segment start index (original tour indexing)
  segLen: 1 | 2 | 3
  j: number             // insert after original position j
  reversed: boolean     // reversed insertion (Or-2 / Or-3 only)
  delta: number         // new tour length − old tour length (negative = improvement)
  segCities: number[]   // cities in the segment, in tour order
  insertAfter: number   // city the segment lands after (path[j])
  insertBefore: number  // city the segment lands before (path[(j+1) % n])
  cutEdges: [number, number][]   // edges removed: prev→first, last→after, x→y
  pasteEdges: [number, number][] // edges added: prev→after, x→first, last→y (or reversed)
  improvingGaps: number[]        // original positions j with any improving move (scan ticks)
}

export interface SimState {
  phase: Phase
  tour: number[]
  bestTour: number[]
  bestCost: number
  pass: number          // completed scans (one per applied move + final local-optimum scan)
  moves: number         // applied relocations
  pending: MoveEvent | null // candidate awaiting its apply phase
  lastMove: MoveEvent | null // most recently applied move (for the flash)
  costHistory: number[] // best cost after each pass (sparkline)
  step: number
}

// Rust find_best_move: scan all Or-1/2/3 relocations, return the best
// improving move (delta < −1e-3) or null. Tie-break order matches Rust:
// seg_len asc → i asc → j asc → forward before reversed.
export function scanBestMove(tour: number[]): MoveEvent | null {
  const n = tour.length
  const improvingGaps = new Set<number>()
  // −1e-3 threshold + strictly-lower replacement, same as the solver's
  // best_delta (f32 rounding guard at large coordinate scales).
  let bestDelta = -1e-3
  let best: {
    i: number
    segLen: 1 | 2 | 3
    j: number
    reversed: boolean
    delta: number
  } | null = null

  for (const segLen of [1, 2, 3] as const) {
    if (n <= segLen + 1) continue
    for (let i = 0; i < n; i++) {
      if (i + segLen > n) continue // no wrap-around segments
      const prev = i === 0 ? n - 1 : i - 1
      const afterSeg = (i + segLen) % n
      const a = tour[prev]
      const firstSeg = tour[i]
      const lastSeg = tour[i + segLen - 1]
      const d = tour[afterSeg]
      const removeGain = dist(a, firstSeg) + dist(lastSeg, d) - dist(a, d)

      for (let j = 0; j < n; j++) {
        if (j === prev || (j >= i && j < i + segLen)) continue
        const x = tour[j]
        const y = tour[(j + 1) % n]
        const edgeXY = dist(x, y)

        const fwdDelta = -removeGain + dist(x, firstSeg) + dist(lastSeg, y) - edgeXY
        if (fwdDelta < bestDelta) {
          bestDelta = fwdDelta
          best = { i, segLen, j, reversed: false, delta: fwdDelta }
        }
        if (fwdDelta < -1e-3) improvingGaps.add(j)

        if (segLen > 1) {
          const revDelta = -removeGain + dist(x, lastSeg) + dist(firstSeg, y) - edgeXY
          if (revDelta < bestDelta) {
            bestDelta = revDelta
            best = { i, segLen, j, reversed: true, delta: revDelta }
          }
          if (revDelta < -1e-3) improvingGaps.add(j)
        }
      }
    }
  }

  if (!best) return null

  const { i, segLen, j, reversed, delta } = best
  const segCities = tour.slice(i, i + segLen)
  const prev = i === 0 ? n - 1 : i - 1
  const afterSeg = (i + segLen) % n
  const x = tour[j]
  const y = tour[(j + 1) % n]
  const cutEdges: [number, number][] = [
    [tour[prev], tour[i]],
    [tour[i + segLen - 1], tour[afterSeg]],
    [x, y],
  ]
  const pasteEdges: [number, number][] = reversed
    ? [
        [tour[prev], tour[afterSeg]],
        [x, tour[i + segLen - 1]],
        [tour[i], y],
      ]
    : [
        [tour[prev], tour[afterSeg]],
        [x, tour[i]],
        [tour[i + segLen - 1], y],
      ]

  return {
    i, segLen, j, reversed, delta,
    segCities,
    insertAfter: x,
    insertBefore: y,
    cutEdges,
    pasteEdges,
    improvingGaps: [...improvingGaps].sort((a, b) => a - b),
  }
}

// Rust apply_relocation: remove tour[i..i+segLen), insert it after position j
// (original indexing), reversing the segment when `reversed`.
export function applyRelocation(
  tour: number[],
  i: number,
  segLen: number,
  j: number,
  reversed: boolean,
): number[] {
  const seg = tour.slice(i, i + segLen)
  const rest = [...tour.slice(0, i), ...tour.slice(i + segLen)]
  const insertAt = j >= i + segLen ? j - segLen + 1 : j + 1
  const toInsert = reversed ? [...seg].reverse() : seg
  return [...rest.slice(0, insertAt), ...toInsert, ...rest.slice(insertAt)]
}

// ---------------------------------------------------------------
// Scenarios (crafted tours, tuned offline — see or-opt.test.ts pins)
// ---------------------------------------------------------------
export interface Scenario {
  label: string
  desc: string
  tour: number[]
}

export const SCENARIOS: Record<string, Scenario> = {
  single_segment: {
    label: 'Single segment',
    desc: 'The centre city is wedged in the wrong spot — one Or-1 move slides it out',
    tour: [0, 1, 2, 3, 4, 8, 5, 9, 6, 7],
  },
  triplet_move: {
    label: 'Triplet move',
    desc: 'A block of 3 cities got folded into the tour — the best move relocates the whole block (Or-3)',
    tour: [0, 1, 2, 8, 5, 4, 3, 9, 6, 7],
  },
  already_optimal: {
    label: 'Already optimal',
    desc: 'No improving Or-1/2/3 relocation exists — this tour is the global optimum',
    tour: [0, 1, 2, 3, 4, 5, 8, 9, 6, 7],
  },
  two_opt_stuck: {
    label: '2-opt stuck',
    desc: '2-opt calls this a local optimum — but Or-opt still finds an improving relocation',
    tour: [0, 7, 6, 5, 4, 3, 2, 1, 8, 9],
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
    moves: 0,
    pending: null,
    lastMove: null,
    costHistory: [cost],
    step: 0,
  }
}

// One Step press. Two-click model like the 2-opt explainer:
//   idle | move_applied → 'candidate'   (scan; show best move + faint gaps)
//   'candidate'          → 'move_applied' (apply the pending relocation)
// Either scan may instead end in 'local_optimum' when no move improves.
// Pure function of the input state — no mutation.
export function stepOnce(state: SimState): SimState {
  if (state.phase === 'local_optimum') return { ...state, step: state.step + 1 }

  // Second click — apply the pending move
  if (state.phase === 'candidate' && state.pending) {
    const m = state.pending
    const newTour = applyRelocation(state.tour, m.i, m.segLen, m.j, m.reversed)
    const cost = tourLength(newTour)

    return {
      ...state,
      phase: 'move_applied',
      tour: newTour,
      bestTour: cost < state.bestCost ? [...newTour] : state.bestTour,
      bestCost: Math.min(cost, state.bestCost),
      pass: state.pass + 1,
      moves: state.moves + 1,
      lastMove: m,
      costHistory: [...state.costHistory, cost],
      step: state.step + 1,
    }
  }

  // First click — scan for the best improving move
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

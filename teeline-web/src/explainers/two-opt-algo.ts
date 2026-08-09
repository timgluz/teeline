// Pure simulation logic for the 2-opt interactive explainer.
// Two-click step: first click scans and highlights the best candidate swap
// (tour stays as-is); second click applies the swap. No DOM, fully testable.

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

export type Phase = 'idle' | 'candidate' | 'swap_found' | 'local_optimum'

export interface SwapEvent {
  i: number
  j: number
  removed: [number, number][]
  added: [number, number][]
  delta: number
}

export interface SimState {
  phase: Phase
  tour: number[]
  bestTour: number[]
  bestCost: number
  pass: number
  totalSwaps: number
  lastSwap: SwapEvent | null
  costHistory: number[]
  step: number
}

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

// Predefined scenario tours
export const SCENARIOS: Record<string, { label: string; desc: string; tour: number[] }> = {
  single_crossing: {
    label: 'Single crossing',
    desc: 'One pair of edges cross — one swap fixes it',
    tour: [0, 1, 8, 7, 2, 3, 4, 5, 9, 6],
  },
  multiple_crossings: {
    label: 'Multiple crossings',
    desc: 'Several edge crossings — resolved over a few passes',
    tour: [0, 2, 8, 7, 1, 3, 5, 4, 9, 6],
  },
  two_optimal: {
    label: 'Already 2-optimal',
    desc: 'Local optimum — no improving 2-opt swap exists',
    tour: [0, 7, 6, 5, 4, 3, 2, 1, 8, 9],
  },
  bad_shuffle: {
    label: 'Bad shuffle',
    desc: 'Many crossings — needs several passes to untangle',
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
    totalSwaps: 0,
    lastSwap: null,
    costHistory: [cost],
    step: 0,
  }
}

// Scan all non-adjacent edge pairs; return the best negative-delta swap, or
// { bestDelta: 0, bestI: -1, bestJ: -1 } if no improving swap exists.
export function scanOnePass(state: SimState): {
  bestDelta: number
  bestI: number
  bestJ: number
} {
  const tour = state.tour
  const n = tour.length
  let bestDelta = 0
  let bestI = -1
  let bestJ = -1

  for (let i = 0; i < n; i++) {
    const ni = (i + 1) % n
    for (let j = i + 2; j < n; j++) {
      if (i === 0 && j === n - 1) continue // wrap-around adjacent
      const nj = (j + 1) % n
      const delta =
        dist(tour[i], tour[j]) +
        dist(tour[ni], tour[nj]) -
        dist(tour[i], tour[ni]) -
        dist(tour[j], tour[nj])
      if (delta < bestDelta) {
        bestDelta = delta
        bestI = i
        bestJ = j
      }
    }
  }
  return { bestDelta, bestI, bestJ }
}

// Reverse the segment [i+1 .. j] in-place on a copy.
export function applySwap(tour: number[], i: number, j: number): number[] {
  const next = [...tour]
  let left = i + 1
  let right = j
  while (left < right) {
    ;[next[left], next[right]] = [next[right], next[left]]
    left++
    right--
  }
  return next
}

// One step: if in candidate phase, apply the pending swap.
// Otherwise scan for the best improving swap and show it as a candidate.
export function stepOnce(state: SimState): SimState {
  if (state.phase === 'local_optimum')
    return { ...state, step: state.step + 1 }

  // Second click — apply the candidate swap
  if (state.phase === 'candidate' && state.lastSwap) {
    const { i, j, delta } = state.lastSwap
    const ni = (i + 1) % N_CITIES
    const nj = (j + 1) % N_CITIES
    const newTour = applySwap(state.tour, i, j)
    const removed: [number, number][] = [
      [state.tour[i], state.tour[ni]],
      [state.tour[j], state.tour[nj]],
    ]
    const added: [number, number][] = [
      [newTour[i], newTour[ni]],
      [newTour[j], newTour[nj]],
    ]
    const cost = tourLength(newTour)

    return {
      ...state,
      phase: 'swap_found',
      tour: newTour,
      pass: state.pass + 1,
      totalSwaps: state.totalSwaps + 1,
      lastSwap: { i, j, removed, added, delta },
      bestTour: cost < state.bestCost ? [...newTour] : state.bestTour,
      bestCost: Math.min(cost, state.bestCost),
      costHistory: [...state.costHistory, cost],
      step: state.step + 1,
    }
  }

  // First click — scan for best improvement
  const { bestDelta, bestI, bestJ } = scanOnePass(state)

  if (bestDelta < 0) {
    const ni = (bestI + 1) % N_CITIES
    const nj = (bestJ + 1) % N_CITIES
    // Record candidate swap but DON'T modify the tour
    const removed: [number, number][] = [
      [state.tour[bestI], state.tour[ni]],
      [state.tour[bestJ], state.tour[nj]],
    ]
    const newTour = applySwap(state.tour, bestI, bestJ)
    const added: [number, number][] = [
      [newTour[bestI], newTour[ni]],
      [newTour[bestJ], newTour[nj]],
    ]

    return {
      ...state,
      phase: 'candidate',
      lastSwap: { i: bestI, j: bestJ, removed, added, delta: bestDelta },
      step: state.step + 1,
    }
  }

  return {
    ...state,
    phase: 'local_optimum',
    pass: state.pass + 1,
    costHistory: [...state.costHistory, state.bestCost],
    lastSwap: null,
    step: state.step + 1,
  }
}

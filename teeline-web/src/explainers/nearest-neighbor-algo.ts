// Pure simulation logic for the Nearest Neighbor interactive explainer.
// One `stepOnce` call picks the closest unvisited city and adds it to the tour.
// Reuses the same 10-city set as the 2-opt explainer. No DOM, fully testable.

// Note: this explainer keeps its own OPEN-PATH tourLength (no wrap-around edge);
// only the layout and dist come from the shared module.
import { CITIES, N_CITIES, dist } from './explainer-cities'
export { CITIES, N_CITIES, dist }

export type EventMode = 'visited' | 'closing' | 'done'

export interface Candidate {
  city: number
  dist: number
}

export interface SimState {
  tour: number[]             // ordered path, grows from [start] to [start,...,start]
  unvisited: number[]        // cities not yet added to tour
  step: number
  done: boolean
  lastEvent: EventMode | null
  lastCity: number | null    // most recently visited city (or start on closing)
  lastDist: number           // distance of the most recent edge
  candidateDists: Candidate[] | null // distances from current city to all unvisited
}

export function tourLength(tour: number[]): number {
  let d = 0
  for (let k = 0; k < tour.length - 1; k++) {
    d += dist(tour[k], tour[k + 1])
  }
  return d
}

export const SCENARIOS: Record<string, { label: string; desc: string; startCity: number }> = {
  balanced: {
    label: 'Balanced',
    desc: 'Start from city 0 (top) — walks clockwise, decent result',
    startCity: 0,
  },
  worst_leg: {
    label: 'Worst leg',
    desc: 'Start from city 4 (bottom) — last visited city is far from start',
    startCity: 4,
  },
  centre_out: {
    label: 'Centre out',
    desc: 'Start from city 8 (centre) — builds outward in both directions',
    startCity: 8,
  },
}

export function makeInitState(startCity: number): SimState {
  const unvisited: number[] = []
  for (let i = 0; i < N_CITIES; i++) {
    if (i !== startCity) unvisited.push(i)
  }
  return {
    tour: [startCity],
    unvisited,
    step: 0,
    done: false,
    lastEvent: null,
    lastCity: null,
    lastDist: 0,
    candidateDists: null,
  }
}

export function stepOnce(state: SimState): SimState {
  if (state.done) return { ...state, step: state.step + 1 }

  const current = state.tour[state.tour.length - 1]

  if (state.unvisited.length === 0) {
    // Closing step — return to start
    const start = state.tour[0]
    const d = dist(current, start)
    return {
      ...state,
      tour: [...state.tour, start],
      step: state.step + 1,
      done: true,
      lastEvent: 'closing',
      lastCity: start,
      lastDist: d,
      candidateDists: null,
    }
  }

  // Find nearest unvisited
  const cands: Candidate[] = state.unvisited
    .map(c => ({ city: c, dist: dist(current, c) }))
    .sort((a, b) => a.dist - b.dist)

  const nearest = cands[0]

  return {
    ...state,
    tour: [...state.tour, nearest.city],
    unvisited: state.unvisited.filter(c => c !== nearest.city),
    step: state.step + 1,
    lastEvent: 'visited',
    lastCity: nearest.city,
    lastDist: nearest.dist,
    candidateDists: [...cands],
  }
}

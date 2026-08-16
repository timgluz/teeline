// Pure simulation logic for the Stochastic Hill Climbing interactive explainer.
// Rust-faithful model (src/tsp/stochastic_hill.rs + src/tsp/route.rs):
//   - the tour starts as a random shuffle of the 8 cities
//   - each epoch draws ONE random 2-opt candidate (random non-adjacent pair,
//     inclusive segment reversal) from the current tour
//   - the candidate is accepted ONLY if it beats the best-so-far cost, so the
//     current tour equals the best tour except right after a restart, when a
//     fresh random tour is shown until it either beats the best or the search
//     goes stale again
//   - when the number of consecutive rejections exceeds `patience`, the
//     current tour is shuffled again (the best tour survives)
// All randomness comes from a pure counter-based RNG (seed + counter) stored
// inside SimState, so Back/Step replay identically and tests can assert exact
// sequences. No DOM, fully testable.

// 8 cities in a rough ring (the 10-city layout minus the two inner points) —
// small enough that a full scenario run stays short, which suits an explainer
// whose job is to show the mechanics (random 2-opt, accept-if-better,
// restart-when-stale), not to solve the instance.
export const CITIES: [number, number][] = [
  [150, 20],   // 0 — top
  [270, 70],   // 1 — top-right
  [260, 180],  // 2 — right
  [180, 280],  // 3 — bottom-right
  [120, 290],  // 4 — bottom
  [35, 220],   // 5 — bottom-left
  [25, 80],    // 6 — left
  [80, 25],    // 7 — top-left
]
export const N_CITIES = CITIES.length

export type Phase = 'idle' | 'propose' | 'accepted' | 'rejected' | 'restart' | 'done'

// ---------------------------------------------------------------
// Seeded RNG — pure (seed, counter) → [0, 1), so it can be
// structuredClone'd as part of SimState and replay identically.
// ---------------------------------------------------------------
export interface RngState {
  seed: number
  counter: number
}

export function makeRng(seed: number): RngState {
  return { seed: seed >>> 0, counter: 0 }
}

export function nextRand(rng: RngState): { value: number; rng: RngState } {
  // mulberry32-style scramble of (seed + counter·φ), pure w.r.t. its inputs.
  let t = (rng.seed + Math.imul(rng.counter, 0x6d2b79f5)) >>> 0
  t = Math.imul(t ^ (t >>> 15), t | 1)
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
  return {
    value: ((t ^ (t >>> 14)) >>> 0) / 4294967296,
    rng: { seed: rng.seed, counter: rng.counter + 1 },
  }
}

// ---------------------------------------------------------------
// Geometry (same 10-city set as the 2-opt and NN explainers)
// ---------------------------------------------------------------
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
// Moves — Rust route.rs semantics
// ---------------------------------------------------------------
export function seededShuffle(rng: RngState): { tour: number[]; rng: RngState } {
  const tour = Array.from({ length: N_CITIES }, (_, i) => i)
  let r = rng
  for (let i = N_CITIES - 1; i > 0; i--) {
    const { value, rng: r2 } = nextRand(r)
    r = r2
    const j = Math.floor(value * (i + 1))
    ;[tour[i], tour[j]] = [tour[j], tour[i]]
  }
  return { tour, rng: r }
}

// Reverse the inclusive segment [i..=j] on a copy (Rust swap_cities — the
// "2-OPT keeps changes more stable" reversal).
export function reverseSegment(tour: number[], i: number, j: number): number[] {
  const next = [...tour]
  let left = i
  let right = j
  while (left < right) {
    ;[next[left], next[right]] = [next[right], next[left]]
    left++
    right--
  }
  return next
}

// Rust random_successor(): draw a random pair with positions > 1 apart (the
// wrap pair (0, n−1) is allowed), then reverse the inclusive segment [i..=j].
// Returns the candidate tour plus the removed/added edge pairs for display.
export function randomSuccessor(tour: number[], rng: RngState): {
  tour: number[]
  i: number
  j: number
  removed: [number, number][]
  added: [number, number][]
  rng: RngState
} {
  const n = tour.length
  let r = rng
  let i = 0
  let j = 0
  for (let tries = 0; tries < 10; tries++) {
    const a = nextRand(r)
    r = a.rng
    const b = nextRand(r)
    r = b.rng
    const lo = Math.floor(a.value * n)
    const hi = Math.floor(b.value * n)
    i = Math.min(lo, hi)
    j = Math.max(lo, hi)
    if (j - i > 1) break
  }
  const next = reverseSegment(tour, i, j)
  let removed: [number, number][]
  let added: [number, number][]
  if (i === 0 && j === n - 1) {
    // Full-tour reversal: both break points sit on the same wrap-around edge,
    // so report that single edge once (the general formula would duplicate it
    // and build self-loop "added" edges that never match a real tour edge).
    removed = [[tour[n - 1], tour[0]]]
    added = [[tour[n - 1], tour[0]]]
  } else {
    removed = [
      [tour[(i - 1 + n) % n], tour[i]],
      [tour[j], tour[(j + 1) % n]],
    ]
    added = [
      [tour[(i - 1 + n) % n], tour[j]],
      [tour[i], tour[(j + 1) % n]],
    ]
  }
  return { tour: next, i, j, removed, added, rng: r }
}

// ---------------------------------------------------------------
// Scenario + simulation state
// ---------------------------------------------------------------
export interface Scenario {
  label: string
  desc: string
  seed: number
  // Optional crafted starting tour (Rust's init_tour path — used by the
  // "quick convergence" scenario, whose premise is a *good* start). When
  // omitted the tour starts as a seeded random shuffle, like the solver's
  // default `shuffle` auto-seed.
  initTour?: number[]
  epochs: number
  patience: number
}

export interface CandidateEvent {
  i: number
  j: number
  removed: [number, number][]
  added: [number, number][]
  delta: number            // candidateCost − currentCost (visual Δ on screen)
  candidateTour: number[]
  candidateCost: number
  accepted: boolean        // candidateCost < bestCost (Rust: < best_distance)
  restart: boolean         // this rejection pushes nStale past patience
  restartTour: number[] | null // fresh random tour if restarting
}

export interface SimState {
  phase: Phase
  tour: number[]           // current tour (== bestTour except right after a restart)
  bestTour: number[]
  bestCost: number
  currentCost: number
  epoch: number            // completed candidate evaluations (verdicts)
  nStale: number           // consecutive rejections since last accept/restart
  restarts: number
  acceptedCount: number
  rejectedCount: number
  maxEpochs: number
  patience: number
  rng: RngState
  pending: CandidateEvent | null // drawn candidate awaiting its verdict phase
  costHistory: number[]    // best cost after each epoch (sparkline)
  step: number
}

// Seeds tuned so each scenario reliably behaves as its name suggests
// (see stochastic-hill.test.ts for the assertions that pin them):
//   quick  — seed 371 + a crafted start (the optimal perimeter with its head
//            segment reversed, [2,1,0,3,4,5,6,7], 1.36× optimal): reaches the
//            optimum (827.8) at epoch 15, ZERO restarts
//   rugged — seed 917: 9 restarts, ends at 1.65× the optimum
//   needle — seed 1548: the optimum is only found at epoch 37, after 4
//            restarts have wandered through mediocre local optima
export const SCENARIOS: Record<string, Scenario> = {
  quick_convergence: {
    label: 'Quick convergence',
    desc: 'A good starting tour — a few improving moves reach the optimum, no restarts',
    seed: 371,
    initTour: [2, 1, 0, 3, 4, 5, 6, 7],
    epochs: 30,
    patience: 25,
  },
  rugged_landscape: {
    label: 'Rugged landscape',
    desc: 'Many mediocre local optima — the search keeps stalling and restarting',
    seed: 917,
    epochs: 50,
    patience: 5,
  },
  needle_in_haystack: {
    label: 'Needle in haystack',
    desc: 'One good optimum hidden among many — only a rare restart finds it',
    seed: 1548,
    epochs: 60,
    patience: 6,
  },
}

export function makeInitState(scenario: Scenario): SimState {
  const rng = makeRng(scenario.seed)
  let rng2 = rng
  let tour: number[]
  if (scenario.initTour) {
    tour = [...scenario.initTour]
  } else {
    const s = seededShuffle(rng2)
    tour = s.tour
    rng2 = s.rng
  }
  const cost = tourLength(tour)
  return {
    phase: 'idle',
    tour,
    bestTour: [...tour],
    bestCost: cost,
    currentCost: cost,
    epoch: 0,
    nStale: 0,
    restarts: 0,
    acceptedCount: 0,
    rejectedCount: 0,
    maxEpochs: scenario.epochs,
    patience: scenario.patience,
    rng: rng2,
    pending: null,
    costHistory: [cost],
    step: 0,
  }
}

// One Step press. Two phases per step (like 2-opt's candidate/apply):
//   idle | verdict-phase  → 'propose'   (draw a random candidate, tour unchanged)
//   'propose'             → verdict phase ('accepted' | 'rejected' | 'restart',
//                            or 'done' when the epoch cap is hit)
// Everything is a pure function of the input state — no mutation, fully
// deterministic for a given seed.
export function stepOnce(state: SimState): SimState {
  if (state.phase === 'done') return { ...state, step: state.step + 1 }

  // Phase 2 — apply the pending verdict
  if (state.phase === 'propose' && state.pending) {
    const p = state.pending
    const epoch = state.epoch + 1
    const accepted = p.accepted
    const restart = p.restart

    let phase: Phase
    let tour: number[]
    if (accepted) {
      phase = epoch >= state.maxEpochs ? 'done' : 'accepted'
      tour = p.candidateTour
    } else if (restart && p.restartTour) {
      phase = epoch >= state.maxEpochs ? 'done' : 'restart'
      tour = p.restartTour
    } else {
      phase = epoch >= state.maxEpochs ? 'done' : 'rejected'
      tour = state.tour
    }
    const bestCost = accepted ? p.candidateCost : state.bestCost

    return {
      ...state,
      phase,
      tour,
      bestTour: accepted ? [...p.candidateTour] : state.bestTour,
      bestCost,
      currentCost: tourLength(tour),
      epoch,
      nStale: accepted || restart ? 0 : state.nStale + 1,
      restarts: state.restarts + (restart ? 1 : 0),
      acceptedCount: state.acceptedCount + (accepted ? 1 : 0),
      rejectedCount: state.rejectedCount + (!accepted && !restart ? 1 : 0),
      costHistory: [...state.costHistory, bestCost],
      step: state.step + 1,
    }
  }

  // Phase 1 — draw the next random candidate
  if (state.epoch >= state.maxEpochs) {
    return { ...state, phase: 'done', step: state.step + 1 }
  }
  const { tour: candidateTour, i, j, removed, added, rng } = randomSuccessor(state.tour, state.rng)
  const candidateCost = tourLength(candidateTour)
  const delta = candidateCost - state.currentCost
  const accepted = candidateCost < state.bestCost
  const staleAfter = state.nStale + 1
  const restart = !accepted && staleAfter > state.patience

  let rng2 = rng
  let restartTour: number[] | null = null
  if (restart) {
    const s = seededShuffle(rng2)
    restartTour = s.tour
    rng2 = s.rng
  }

  return {
    ...state,
    phase: 'propose',
    rng: rng2,
    pending: {
      i, j, removed, added, delta,
      candidateTour,
      candidateCost,
      accepted,
      restart,
      restartTour,
    },
    step: state.step + 1,
  }
}

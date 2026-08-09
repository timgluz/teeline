// Pure simulation logic for the Ant Colony Optimization explainer.
// One "step" builds one ant's tour (like CS processes one nest per cuckoo event);
// after numAnts ants finish, the between-epoch phase evaporates + deposits pheromone
// and advances the epoch. Mirrors the Rust `ant_colony.rs` Ant System loop
// (construct -> best-update -> evaporate -> deposit).

export const CITIES: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]
export const N_CITIES = CITIES.length

export type Phase = 'building' | 'depositing'
export type EventMode = 'ant-built' | 'improved' | 'evaporated' | 'deposited'

export type SimState = {
  phase: Phase
  antIdx: number            // 0..numAnts — which ant is building now (building phase only)
  epoch: number
  pheromone: number[][]     // symmetric NxN; pheromone[i][j] = pheromone[j][i]
  eta: number[][]           // precomputed (1/dist)^beta; symmetric
  alpha: number
  beta: number
  evaporationRate: number
  numAnts: number
  tau0: number
  tauMin: number
  bestTour: number[]
  bestCost: number
  lastTours: number[][]     // tours built this epoch (cleared after deposit)
  lastEvent: EventMode | null
  lastTour: number[] | null // most recently built tour (for canvas highlight)
  costHistory: number[]     // epoch-best costs
  step: number
}

export function dist(i: number, j: number): number {
  const [x1, y1] = CITIES[i]
  const [x2, y2] = CITIES[j]
  return Math.hypot(x2 - x1, y2 - y1)
}

function averageDistance(): number {
  let sum = 0, count = 0
  for (let i = 0; i < N_CITIES; i++)
    for (let j = i + 1; j < N_CITIES; j++)
      { sum += dist(i, j); count++ }
  return sum / count
}

function computeEta(beta: number): number[][] {
  const eta: number[][] = Array.from({ length: N_CITIES }, () => new Array(N_CITIES))
  for (let i = 0; i < N_CITIES; i++) {
    eta[i][i] = 0
    for (let j = i + 1; j < N_CITIES; j++) {
      const v = (1 / Math.max(dist(i, j), 1e-6)) ** beta
      eta[i][j] = v
      eta[j][i] = v
    }
  }
  return eta
}

export function tourLength(tour: number[]): number {
  if (tour.length <= 1) return 0
  let d = 0
  for (let i = 0; i < tour.length; i++) {
    d += dist(tour[i], tour[(i + 1) % tour.length])
  }
  return d
}

// Roulette-wheel construction: starting from a random city, each next city is
// chosen probabilistically weighted by pheromone^alpha * eta^beta among unvisited
// cities. Falls back to first-unvisited if all weights are zero (matching the
// Rust solver's graceful underflow guard).
function buildTour(pheromone: number[][], eta: number[][], alpha: number, _beta: number): number[] {
  const unvisited = new Set(Array.from({ length: N_CITIES }, (_, i) => i))
  const start = Math.floor(Math.random() * N_CITIES)
  unvisited.delete(start)
  const tour: number[] = [start]

  while (unvisited.size > 0) {
    const current = tour[tour.length - 1]
    const candidates = Array.from(unvisited)
    const weights = candidates.map(j => {
      const p = Math.max(pheromone[current][j], 1e-30)
      const e = eta[current][j]
      return (p ** alpha) * e
    })
    const total = weights.reduce((s, w) => s + w, 0)

    let next = candidates[candidates.length - 1]
    if (total <= 0 || !isFinite(total)) {
      // underflow / overflow → first unvisited (next already defaults above)
    } else {
      let r = Math.random() * total
      for (let k = 0; k < candidates.length; k++) {
        r -= weights[k]
        if (r <= 0) { next = candidates[k]; break }
      }
    }
    tour.push(next)
    unvisited.delete(next)
  }
  return tour
}

export function maxPheromone(s: SimState): number {
  let m = 0
  for (let i = 0; i < N_CITIES; i++)
    for (let j = i + 1; j < N_CITIES; j++)
      if (s.pheromone[i][j] > m) m = s.pheromone[i][j]
  return m
}

export function makeInitState(
  alpha: number, beta: number, evaporationRate: number, numAnts: number,
): SimState {
  const avgDist = averageDistance()
  const tau0 = numAnts / avgDist
  const pheromone: number[][] = Array.from({ length: N_CITIES }, () => new Array(N_CITIES).fill(tau0))
  for (let i = 0; i < N_CITIES; i++) pheromone[i][i] = 0
  const eta = computeEta(beta)

  // seed with identity tour — a clean clockwise perimeter order that
  // the ACO colony will refine from (random can produce crossings)
  const initTour = Array.from({ length: N_CITIES }, (_, i) => i)
  const initCost = tourLength(initTour)

  return {
    phase: 'building',
    antIdx: 0,
    epoch: 0,
    pheromone,
    eta,
    alpha,
    beta,
    evaporationRate,
    numAnts,
    tau0,
    tauMin: tau0 * 1e-4,
    bestTour: initTour.slice(),
    bestCost: initCost,
    lastTours: [],
    lastEvent: null,
    lastTour: null,
    costHistory: [],
    step: 0,
  }
}

export function stepOnce(s: SimState): SimState {
  if (s.phase === 'building') {
    // Build one ant's tour
    const tour = buildTour(s.pheromone, s.eta, s.alpha, s.beta)
    const cost = tourLength(tour)
    const newLastTours = [...s.lastTours, tour]
    const improved = cost < s.bestCost
    const nextAnt = s.antIdx + 1

    // Last ant of this epoch → transition to depositing
    if (nextAnt >= s.numAnts) {
      return {
        ...s,
        phase: 'depositing',
        antIdx: 0,
        lastTours: newLastTours,
        lastEvent: improved ? 'improved' : 'ant-built',
        lastTour: tour,
        bestTour: improved ? tour.slice() : s.bestTour,
        bestCost: improved ? cost : s.bestCost,
        step: s.step + 1,
      }
    }

    return {
      ...s,
      antIdx: nextAnt,
      lastTours: newLastTours,
      lastEvent: improved ? 'improved' : 'ant-built',
      lastTour: tour,
      bestTour: improved ? tour.slice() : s.bestTour,
      bestCost: improved ? cost : s.bestCost,
      step: s.step + 1,
    }
  }

  // Phase === 'depositing': evaporate + deposit, then advance epoch
  const rate = s.evaporationRate
  const newPheromone = s.pheromone.map(row => row.map(p => {
    const evap = p * (1 - rate)
    return Math.max(evap, s.tauMin)
  }))

  // Deposit from each ant's tour
  for (const tour of s.lastTours) {
    const deposit = 1 / tourLength(tour)
    for (let k = 0; k < tour.length; k++) {
      const u = tour[k], v = tour[(k + 1) % tour.length]
      newPheromone[u][v] += deposit
      newPheromone[v][u] += deposit
    }
  }

  const newEpoch = s.epoch + 1

  return {
    ...s,
    phase: 'building',
    antIdx: 0,
    epoch: newEpoch,
    pheromone: newPheromone,
    lastTours: [],
    lastEvent: 'deposited',
    lastTour: null,
    costHistory: [...s.costHistory.slice(-59), s.bestCost],
    step: s.step + 1,
  }
}

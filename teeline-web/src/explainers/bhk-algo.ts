// Pure simulation logic for the Bellman-Held-Karp explainer.
//
// Rust-faithful model (src/tsp/bellman_karp.rs) with a cleaner convention:
// the start city is city 0 and the DP lives on the OTHER n-1 cities (1..n-1),
// indexed by bitmask subsets. dp[mask][i] = minimum cost of a path that starts
// at city 0, visits exactly the cities in `mask`, and ends at city i:
//   base:  dp[{i}][i] = d(0, i)
//   step:  dp[mask][i] = min over j in mask\{i} of dp[mask\{i}][j] + d(j, i)
// Answer: min over i of dp[full][i] + d(i, 0).
// The predecessor choice is recorded per cell so the optimal route can be
// read back and each cell can explain its computation.
//
// n = 6 per scenario → the table is 5 rows × 32 subsets, easy to scan.
// Deterministic — no RNG; SimState holds only plain data (cloneable for Back).

import { makeDm } from './explainer-cities'
export { makeDm }

export type Phase = 'forward' | 'readback' | 'done'

export interface Scenario {
  label: string
  desc: string
  cities: [number, number][]
}

export interface SimState {
  phase: Phase
  cities: [number, number][]
  n: number            // total cities (city 0 = start)
  m: number            // n-1 (DP cities 1..n-1)
  dm: number[][]       // distance matrix (plain numbers)
  // DP table: rows = end city index (0..m-1 ↔ city i+1), cols = mask
  table: number[][]    // ∞ until computed
  pred: number[][]     // predecessor city (index into rows) per computed cell, -1 if none
  // fill order (deterministic): masks by popcount ascending, then value
  fillOrder: { mask: number; row: number }[]
  fillPtr: number
  // optimal route (computed when forward completes)
  optCost: number | null
  route: number[] | null       // city ids, [0, c1, ..., cn-1]
  readback: number[]           // city ids revealed so far (from the end backwards)
  readbackPtr: number
  lastEvent: string | null
  step: number
}

export const SCENARIOS: Record<string, Scenario> = {
  grid_6: {
    label: '6-city grid',
    desc: 'The familiar compact grid — a clean DP table to scan',
    cities: [[60, 60], [240, 60], [240, 240], [60, 240], [150, 60], [150, 240]],
  },
  circle_6: {
    label: 'Circle',
    desc: 'Six cities on a circle — the optimal route is the perimeter',
    cities: [[270, 150], [210, 254], [90, 254], [30, 150], [90, 46], [210, 46]],
  },
  clusters_6: {
    label: 'Two clusters',
    desc: 'Two tight triples far apart — the DP must bridge them twice',
    cities: [[60, 60], [90, 60], [60, 90], [240, 240], [210, 240], [240, 210]],
  },
}

const INF = Infinity

export function popcount(x: number): number {
  let c = 0
  while (x > 0) {
    x &= x - 1
    c++
  }
  return c
}

// Deterministic fill order: subset size ascending (starting at size 2 — the
// size-1 base cells are pre-filled in makeInitState), then mask value, then row.
export function computeFillOrder(m: number): { mask: number; row: number }[] {
  const order: { mask: number; row: number }[] = []
  const full = (1 << m) - 1
  for (let size = 2; size <= m; size++) {
    for (let mask = 1; mask <= full; mask++) {
      if (popcount(mask) !== size) continue
      for (let row = 0; row < m; row++) {
        if (mask & (1 << row)) order.push({ mask, row })
      }
    }
  }
  return order
}

export function makeInitState(scenario: Scenario): SimState {
  const cities = scenario.cities
  const n = cities.length
  const m = n - 1
  const dm = makeDm(cities)
  const size = 1 << m
  const table: number[][] = Array.from({ length: m }, () => new Array(size).fill(INF))
  const pred: number[][] = Array.from({ length: m }, () => new Array(size).fill(-1))

  // base cells: dp[{i}][i] = d(0, city i+1)
  for (let row = 0; row < m; row++) {
    table[row][1 << row] = dm[0][row + 1]
  }

  return {
    phase: 'forward',
    cities,
    n,
    m,
    dm,
    table,
    pred,
    fillOrder: computeFillOrder(m),
    fillPtr: 0,
    optCost: null,
    route: null,
    readback: [],
    readbackPtr: 0,
    lastEvent: null,
    step: 0,
  }
}

// Compute one DP cell (mask, row) from its predecessors; record the argmin.
function fillCell(state: SimState, mask: number, row: number): { value: number; via: number } {
  const rest = mask & ~(1 << row)
  const city = row + 1
  let best = INF
  let via = -1
  for (let j = 0; j < state.m; j++) {
    if ((rest & (1 << j)) === 0) continue
    const v = state.table[j][rest] + state.dm[j + 1][city]
    if (v < best) {
      best = v
      via = j
    }
  }
  return { value: best, via }
}

// The optimal route: end city e = argmin_i table[i][full] + d(i+1, 0),
// then read predecessors back to the start.
export function computeRoute(state: SimState): { cost: number; route: number[] } {
  const full = (1 << state.m) - 1
  let best = INF
  let end = -1
  for (let i = 0; i < state.m; i++) {
    const v = state.table[i][full] + state.dm[i + 1][0]
    if (v < best) {
      best = v
      end = i
    }
  }
  if (end < 0) return { cost: best, route: [0] } // no complete tour found
  const routeRev = [end + 1]
  let cur = end
  let mask = full
  while (mask !== 0) {
    const via = state.pred[cur][mask]
    if (via < 0) break
    mask &= ~(1 << cur)
    cur = via
    if (mask !== 0) routeRev.push(via + 1)
  }
  const route = [0, ...routeRev.reverse()]
  return { cost: best, route }
}

export function stepOnce(state: SimState): SimState {
  if (state.phase === 'done') return { ...state, step: state.step + 1 }

  // ----- readback: reveal one city of the optimal route per step -----
  if (state.phase === 'readback') {
    if (state.readbackPtr >= (state.route?.length ?? 0) - 1) {
      return { ...state, phase: 'done', lastEvent: `Done — optimal tour ${state.route!.join('→')} = ${state.optCost!.toFixed(1)}`, step: state.step + 1 }
    }
    // the end city is seeded; each step reveals the next city back toward the start
    const city = state.route![state.route!.length - 1 - state.readbackPtr]
    return {
      ...state,
      readback: [...state.readback, city],
      readbackPtr: state.readbackPtr + 1,
      lastEvent: `Read-back — optimal tour passes ${city} next (${state.readbackPtr + 1}/${state.n - 1} revealed)`,
      step: state.step + 1,
    }
  }

  // ----- forward: fill the next cell -----
  if (state.fillPtr >= state.fillOrder.length) {
    // forward complete → compute the optimal route, start read-back
    const { cost, route } = computeRoute(state)
    return {
      ...state,
      phase: 'readback',
      optCost: cost,
      route,
      readback: [route[route.length - 1]],
      readbackPtr: 1,
      lastEvent: `DP complete — optimal cost ${cost.toFixed(1)}. Step to read the route back`,
      step: state.step + 1,
    }
  }

  const { mask, row } = state.fillOrder[state.fillPtr]
  const { value, via } = fillCell(state, mask, row)

  const table = state.table.map((r, ri) => (ri === row ? [...r] : r))
  const pred = state.pred.map((r, ri) => (ri === row ? [...r] : r))
  table[row][mask] = value
  pred[row][mask] = via

  const sizeLabel = mask.toString(2).padStart(state.m, '0')
  const event = value === INF
    ? `Cell (${sizeLabel}, end ${row + 1}) — unreachable`
    : `dp[${sizeLabel}][${row + 1}] = ${value.toFixed(1)} via city ${via + 1}`

  return {
    ...state,
    table,
    pred,
    fillPtr: state.fillPtr + 1,
    lastEvent: event,
    step: state.step + 1,
  }
}

// Pure simulation logic for the Christofides explainer.
//
// Rust-faithful (src/tsp/christofides.rs), six deterministic steps:
//   1. Prim's MST       — min-key vertex first, tie-break by lowest index,
//                         strictly-less key updates
//   2. Odd-degree nodes — handshaking lemma: the count is always even
//   3. Greedy min-weight perfect matching — all C(k,2) pairs sorted
//                         shortest-first (stable, matching Rust's sort_by),
//                         match greedily
//   4. Multigraph union (MST ∪ matching) — all degrees even ⇒ Eulerian
//   5. Hierholzer's circuit — iterative, pops the last half-edge and
//                         swap_removes the reverse (parallel edges allowed)
//   6. Shortcut — keep the first occurrence of each city
//
// Teaches the 1.5× proof:
//   MST ≤ OPT · true-minimum matching ≤ OPT/2 · Euler = MST + matching
//   · shortcut ≤ Euler (triangle inequality). The solver (and this demo) use
//   a GREEDY matching, which can exceed the OPT/2 theory line — the panel
//   shows both the observed greedy cost and the theory line, honestly.
//
// Each instance is a small city layout (≤ 10 cities) so the exact OPT can be
// brute-forced for the ratio meter. No RNG — fully deterministic.

import { CITIES_10, makeDist } from './explainer-cities'

export interface Instance {
  label: string
  desc: string
  cities: [number, number][]
}

export type Phase = 'mst' | 'odd' | 'matching' | 'euler' | 'shortcut' | 'done'

export const PHASES: Phase[] = ['mst', 'odd', 'matching', 'euler', 'shortcut', 'done']

export interface SimState {
  phase: Phase
  cities: [number, number][]
  n: number
  // precomputed pipeline (deterministic per instance) — the distance closure
  // is deliberately NOT stored so the state stays structuredClone-able (Back)
  mstEdges: [number, number][]
  odd: number[]
  matchingEdges: [number, number][]
  eulerCircuit: number[]
  shortcutTour: number[]
  mstCost: number
  matchingCost: number
  eulerCost: number
  tourCost: number
  opt: number
  ratio: number
  nnCost: number       // greedy nearest-neighbor tour (comparison)
  approx2Cost: number  // doubled-MST tour (comparison)
  // progressive state
  mstRevealed: number      // MST edges revealed so far
  matchingRevealed: number // matching pairs revealed so far
  walkerPos: number        // index into eulerCircuit (euler phase)
  shortcutStep: number     // index into eulerCircuit (shortcut phase)
  kept: number[]           // shortcut: fresh cities so far (in walk order)
  skipped: number[]        // shortcut: circuit positions that were repeats
  step: number
}

// ---------------------------------------------------------------
// Step 1 — Prim's MST (port of prim_mst)
// ---------------------------------------------------------------
export function primMst(n: number, dist: (i: number, j: number) => number): [number, number][] {
  const inMst = new Array<boolean>(n).fill(false)
  const key = new Array<number>(n).fill(Infinity)
  const parent = new Array<number>(n).fill(-1)
  key[0] = 0
  const edges: [number, number][] = []

  for (let iter = 0; iter < n; iter++) {
    // min-key vertex not yet in MST; ties → lowest index (min_by over 0..n)
    let u = -1
    let bestKey = Infinity
    for (let i = 0; i < n; i++) {
      if (!inMst[i] && key[i] < bestKey) {
        bestKey = key[i]
        u = i
      }
    }
    inMst[u] = true
    if (parent[u] !== -1) edges.push([u, parent[u]] as [number, number])

    for (let v = 0; v < n; v++) {
      if (!inMst[v]) {
        const d = dist(u, v)
        if (d < key[v]) {
          key[v] = d
          parent[v] = u
        }
      }
    }
  }
  return edges
}

// ---------------------------------------------------------------
// Step 2 — odd-degree vertices (port of odd_degree_nodes)
// ---------------------------------------------------------------
export function oddDegreeNodes(mstEdges: [number, number][], n: number): number[] {
  const degree = new Array<number>(n).fill(0)
  for (const [u, v] of mstEdges) {
    degree[u]++
    degree[v]++
  }
  return degree.map((d, i) => (d % 2 === 1 ? i : -1)).filter((i) => i >= 0)
}

// ---------------------------------------------------------------
// Step 3 — greedy min-weight perfect matching (port of greedy_matching)
// ---------------------------------------------------------------
export function greedyMatching(
  odd: number[],
  dist: (i: number, j: number) => number,
  n: number,
): [number, number][] {
  const pairs: { d: number; u: number; v: number }[] = []
  for (let i = 0; i < odd.length; i++) {
    for (let j = i + 1; j < odd.length; j++) {
      pairs.push({ d: dist(odd[i], odd[j]), u: odd[i], v: odd[j] })
    }
  }
  // stable sort by weight (Array.prototype.sort is stable — matches Rust sort_by)
  pairs.sort((a, b) => a.d - b.d)

  const matched = new Array<boolean>(n).fill(false)
  const result: [number, number][] = []
  for (const { u, v } of pairs) {
    if (!matched[u] && !matched[v]) {
      matched[u] = true
      matched[v] = true
      result.push([u, v] as [number, number])
    }
  }
  return result
}

// ---------------------------------------------------------------
// Step 4 — multigraph adjacency (port of build_multigraph)
// ---------------------------------------------------------------
export function buildMultigraph(
  n: number,
  mstEdges: [number, number][],
  matching: [number, number][],
): number[][] {
  const adj: number[][] = Array.from({ length: n }, () => [])
  for (const [u, v] of [...mstEdges, ...matching]) {
    adj[u].push(v)
    adj[v].push(u)
  }
  return adj
}

// ---------------------------------------------------------------
// Step 5 — Hierholzer's Eulerian circuit (port of hierholzer)
// ---------------------------------------------------------------
export function hierholzer(adjIn: number[][], start: number): number[] {
  // copy — the algorithm consumes the adjacency
  const adj = adjIn.map((row) => [...row])
  const stack = [start]
  const circuit: number[] = []

  while (stack.length > 0) {
    const v = stack[stack.length - 1]
    if (adj[v].length > 0) {
      const u = adj[v].pop()!
      // swap_remove the reverse half-edge, exactly like Rust's
      // `if let Some(pos) = adj[u].iter().position(...)` guard.
      const pos = adj[u].indexOf(v)
      if (pos >= 0) {
        adj[u][pos] = adj[u][adj[u].length - 1]
        adj[u].pop()
      }
      stack.push(u)
    } else {
      circuit.push(stack.pop()!)
    }
  }
  return circuit.reverse()
}

// ---------------------------------------------------------------
// Step 6 — shortcut to a Hamiltonian tour (port of shortcut)
// ---------------------------------------------------------------
export function shortcut(circuit: number[], n: number): number[] {
  const seen = new Array<boolean>(n).fill(false)
  const tour: number[] = []
  for (const v of circuit) {
    if (!seen[v]) {
      seen[v] = true
      tour.push(v)
    }
  }
  return tour
}

// ---------------------------------------------------------------
// Full pipeline (like the Rust solve())
// ---------------------------------------------------------------
export function christofidesPipeline(
  n: number,
  dist: (i: number, j: number) => number,
): {
  mstEdges: [number, number][]
  odd: number[]
  matchingEdges: [number, number][]
  eulerCircuit: number[]
  shortcutTour: number[]
} {
  const mstEdges = primMst(n, dist)
  const odd = oddDegreeNodes(mstEdges, n)
  const matchingEdges = greedyMatching(odd, dist, n)
  const adj = buildMultigraph(n, mstEdges, matchingEdges)
  const eulerCircuit = hierholzer(adj, 0)
  const shortcutTour = shortcut(eulerCircuit, n)
  return { mstEdges, odd, matchingEdges, eulerCircuit, shortcutTour }
}

// ---------------------------------------------------------------
// Comparisons — the 2× cousin (doubled MST) and greedy NN
// ---------------------------------------------------------------
export function doubledMstTourCost(n: number, dist: (i: number, j: number) => number): number {
  const mstEdges = primMst(n, dist)
  // double every MST edge → all degrees even → Eulerian
  const adj: number[][] = Array.from({ length: n }, () => [])
  for (const [u, v] of mstEdges) {
    adj[u].push(v, v)
    adj[v].push(u, u)
  }
  const circuit = hierholzer(adj, 0)
  const tour = shortcut(circuit, n)
  return closedTourCost(tour, dist)
}

export function nearestNeighborTourCost(n: number, dist: (i: number, j: number) => number): number {
  const visited = new Array<boolean>(n).fill(false)
  const tour: number[] = []
  let cur = 0
  visited[cur] = true
  tour.push(cur)
  for (let t = 1; t < n; t++) {
    let best = -1
    let bestD = Infinity
    for (let v = 0; v < n; v++) {
      if (!visited[v]) {
        const d = dist(cur, v)
        if (d < bestD) {
          bestD = d
          best = v
        }
      }
    }
    visited[best] = true
    tour.push(best)
    cur = best
  }
  return closedTourCost(tour, dist)
}

export function closedTourCost(tour: number[], dist: (i: number, j: number) => number): number {
  let d = 0
  for (let k = 0; k < tour.length; k++) {
    d += dist(tour[k], tour[(k + 1) % tour.length])
  }
  return d
}

// ---------------------------------------------------------------
// Exact optimum (brute force over (n-1)! permutations, start fixed at 0)
// ---------------------------------------------------------------
export function bruteForceOpt(n: number, dist: (i: number, j: number) => number): number {
  let best = Infinity
  const rest = Array.from({ length: n - 1 }, (_, i) => i + 1)
  const used = new Array<boolean>(rest.length).fill(false)
  const order: number[] = [0]
  const search = () => {
    if (order.length === n) {
      const d = closedTourCost(order, dist)
      if (d < best) best = d
      return
    }
    for (let k = 0; k < rest.length; k++) {
      if (used[k]) continue
      used[k] = true
      order.push(rest[k])
      search()
      order.pop()
      used[k] = false
    }
  }
  search()
  return best
}

// ---------------------------------------------------------------
// Scenarios (city layouts, tuned offline — see christofides.test.ts pins)
// ---------------------------------------------------------------
export const SCENARIOS: Record<string, Instance> = {
  balanced: {
    label: 'Balanced',
    desc: 'The classic 10-city ring — a typical 1.11× result',
    cities: CITIES_10,
  },
  near_optimal: {
    label: 'Near-optimal',
    desc: 'Cities on a circle — Christofides finds the exact optimum (1.00×)',
    cities: [
      [280, 150], [255, 226], [190, 274], [110, 274], [45, 226],
      [20, 150], [45, 74], [110, 26], [190, 26], [255, 74],
    ],
  },
  matching_heavy: {
    label: 'Matching-heavy',
    desc: 'Two tight clusters joined by a bridge — the odd-vertex matching is half the tour cost',
    cities: [
      [57, 69], [77, 80], [65, 93], [93, 71], [79, 80],
      [247, 244], [230, 240], [238, 244], [224, 215], [246, 211],
    ],
  },
  clustered: {
    label: 'Clustered',
    desc: 'Three clusters — the matching edges bridge between them (the doubled-MST cousin loses here)',
    cities: [
      [69, 65], [90, 62], [84, 60],
      [202, 79], [216, 79], [196, 101],
      [149, 215], [152, 239], [159, 220], [139, 215],
    ],
  },
  worst_case: {
    label: 'Worst case',
    desc: 'A layout where the ratio stretches to 1.39× — the 1.5× bound is real, not hand-wavy',
    cities: [
      [155, 192], [60, 131], [170, 148], [187, 60], [148, 139],
      [105, 224], [108, 20], [52, 220], [148, 262], [165, 174],
    ],
  },
}

export function makeInitState(instance: Instance): SimState {
  const cities = instance.cities
  const n = cities.length
  const dist = makeDist(cities)
  const { mstEdges, odd, matchingEdges, eulerCircuit, shortcutTour } = christofidesPipeline(n, dist)

  const mstCost = mstEdges.reduce((s, [u, v]) => s + dist(u, v), 0)
  const matchingCost = matchingEdges.reduce((s, [u, v]) => s + dist(u, v), 0)
  const eulerCost = mstCost + matchingCost
  const tourCost = closedTourCost(shortcutTour, dist)
  const opt = bruteForceOpt(n, dist)
  const ratio = tourCost / opt

  return {
    phase: 'mst',
    cities,
    n,
    mstEdges,
    odd,
    matchingEdges,
    eulerCircuit,
    shortcutTour,
    mstCost,
    matchingCost,
    eulerCost,
    tourCost,
    opt,
    ratio,
    nnCost: nearestNeighborTourCost(n, dist),
    approx2Cost: doubledMstTourCost(n, dist),
    mstRevealed: 0,
    matchingRevealed: 0,
    walkerPos: 0,
    shortcutStep: 0,
    kept: [eulerCircuit[0]],
    skipped: [],
    step: 0,
  }
}

// One Step press — advances the current phase's fine-grained animation, then
// moves to the next phase at each phase boundary. Pure — no mutation.
export function stepOnce(state: SimState): SimState {
  if (state.phase === 'done') return { ...state, step: state.step + 1 }

  // --- MST phase: reveal one Prim edge per step ---
  if (state.phase === 'mst') {
    if (state.mstRevealed < state.mstEdges.length) {
      return { ...state, mstRevealed: state.mstRevealed + 1, step: state.step + 1 }
    }
    return { ...state, phase: 'odd', step: state.step + 1 }
  }

  // --- Odd phase: one step reveals all odd vertices, next advances ---
  if (state.phase === 'odd') {
    return { ...state, phase: 'matching', step: state.step + 1 }
  }

  // --- Matching phase: reveal one pair per step ---
  if (state.phase === 'matching') {
    if (state.matchingRevealed < state.matchingEdges.length) {
      return { ...state, matchingRevealed: state.matchingRevealed + 1, step: state.step + 1 }
    }
    return { ...state, phase: 'euler', step: state.step + 1 }
  }

  // --- Euler phase: walker advances one edge of the circuit ---
  if (state.phase === 'euler') {
    if (state.walkerPos < state.eulerCircuit.length - 1) {
      return { ...state, walkerPos: state.walkerPos + 1, step: state.step + 1 }
    }
    return {
      ...state,
      phase: 'shortcut',
      shortcutStep: 1,
      kept: [state.eulerCircuit[0]],
      skipped: [],
      step: state.step + 1,
    }
  }

  // --- Shortcut phase: walker re-traverses; fresh cities extend the tour,
  //     repeats get crossed out ---
  if (state.phase === 'shortcut') {
    if (state.shortcutStep >= state.eulerCircuit.length) {
      return { ...state, phase: 'done', step: state.step + 1 }
    }
    const city = state.eulerCircuit[state.shortcutStep]
    const isFresh = !state.kept.includes(city)
    return {
      ...state,
      shortcutStep: state.shortcutStep + 1,
      kept: isFresh ? [...state.kept, city] : state.kept,
      skipped: isFresh ? state.skipped : [...state.skipped, state.shortcutStep],
      step: state.step + 1,
    }
  }

  return { ...state, step: state.step + 1 }
}

// Pure simulation logic for the Branch & Bound explainer.
//
// Rust-faithful model (src/tsp/branch_bound.rs), simplified for teaching:
//   - depth-first search with full branching (the solver additionally restricts
//     candidates to the n_nearest geometric neighbours — omitted here so the
//     tree shows every branch)
//   - lower bound per node: LB = running_cost + MST({start} ∪ unvisited)
//     (the solver's bound: the completion must span {start} ∪ unvisited, so its
//     cost is at least the MST of that set — a valid lower bound)
//   - children are ordered by bound ascending (most promising first), and a
//     child is pruned immediately when LB ≥ best tour found so far
//   - a complete tour is a leaf; if it beats `best` it becomes the new best
// The search runs on ≤ 6 cities per scenario so the tree stays renderable.
// Deterministic — no RNG; SimState holds only plain data (cloneable for Back).

import { makeDm } from './explainer-cities'
export { makeDm }

export type NodeStatus = 'open' | 'explored' | 'pruned' | 'leaf' | 'best'

export interface BnBNode {
  id: number
  parent: number | null
  depth: number          // path length (k)
  path: number[]         // partial tour (visited city ids in order)
  cost: number           // running cost of the partial tour
  unvisited: number[]
  lb: number             // cost + MST({start} ∪ unvisited)
  status: NodeStatus
}

export type Phase = 'searching' | 'done'

export interface SimState {
  phase: Phase
  cities: [number, number][]
  n: number
  startCity: number
  dm: number[][]          // distance matrix (plain numbers — cloneable)
  nodes: BnBNode[]
  stack: number[]         // DFS stack (node ids)
  current: number | null  // node being expanded (top of stack)
  bestCost: number | null
  bestTour: number[] | null
  explored: number        // nodes pushed onto the stack
  pruned: number          // children filtered by LB ≥ best
  leaves: number          // complete tours evaluated
  lastEvent: string | null
  step: number
}

export interface Scenario {
  label: string
  desc: string
  cities: [number, number][]
}

// ---------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------
// Prim's MST cost over the given city id set (dm is indexed by city id).
export function mstCost(ids: number[], dm: number[][]): number {
  const n = ids.length
  if (n <= 1) return 0
  const inTree = new Array<boolean>(n).fill(false)
  const key = new Array<number>(n).fill(Infinity)
  key[0] = 0
  let total = 0
  for (let iter = 0; iter < n; iter++) {
    let u = -1
    let best = Infinity
    for (let i = 0; i < n; i++) {
      if (!inTree[i] && key[i] < best) {
        best = key[i]
        u = i
      }
    }
    if (u === -1) return total // disconnected — no further edges can be added
    inTree[u] = true
    total += key[u]
    for (let v = 0; v < n; v++) {
      if (!inTree[v]) {
        const d = dm[ids[u]][ids[v]]
        if (d < key[v]) key[v] = d
      }
    }
  }
  return total
}

// ---------------------------------------------------------------
// Scenario layouts (6 cities — tuned offline, see tests)
// ---------------------------------------------------------------
export const SCENARIOS: Record<string, Scenario> = {
  small_grid: {
    label: 'Small grid',
    desc: 'A compact 6-city instance — the tree completes quickly and shows the full search',
    cities: [[60, 60], [240, 60], [240, 240], [60, 240], [150, 60], [150, 240]],
  },
  good_bound: {
    label: 'Good bound',
    desc: 'A layout where the MST bound is tight — pruning cuts the tree to a couple of dozen nodes',
    cities: [[36, 189], [238, 108], [166, 57], [235, 123], [159, 67], [172, 178]],
  },
  worst_case: {
    label: 'Worst case',
    desc: 'A scattered layout with a weak bound — the search nearly enumerates the whole tree',
    cities: [[221, 167], [83, 178], [184, 162], [197, 243], [263, 44], [188, 171]],
  },
  early_best: {
    label: 'Early best',
    desc: 'Cities on a circle — the first complete tour is already optimal, so almost everything prunes away',
    cities: [[270, 150], [210, 254], [90, 254], [30, 150], [90, 46], [210, 46]],
  },
}

// ---------------------------------------------------------------
// makeInitState + stepOnce
// ---------------------------------------------------------------
export function makeInitState(scenario: Scenario): SimState {
  const cities = scenario.cities
  const n = cities.length
  const dm = makeDm(cities)
  const startCity = 0
  const unvisited = Array.from({ length: n }, (_, i) => i).filter((i) => i !== startCity)
  const root: BnBNode = {
    id: 0,
    parent: null,
    depth: 1,
    path: [startCity],
    cost: 0,
    unvisited,
    lb: mstCost([startCity, ...unvisited], dm),
    status: 'open',
  }
  return {
    phase: 'searching',
    cities,
    n,
    startCity,
    dm,
    nodes: [root],
    stack: [0],
    current: 0,
    bestCost: null,
    bestTour: null,
    explored: 1,
    pruned: 0,
    leaves: 0,
    lastEvent: null,
    step: 0,
  }
}

// One search step: expand the frontier node's next candidate (create a child,
// prune it if its bound cannot beat the best, or backtrack when exhausted).
export function stepOnce(state: SimState): SimState {
  if (state.phase === 'done') return { ...state, step: state.step + 1 }

  if (state.stack.length === 0) {
    return { ...state, phase: 'done', current: null, lastEvent: 'Search complete — optimal tour found', step: state.step + 1 }
  }

  const topId = state.stack[state.stack.length - 1]
  const top = state.nodes[topId]
  const prev = top.path[top.path.length - 1]

  // Candidates not yet branched: unvisited sorted by distance from the
  // previous city. This equals the solver's bound-ascending order — for a
  // fixed node the MST({start} ∪ unvisited) term is the same for every
  // candidate, so the lower bound differs only in the added edge.
  const childCount = state.nodes.filter((nd) => nd.parent === topId).length
  const candidates = [...top.unvisited].sort((a, b) => state.dm[prev][a] - state.dm[prev][b])

  if (childCount < candidates.length) {
    const c = candidates[childCount]
    const path = [...top.path, c]
    const cost = top.cost + state.dm[prev][c]
    const unvisited = top.unvisited.filter((u) => u !== c)
    const id = state.nodes.length

    if (unvisited.length === 0) {
      // Leaf — a complete tour. bestTour stores the closed cycle (start re-appended).
      const tourCost = cost + state.dm[c][state.startCity]
      const isBest = state.bestCost === null || tourCost < state.bestCost
      const node: BnBNode = {
        id, parent: topId, depth: path.length, path, cost, unvisited,
        lb: tourCost,
        status: isBest ? 'best' : 'leaf',
      }
      return {
        ...state,
        nodes: [...state.nodes, node],
        leaves: state.leaves + 1,
        bestCost: isBest ? tourCost : state.bestCost,
        bestTour: isBest ? [...path, state.startCity] : state.bestTour,
        lastEvent: isBest
          ? `New best — complete tour ${path.join('→')} costs ${tourCost.toFixed(0)}`
          : `Leaf — tour ${path.join('→')} costs ${tourCost.toFixed(0)} (not better than ${state.bestCost!.toFixed(0)})`,
        step: state.step + 1,
      }
    }

    // Interior child: bound = cost + MST({start} ∪ unvisited)
    const lb = cost + mstCost([state.startCity, ...unvisited], state.dm)
    if (state.bestCost !== null && lb >= state.bestCost) {
      // Pruned immediately — cannot beat the best tour
      const node: BnBNode = {
        id, parent: topId, depth: path.length, path, cost, unvisited, lb, status: 'pruned',
      }
      return {
        ...state,
        nodes: [...state.nodes, node],
        pruned: state.pruned + 1,
        lastEvent: `Pruned — bound ${lb.toFixed(0)} ≥ best ${state.bestCost.toFixed(0)} (city ${c} after ${prev})`,
        step: state.step + 1,
      }
    }

    const node: BnBNode = {
      id, parent: topId, depth: path.length, path, cost, unvisited, lb, status: 'open',
    }
    return {
      ...state,
      nodes: [...state.nodes, node],
      stack: [...state.stack, id],
      current: id,
      explored: state.explored + 1,
      lastEvent: `Expanded — added ${c} after ${prev} (bound ${lb.toFixed(0)})`,
      step: state.step + 1,
    }
  }

  // All candidates processed — backtrack
  const rest = state.stack.slice(0, -1)
  const parent = rest.length > 0 ? rest[rest.length - 1] : null
  return {
    ...state,
    stack: rest,
    current: parent,
    // mark this node explored (it stays in the tree, but no longer frontier)
    nodes: state.nodes.map((nd) => (nd.id === topId ? { ...nd, status: 'explored' as NodeStatus } : nd)),
    lastEvent: `Backtracked — no better branch under ${top.path.join('→')}`,
    step: state.step + 1,
  }
}

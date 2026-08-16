// Pure simulation logic for the Savings Construction explainer.
// Forked from greedy-edge-algo.ts — same `select_edges` scan (degree ≤ 2,
// no premature sub-cycle via union-find), same termination guarantee, only
// the sort key differs: edges are ranked by Clarke-Wright *savings* (descending)
// instead of raw distance (ascending). A hub city (nearest the centroid) is
// computed and highlighted but visited like every other city.

import { CITIES_12 as CITIES, N_CITIES_12 as N_CITIES, dist12 as dist } from './explainer-cities'
export { CITIES, N_CITIES, dist }

export type Edge = { u: number; v: number; dist: number; val: number }

export type RejectReason = 'degree' | 'cycle'
export type EventMode = 'accepted' | 'closing' | 'rejected-degree' | 'rejected-cycle' | 'done'

export type SimState = {
  sortedEdges: Edge[]        // all n(n-1)/2 pairs, descending by savings
  hubIndex: number           // city nearest the coordinate centroid
  scanIndex: number          // next edge to evaluate
  parent: number[]           // union-find: parent[i]
  degree: number[]           // degree[i]
  accepted: Edge[]           // accepted edges, in scan order
  rejected: Array<{ edge: Edge; reason: RejectReason }>
  step: number
  lastEdge: Edge | null      // edge evaluated on the most recent step
  lastEvent: EventMode | null
  done: boolean
  tour: number[] | null      // ordered path once the cycle closes (null until done)
}

// Savings for edge (i, j) relative to hub h: s(i,j) = d(h,i) + d(h,j) - d(i,j).
// Large positive = i and j are close to each other but far from the hub — the
// edges a good tour wants. Used as the sort key only; distance is still dist(i,j).
function savings(i: number, j: number, hub: number): number {
  return dist(hub, i) + dist(hub, j) - dist(i, j)
}

// City nearest the coordinate centroid — matches the Rust `hub_position`.
export function hubIndex(): number {
  let cx = 0, cy = 0
  for (const [x, y] of CITIES) { cx += x; cy += y }
  cx /= N_CITIES; cy /= N_CITIES
  let best = 0, bestD2 = Infinity
  for (let i = 0; i < N_CITIES; i++) {
    const [x, y] = CITIES[i]
    const d2 = (x - cx) ** 2 + (y - cy) ** 2
    if (d2 < bestD2) { bestD2 = d2; best = i }
  }
  return best
}

// All n(n-1)/2 pairwise edges, descending by savings (highest first).
function sortedEdgesBySavings(hub: number): Edge[] {
  const edges: Edge[] = []
  for (let i = 0; i < N_CITIES; i++) {
    for (let j = i + 1; j < N_CITIES; j++) {
      edges.push({ u: i, v: j, dist: dist(i, j), val: savings(i, j, hub) })
    }
  }
  // descending by savings; stable tiebreak on (u, v)
  edges.sort((a, b) => b.val - a.val || a.u - b.u || a.v - b.v)
  return edges
}

function makeUnionFind(n: number): number[] {
  return Array.from({ length: n }, (_, i) => i)
}

function find(parent: number[], x: number): number {
  let root = x
  while (parent[root] !== root) root = parent[root]
  while (parent[x] !== root) {
    const next = parent[x]
    parent[x] = root
    x = next
  }
  return root
}

function connected(parent: number[], a: number, b: number): boolean {
  return find(parent, a) === find(parent, b)
}

function union(parent: number[], a: number, b: number): void {
  const ra = find(parent, a)
  const rb = find(parent, b)
  if (ra !== rb) parent[ra] = rb
}

export function components(parent: number[]): number[][] {
  const groups: Record<number, number[]> = {}
  for (let i = 0; i < parent.length; i++) {
    const r = find(parent, i)
    ;(groups[r] ??= []).push(i)
  }
  return Object.values(groups).map(g => g.sort((a, b) => a - b))
}

export function makeInitState(): SimState {
  const hub = hubIndex()
  return {
    sortedEdges: sortedEdgesBySavings(hub),
    hubIndex: hub,
    scanIndex: 0,
    parent: makeUnionFind(N_CITIES),
    degree: new Array(N_CITIES).fill(0),
    accepted: [],
    rejected: [],
    step: 0,
    lastEdge: null,
    lastEvent: null,
    done: false,
    tour: null,
  }
}

function walkCycle(accepted: Edge[]): number[] {
  const n = accepted.length
  const adj: number[][] = Array.from({ length: n }, () => [])
  for (const e of accepted) {
    adj[e.u].push(e.v)
    adj[e.v].push(e.u)
  }
  const path: number[] = []
  const seen = new Array(n).fill(false)
  let prev = -1
  let cur = 0
  for (let i = 0; i < n; i++) {
    path.push(cur)
    seen[cur] = true
    const next = adj[cur].find(x => x !== prev && !seen[x]) ?? adj[cur][0]
    prev = cur
    cur = next
  }
  return path
}

export function stepOnce(s: SimState): SimState {
  if (s.done) return s

  const edges = s.sortedEdges
  let idx = s.scanIndex
  const parent = s.parent.slice()
  const degree = s.degree.slice()
  const accepted = s.accepted.slice()
  const rejected = s.rejected.slice()
  let lastEdge: Edge | null = s.lastEdge
  let lastEvent: EventMode | null = s.lastEvent

  while (idx < edges.length) {
    const e = edges[idx]
    idx++
    lastEdge = e

    if (degree[e.u] >= 2 || degree[e.v] >= 2) {
      rejected.push({ edge: e, reason: 'degree' })
      lastEvent = 'rejected-degree'
      continue
    }
    const isClosing = accepted.length === N_CITIES - 1
    if (connected(parent, e.u, e.v) && !isClosing) {
      rejected.push({ edge: e, reason: 'cycle' })
      lastEvent = 'rejected-cycle'
      continue
    }

    union(parent, e.u, e.v)
    degree[e.u]++
    degree[e.v]++
    accepted.push(e)
    lastEvent = isClosing ? 'closing' : 'accepted'

    if (accepted.length === N_CITIES) {
      return {
        ...s,
        scanIndex: idx,
        parent,
        degree,
        accepted,
        rejected,
        step: s.step + 1,
        lastEdge,
        lastEvent,
        done: true,
        tour: walkCycle(accepted),
      }
    }
    break
  }

  return {
    ...s,
    scanIndex: idx,
    parent,
    degree,
    accepted,
    rejected,
    step: s.step + 1,
    lastEdge,
    lastEvent,
    done: false,
    tour: null,
  }
}

export function runToCompletion(s: SimState): SimState {
  let cur = s
  while (!cur.done) cur = stepOnce(cur)
  return cur
}

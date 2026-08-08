// Pure simulation logic for the Greedy Edge Construction explainer.
// Mirrors the Rust `select_edges` scan in src/tsp/graph.rs:98-130 — same
// invariants (degree <= 2, no premature sub-cycle via union-find), same
// guarantee of terminating with exactly n accepted edges on a complete graph.

export const CITIES: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]
export const N_CITIES = CITIES.length

export type Edge = { u: number; v: number; dist: number }

export type RejectReason = 'degree' | 'cycle'
export type EventMode = 'accepted' | 'closing' | 'rejected-degree' | 'rejected-cycle' | 'done'

export type SimState = {
  sortedEdges: Edge[]        // all n(n-1)/2 pairs, ascending by distance
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

export function dist(i: number, j: number): number {
  const [x1, y1] = CITIES[i]
  const [x2, y2] = CITIES[j]
  return Math.hypot(x2 - x1, y2 - y1)
}

// All n(n-1)/2 pairwise edges, ascending by distance. Stable tiebreak on
// (u, v) keeps the scan order deterministic across runs, matching the Rust
// sorted_edges (which uses a stable sort over u32 endpoint pairs).
export function sortedEdges(): Edge[] {
  const edges: Edge[] = []
  for (let i = 0; i < N_CITIES; i++) {
    for (let j = i + 1; j < N_CITIES; j++) {
      edges.push({ u: i, v: j, dist: dist(i, j) })
    }
  }
  edges.sort((a, b) => a.dist - b.dist || a.u - b.u || a.v - b.v)
  return edges
}

function makeUnionFind(n: number): number[] {
  return Array.from({ length: n }, (_, i) => i)
}

function find(parent: number[], x: number): number {
  // path-compressing find (iterative; safe for the small n here)
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

// Current union-find components as sets of city indices — for the side panel.
export function components(parent: number[]): number[][] {
  const groups: Record<number, number[]> = {}
  for (let i = 0; i < parent.length; i++) {
    const r = find(parent, i)
    ;(groups[r] ??= []).push(i)
  }
  return Object.values(groups).map(g => g.sort((a, b) => a - b))
}

export function makeInitState(): SimState {
  return {
    sortedEdges: sortedEdges(),
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

// Walks the accepted degree-2 cycle into an ordered path starting at city 0.
// Mirrors Rust `hamiltonian_cycle_to_path` (graph.rs:140) — only called once
// the scan has placed all n edges.
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

// Evaluate edges one at a time, skipping rejects, until one is accepted OR a
// reject worth showing is found. A single user "step" advances the *accepted*
// frontier by one (skipping over any intervening rejects), so the viz always
// shows forward progress — mirroring how the Rust loop's `continue` statements
// are invisible bookkeeping around each acceptance.
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
    // The n-th accepted edge (0-indexed n-1) closes the cycle and MUST be a
    // same-component edge — the only time that is allowed.
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

    // Did the scan place all n edges? Rust asserts accepted.len() == n here.
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
    // accepted-frontier advanced by one: stop here so the user sees this edge.
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

// Run to completion from any state — used by the "Run" button and by tests.
export function runToCompletion(s: SimState): SimState {
  let cur = s
  while (!cur.done) cur = stepOnce(cur)
  return cur
}

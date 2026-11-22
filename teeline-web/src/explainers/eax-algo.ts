// Pure simulation logic for the Memetic EAX explainer.
//
// This is a teaching model, not a second solver path: it owns one pinned crossover
// event (16 cities, two parent tours) and derives the union graph, AB-cycles, the
// E-set selection, the symmetric difference, subtour patching, a one-move 2-opt
// preview, and the teaching statistics. The fixture is pinned in `design.md`
// Decision 1, and `eax-algo.test.ts` asserts every invariant.

export interface Pt {
  x: number
  y: number
}

// Pinned 16-city layout (0–300 viewBox). The order is the city index.
export const CITIES: Pt[] = [
  { x: 179.7, y: 55.1 }, // 0
  { x: 141.3, y: 97.7 }, // 1
  { x: 123.3, y: 43.5 }, // 2
  { x: 85.9, y: 85.7 }, // 3
  { x: 102.2, y: 140.4 }, // 4
  { x: 65.6, y: 30.0 }, // 5
  { x: 30.0, y: 74.1 }, // 6
  { x: 79.2, y: 194.5 }, // 7
  { x: 61.3, y: 248.2 }, // 8
  { x: 119.5, y: 234.6 }, // 9
  { x: 172.5, y: 214.3 }, // 10
  { x: 211.1, y: 255.9 }, // 11
  { x: 248.7, y: 212.8 }, // 12
  { x: 270.0, y: 160.2 }, // 13
  { x: 223.9, y: 126.8 }, // 14
  { x: 171.2, y: 148.3 }, // 15
]
export const N_CITIES = CITIES.length

export const PARENT_A: number[] = [
  0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
]
export const PARENT_B: number[] = [
  0, 5, 6, 2, 1, 4, 3, 8, 7, 9, 10, 11, 12, 13, 14, 15,
]

/** The AB-cycle the default `1-AB` selection pins (the subtour-producing one). */
export const PINNED_CYCLE = 0
export const DEFAULT_SEED = 7

export type Edge = readonly [number, number]
export type EdgeKind = 'shared' | 'a-only' | 'b-only'

export interface UnionEdge {
  edge: Edge
  kind: EdgeKind
}

export interface AbCycle {
  id: number
  vertices: number[]
  aEdges: Edge[]
  bEdges: Edge[]
}

export function makeEdge(u: number, v: number): Edge {
  return u < v ? [u, v] : [v, u]
}

export function edgeKey(e: Edge): string {
  return `${e[0]}-${e[1]}`
}

export function vertexKey(u: number, v: number): string {
  return edgeKey(makeEdge(u, v))
}

export function parseEdge(key: string): Edge {
  const [u, v] = key.split('-').map(Number)
  return [u, v]
}

export function dist(i: number, j: number): number {
  return Math.hypot(CITIES[j].x - CITIES[i].x, CITIES[j].y - CITIES[i].y)
}

export function tourEdges(tour: number[]): Edge[] {
  const out: Edge[] = []
  for (let i = 0; i < tour.length; i++) {
    out.push(makeEdge(tour[i], tour[(i + 1) % tour.length]))
  }
  return out
}

export function tourEdgeKeys(tour: number[]): Set<string> {
  return new Set(tourEdges(tour).map(edgeKey))
}

export function tourLength(tour: number[]): number {
  let total = 0
  for (let i = 0; i < tour.length; i++) {
    total += dist(tour[i], tour[(i + 1) % tour.length])
  }
  return total
}

/** Union of both parent edge sets, classified as shared / A-only / B-only. */
export function unionEdges(a: number[], b: number[]): UnionEdge[] {
  const bSet = tourEdgeKeys(b)
  const seen = new Set<string>()
  const out: UnionEdge[] = []
  for (const e of tourEdges(a)) {
    const key = edgeKey(e)
    if (seen.has(key)) continue
    seen.add(key)
    out.push({ edge: e, kind: bSet.has(key) ? 'shared' : 'a-only' })
  }
  for (const e of tourEdges(b)) {
    const key = edgeKey(e)
    if (seen.has(key)) continue
    seen.add(key)
    out.push({ edge: e, kind: 'b-only' })
  }
  return out
}

export function sharedEdgeCount(a: number[], b: number[]): number {
  const bSet = tourEdgeKeys(b)
  let count = 0
  const seen = new Set<string>()
  for (const e of tourEdges(a)) {
    const key = edgeKey(e)
    if (seen.has(key)) continue
    seen.add(key)
    if (bSet.has(key)) count++
  }
  return count
}

/**
 * Decomposes the symmetric difference of two tours into alternating AB-cycles.
 *
 * The pinned fixture has difference degree 2 at every active city (one A-only and
 * one B-only edge), so the pairing is forced and the decomposition is unique. The
 * generic index-pairing fallback mirrors `src/tsp/memetic_eax.rs` for safety.
 */
export function abCycles(a: number[], b: number[]): AbCycle[] {
  const aSet = tourEdgeKeys(a)
  const bSet = tourEdgeKeys(b)
  const aOnly = tourEdges(a).filter((e) => !bSet.has(edgeKey(e)))
  const bOnly = tourEdges(b).filter((e) => !aSet.has(edgeKey(e)))

  const incA = new Map<number, Edge[]>()
  const incB = new Map<number, Edge[]>()
  const add = (m: Map<number, Edge[]>, v: number, e: Edge) => {
    const list = m.get(v) ?? []
    list.push(e)
    m.set(v, list)
  }
  for (const e of aOnly) {
    add(incA, e[0], e)
    add(incA, e[1], e)
  }
  for (const e of bOnly) {
    add(incB, e[0], e)
    add(incB, e[1], e)
  }

  const aPartner = new Map<string, Edge>()
  const bPartner = new Map<string, Edge>()
  for (const [v, al] of incA) {
    const bl = incB.get(v) ?? []
    if (al.length !== bl.length)
      throw new Error('parent tours disagree on edge parity')
    for (let i = 0; i < al.length; i++) {
      aPartner.set(`${edgeKey(al[i])}@${v}`, bl[i])
      bPartner.set(`${edgeKey(bl[i])}@${v}`, al[i])
    }
  }

  const used = new Set<string>()
  const cycles: AbCycle[] = []
  for (const start of aOnly) {
    if (used.has(edgeKey(start))) continue
    const vertices: number[] = [start[0]]
    const aEdges: Edge[] = []
    const bEdges: Edge[] = []
    let entry = start[0]
    let edge = start
    while (true) {
      used.add(edgeKey(edge))
      aEdges.push(edge)
      const exit = edge[0] === entry ? edge[1] : edge[0]
      vertices.push(exit)
      const bEdge = aPartner.get(`${edgeKey(edge)}@${exit}`)
      if (!bEdge) throw new Error('unpaired A endpoint')
      used.add(edgeKey(bEdge))
      bEdges.push(bEdge)
      const bExit = bEdge[0] === exit ? bEdge[1] : bEdge[0]
      if (bExit === start[0]) break
      vertices.push(bExit)
      const nextA = bPartner.get(`${edgeKey(bEdge)}@${bExit}`)
      if (!nextA) throw new Error('unpaired B endpoint')
      entry = bExit
      edge = nextA
    }
    cycles.push({ id: cycles.length, vertices, aEdges, bEdges })
  }
  return cycles
}

/** Applies the selected AB-cycles to Parent A as a symmetric difference. */
export function applyESet(
  a: number[],
  cycles: AbCycle[],
  selected: number[],
): Set<string> {
  const edges = tourEdgeKeys(a)
  for (const id of selected) {
    const cycle = cycles[id]
    if (!cycle) continue
    for (const e of cycle.aEdges) edges.delete(edgeKey(e))
    for (const e of cycle.bEdges) edges.add(edgeKey(e))
  }
  return edges
}

/** Ordered vertex list of each connected component (each is a cycle). */
export function componentsFromEdges(edges: Set<string>): number[][] {
  const adj = new Map<number, number[]>()
  const add = (v: number, w: number) => {
    const list = adj.get(v) ?? []
    list.push(w)
    adj.set(v, list)
  }
  for (const key of edges) {
    const [u, v] = parseEdge(key)
    add(u, v)
    add(v, u)
  }

  const seen = new Set<number>()
  const comps: number[][] = []
  for (let start = 0; start < N_CITIES; start++) {
    if (seen.has(start) || !adj.has(start)) continue
    const path: number[] = [start]
    seen.add(start)
    let prev = -1
    let cur = start
    while (true) {
      const neighbours = adj.get(cur) ?? []
      const next = neighbours.find((x) => x !== prev)
      if (next === undefined || next === start) break
      path.push(next)
      seen.add(next)
      prev = cur
      cur = next
    }
    comps.push(path)
  }
  return comps
}

export interface PatchStep {
  small: number[]
  removed: [Edge, Edge]
  added: [Edge, Edge]
  delta: number
}

interface Group {
  vertices: number[]
  edges: Edge[]
}

function cycleEdges(vertices: number[]): Edge[] {
  return vertices.map((v, i) =>
    makeEdge(v, vertices[(i + 1) % vertices.length]),
  )
}

function edgeListKey(a: Edge, b: Edge): string {
  const first = edgeKey(a)
  const second = edgeKey(b)
  return first < second ? `${first}|${second}` : `${second}|${first}`
}

/**
 * Greedy subtour patching: repeatedly merge the smallest component into another
 * component via the exhaustive cheapest pair of edges, scoring both rewirings.
 * Ties break on the canonical edge list so Node and the browser never diverge.
 */
export function patchComponents(comps: number[][]): {
  tour: number[]
  steps: PatchStep[]
  states: number[][][]
} {
  let groups: Group[] = comps.map((vertices) => ({
    vertices: vertices.slice(),
    edges: cycleEdges(vertices),
  }))
  const steps: PatchStep[] = []
  const states: number[][][] = [groups.map((g) => g.vertices.slice())]

  while (groups.length > 1) {
    let smallIdx = 0
    for (let i = 1; i < groups.length; i++) {
      if (groups[i].vertices.length < groups[smallIdx].vertices.length)
        smallIdx = i
    }

    let best: {
      delta: number
      removed: [Edge, Edge]
      added: [Edge, Edge]
      otherIdx: number
      key: string
    } | null = null

    for (let oi = 0; oi < groups.length; oi++) {
      if (oi === smallIdx) continue
      for (const ea of groups[smallIdx].edges) {
        const [a, b] = ea
        for (const eb of groups[oi].edges) {
          const [c, d] = eb
          if (new Set([a, b, c, d]).size < 4) continue
          const removedLen = dist(a, b) + dist(c, d)
          const options: Array<{ added: [Edge, Edge]; delta: number }> = [
            {
              added: [makeEdge(a, c), makeEdge(b, d)],
              delta: dist(a, c) + dist(b, d) - removedLen,
            },
            {
              added: [makeEdge(a, d), makeEdge(b, c)],
              delta: dist(a, d) + dist(b, c) - removedLen,
            },
          ]
          for (const option of options) {
            const key = edgeListKey(ea, eb) + '|' + edgeListKey(...option.added)
            if (
              best === null ||
              option.delta < best.delta - 1e-12 ||
              (Math.abs(option.delta - best.delta) <= 1e-12 && key < best.key)
            ) {
              best = {
                delta: option.delta,
                removed: [ea, eb],
                added: option.added,
                otherIdx: oi,
                key,
              }
            }
          }
        }
      }
    }
    if (best === null) throw new Error('no bridge between components')

    steps.push({
      small: groups[smallIdx].vertices.slice(),
      removed: best.removed,
      added: best.added,
      delta: best.delta,
    })

    const mergedEdges = groups[smallIdx].edges
      .filter((e) => edgeKey(e) !== edgeKey(best!.removed[0]))
      .concat(
        groups[best.otherIdx].edges.filter(
          (e) => edgeKey(e) !== edgeKey(best!.removed[1]),
        ),
      )
      .concat(best.added)
    const merged = orderFromEdges(mergedEdges)
    groups = groups.filter((_, i) => i !== smallIdx && i !== best!.otherIdx)
    groups.push({ vertices: merged, edges: cycleEdges(merged) })
    states.push(groups.map((g) => g.vertices.slice()))
  }

  return { tour: groups[0].vertices, steps, states }
}

function orderFromEdges(edges: Edge[]): number[] {
  const adj = new Map<number, number[]>()
  const add = (v: number, w: number) => {
    const list = adj.get(v) ?? []
    list.push(w)
    adj.set(v, list)
  }
  for (const e of edges) {
    add(e[0], e[1])
    add(e[1], e[0])
  }
  const start = Math.min(...adj.keys())
  const order: number[] = [start]
  let prev = -1
  let cur = start
  while (true) {
    const neighbours = adj.get(cur) ?? []
    const next = neighbours.find((x) => x !== prev)
    if (next === undefined || next === start) break
    order.push(next)
    prev = cur
    cur = next
  }
  return order
}

export interface TwoOptMove {
  removed: [Edge, Edge]
  added: [Edge, Edge]
  oldLength: number
  newLength: number
  delta: number
}

/** Best single improving 2-opt move over all edge pairs, or null. */
export function bestTwoOpt(tour: number[]): TwoOptMove | null {
  const n = tour.length
  let best: TwoOptMove | null = null
  for (let i = 0; i < n - 1; i++) {
    for (let j = i + 2; j < n; j++) {
      if (i === 0 && j === n - 1) continue
      const a = tour[i]
      const b = tour[(i + 1) % n]
      const c = tour[j]
      const d = tour[(j + 1) % n]
      if (new Set([a, b, c, d]).size < 4) continue
      const oldLength = dist(a, b) + dist(c, d)
      // Removing two edges from a single cycle admits two rewirings: (a,c)+(b,d)
      // yields a single tour, while (a,d)+(b,c) closes each path into its own
      // subtour. Only the first is a valid 2-opt move here (both are valid for the
      // patching pass, which merges two separate components).
      const newLength = dist(a, c) + dist(b, d)
      const delta = newLength - oldLength
      if (delta >= -1e-9) continue
      if (best === null || delta < best.delta - 1e-12) {
        best = {
          removed: [makeEdge(a, b), makeEdge(c, d)],
          added: [makeEdge(a, c), makeEdge(b, d)],
          oldLength,
          newLength,
          delta,
        }
      }
    }
  }
  return best
}

/** Mulberry32 — the seeded PRNG used by `Rand`. */
export function mulberry32(seed: number): () => number {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

/**
 * Seeded `Rand` selection: each cycle independently with probability 1/2; if none
 * is selected, pick one uniformly (matching the solver's `select_e_set`).
 */
export function selectRand(count: number, seed: number): number[] {
  const rng = mulberry32(seed)
  const selected: number[] = []
  for (let i = 0; i < count; i++) {
    if (rng() < 0.5) selected.push(i)
  }
  if (selected.length === 0) {
    selected.push(Math.floor(rng() * count))
  }
  return selected
}

export interface Inheritance {
  shared: number
  fromA: number
  fromB: number
  new: number
}

export interface Statistics {
  sharedEdges: number
  sharedRatio: number
  cycleCount: number
  cycleLengths: number[]
  eSetSize: number
  subtours: number
  bridges: number
  costA: number
  costB: number
  costChild: number
  inheritance: Inheritance
}

export function inheritance(child: number[]): Inheritance {
  const aSet = tourEdgeKeys(PARENT_A)
  const bSet = tourEdgeKeys(PARENT_B)
  const out: Inheritance = { shared: 0, fromA: 0, fromB: 0, new: 0 }
  for (const key of tourEdgeKeys(child)) {
    const inA = aSet.has(key)
    const inB = bSet.has(key)
    if (inA && inB) out.shared++
    else if (inA) out.fromA++
    else if (inB) out.fromB++
    else out.new++
  }
  return out
}

export function makeStatistics(
  cycles: AbCycle[],
  selected: number[],
  subtours: number[][],
  steps: PatchStep[],
  child: number[],
): Statistics {
  const shared = sharedEdgeCount(PARENT_A, PARENT_B)
  return {
    sharedEdges: shared,
    sharedRatio: shared / N_CITIES,
    cycleCount: cycles.length,
    cycleLengths: cycles.map((c) => c.vertices.length),
    eSetSize: selected.length,
    subtours: subtours.length,
    bridges: steps.length,
    costA: tourLength(PARENT_A),
    costB: tourLength(PARENT_B),
    costChild: tourLength(child),
    inheritance: inheritance(child),
  }
}

export type FrameKind =
  | 'parents'
  | 'union'
  | 'cycle'
  | 'cycles-summary'
  | 'select'
  | 'remove-a'
  | 'add-b'
  | 'bridge'
  | 'no-patch'
  | 'final'

export interface Frame {
  stage: 1 | 2 | 3 | 4 | 5 | 6
  kind: FrameKind
  cycleId?: number
  bridge?: PatchStep
  bridgeIndex?: number
}

export interface Simulation {
  cycles: AbCycle[]
  selected: number[]
  subtours: number[][]
  steps: PatchStep[]
  states: number[][][]
  child: number[]
  stats: Statistics
  frames: Frame[]
}

/** Derives the full simulation (and the flat frame sequence) for a selection. */
export function simulate(selected: number[]): Simulation {
  const cycles = abCycles(PARENT_A, PARENT_B)
  const edges = applyESet(PARENT_A, cycles, selected)
  const subtours = componentsFromEdges(edges)
  const { tour: child, steps, states } = patchComponents(subtours)
  const stats = makeStatistics(cycles, selected, subtours, steps, child)

  const frames: Frame[] = [
    { stage: 1, kind: 'parents' },
    { stage: 2, kind: 'union' },
  ]
  for (const cycle of cycles) {
    frames.push({ stage: 3, kind: 'cycle', cycleId: cycle.id })
  }
  frames.push({ stage: 3, kind: 'cycles-summary' })
  frames.push({ stage: 4, kind: 'select' })
  frames.push({ stage: 5, kind: 'remove-a' })
  frames.push({ stage: 5, kind: 'add-b' })
  for (let i = 0; i < steps.length; i++) {
    frames.push({ stage: 6, kind: 'bridge', bridge: steps[i], bridgeIndex: i })
  }
  if (steps.length === 0) frames.push({ stage: 6, kind: 'no-patch' })
  frames.push({ stage: 6, kind: 'final' })

  return { cycles, selected, subtours, steps, states, child, stats, frames }
}

import { describe, it, expect } from 'vitest'
import {
  SCENARIOS, makeInitState, stepOnce,
  mstCost, makeDm,
} from './branch-bound-algo'
import type { SimState } from './branch-bound-algo'

// Brute-force optimum over the same instance (fixed start at 0).
function bruteForce(cities: [number, number][]): number {
  const n = cities.length
  const dm = makeDm(cities)
  const rest = Array.from({ length: n - 1 }, (_, i) => i + 1)
  const used = new Array<boolean>(rest.length).fill(false)
  const order: number[] = [0]
  let best = Infinity
  const search = () => {
    if (order.length === n) {
      let d = 0
      for (let k = 0; k < n; k++) d += dm[order[k]][order[(k + 1) % n]]
      if (d < best) best = d
      return
    }
    for (let i = 0; i < rest.length; i++) {
      if (used[i]) continue
      used[i] = true
      order.push(rest[i])
      search()
      order.pop()
      used[i] = false
    }
  }
  search()
  return best
}

function runToDone(scenario: (typeof SCENARIOS)[string]): SimState {
  let s = makeInitState(scenario)
  let guard = 5000
  while (s.phase !== 'done' && guard-- > 0) s = stepOnce(s)
  expect(s.phase).toBe('done')
  return s
}

// ---------------------------------------------------------------
describe('mstCost', () => {
  it('is zero for a single node', () => {
    expect(mstCost([0], makeDm([[0, 0], [1, 0]]))).toBe(0)
  })

  it('finds the minimal chain MST', () => {
    const dm = makeDm([[0, 0], [1, 0], [2, 0], [3, 0], [4, 0]])
    expect(mstCost([0, 1, 2, 3, 4], dm)).toBeCloseTo(4, 6)
  })
})

// ---------------------------------------------------------------
describe('makeInitState', () => {
  it('creates the root node with the MST bound', () => {
    const s = makeInitState(SCENARIOS.small_grid)
    expect(s.phase).toBe('searching')
    expect(s.nodes).toHaveLength(1)
    expect(s.stack).toEqual([0])
    expect(s.current).toBe(0)
    expect(s.bestCost).toBeNull()
    expect(s.step).toBe(0)
    // root bound = MST over all 6 cities
    expect(s.nodes[0].lb).toBeCloseTo(mstCost([0, 1, 2, 3, 4, 5], s.dm), 6)
  })
})

// ---------------------------------------------------------------
describe('stepOnce', () => {
  it('expands the root into its first child and pushes it', () => {
    const s0 = makeInitState(SCENARIOS.small_grid)
    const s1 = stepOnce(s0)
    expect(s1.nodes).toHaveLength(2)
    expect(s1.stack).toEqual([0, 1])
    expect(s1.current).toBe(1)
    expect(s1.nodes[1].depth).toBe(2)
    expect(s1.nodes[1].path).toHaveLength(2)
    expect(s1.lastEvent).toContain('Expanded')
  })

  it('a complete tour at a leaf becomes the new best', () => {
    // early_best: the first leaf (a full tour) is reached quickly
    let s = makeInitState(SCENARIOS.early_best)
    let guard = 20
    while (s.bestCost === null && guard-- > 0) s = stepOnce(s)
    expect(s.bestCost).not.toBeNull()
    expect(s.bestTour).toHaveLength(6)
    expect(s.nodes.some((nd) => nd.status === 'best')).toBe(true)
  })

  it('prunes children whose bound cannot beat the best', () => {
    let s = makeInitState(SCENARIOS.early_best)
    let guard = 100
    while (s.phase !== 'done' && guard-- > 0) s = stepOnce(s)
    expect(s.pruned).toBeGreaterThan(0)
    expect(s.nodes.some((nd) => nd.status === 'pruned')).toBe(true)
  })

  it('done is a no-op apart from the step counter', () => {
    const done = runToDone(SCENARIOS.small_grid)
    const again = stepOnce(done)
    expect(again.phase).toBe('done')
    expect(again.nodes).toEqual(done.nodes)
    expect(again.step).toBe(done.step + 1)
  })
})

// ---------------------------------------------------------------
describe('correctness & scenario pins', () => {
  const pins: Record<string, { nodes: number; leaves: number }> = {
    small_grid: { nodes: 42, leaves: 2 },
    good_bound: { nodes: 26, leaves: 2 },
    worst_case: { nodes: 298, leaves: 98 },
    early_best: { nodes: 66, leaves: 4 },
  }

  it('every scenario finds the exact optimum (verified against brute force)', () => {
    for (const [key, sc] of Object.entries(SCENARIOS)) {
      const s = runToDone(sc)
      expect(s.bestCost, `${key} must find the optimum`).toBeCloseTo(bruteForce(sc.cities), 6)
      expect([...s.bestTour!].sort((a, b) => a - b), `${key} best tour must be a permutation`).toEqual(
        Array.from({ length: sc.cities.length }, (_, i) => i),
      )
    }
  })

  it('node/leaf counts are pinned per scenario', () => {
    for (const [key, sc] of Object.entries(SCENARIOS)) {
      const s = runToDone(sc)
      expect(s.nodes.length, `${key} node count`).toBe(pins[key].nodes)
      expect(s.leaves, `${key} leaf count`).toBe(pins[key].leaves)
    }
  })

  it('every node has a valid bound and parent linkage', () => {
    const s = runToDone(SCENARIOS.small_grid)
    for (const nd of s.nodes) {
      expect(nd.lb).toBeGreaterThan(0)
      if (nd.parent !== null) {
        const parent = s.nodes[nd.parent]
        expect(parent.depth).toBe(nd.depth - 1)
      }
    }
  })
})

// ---------------------------------------------------------------
describe('purity & determinism', () => {
  it('stepOnce never mutates its input', () => {
    const s = makeInitState(SCENARIOS.worst_case)
    const snapshot = structuredClone(s)
    let cur = s
    for (let i = 0; i < 20; i++) cur = stepOnce(cur)
    expect(s).toEqual(snapshot)
  })

  it('two runs from the same scenario produce identical sequences', () => {
    const a = makeInitState(SCENARIOS.worst_case)
    const b = makeInitState(SCENARIOS.worst_case)
    for (let i = 0; i < 20; i++) {
      const na = stepOnce(a)
      const nb = stepOnce(b)
      expect(na).toEqual(nb)
      Object.assign(a, na)
      Object.assign(b, nb)
    }
  })
})

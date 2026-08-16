import { describe, it, expect } from 'vitest'
import {
  SCENARIOS,
  primMst, oddDegreeNodes, greedyMatching, buildMultigraph, hierholzer, shortcut,
  christofidesPipeline, closedTourCost, makeInitState, stepOnce,
} from './christofides-algo'
import { makeDist } from './explainer-cities'

// ---------------------------------------------------------------
// Step 1 — Prim's MST (ported from the Rust unit tests)
// ---------------------------------------------------------------
describe('primMst', () => {
  it('has exactly n-1 edges and spans all nodes', () => {
    const dist = makeDist([[0, 0], [1, 0], [2, 0], [1, 1]])
    const edges = primMst(4, dist)
    expect(edges).toHaveLength(3)
    const seen = new Set<number>([0])
    for (const [u, v] of edges) {
      seen.add(u)
      seen.add(v)
    }
    expect(seen).toEqual(new Set([0, 1, 2, 3]))
  })

  it('finds the minimal chain MST (weight 4.0)', () => {
    const dist = makeDist([[0, 0], [1, 0], [2, 0], [3, 0], [4, 0]])
    const edges = primMst(5, dist)
    const weight = edges.reduce((s, [u, v]) => s + dist(u, v), 0)
    expect(weight).toBeCloseTo(4.0, 3)
  })
})

// ---------------------------------------------------------------
// Step 2 — odd-degree nodes (ported)
// ---------------------------------------------------------------
describe('oddDegreeNodes', () => {
  it('counts an even number of odd-degree vertices (handshaking lemma)', () => {
    const mst: [number, number][] = [[0, 1], [1, 2], [1, 3]]
    const odd = oddDegreeNodes(mst, 4)
    expect(odd.length % 2).toBe(0)
  })

  it('finds the endpoints of a path', () => {
    const mst: [number, number][] = [[0, 1], [1, 2], [2, 3]]
    expect(oddDegreeNodes(mst, 4).sort()).toEqual([0, 3])
  })
})

// ---------------------------------------------------------------
// Step 3 — greedy matching (ported)
// ---------------------------------------------------------------
describe('greedyMatching', () => {
  it('covers every odd-degree node exactly once', () => {
    const dist = makeDist([[0, 0], [1, 0], [2, 0], [1, 1], [3, 0]])
    const mst = primMst(5, dist)
    const odd = oddDegreeNodes(mst, 5)
    const matching = greedyMatching(odd, dist, 5)
    const count = new Array<number>(5).fill(0)
    for (const [u, v] of matching) {
      count[u]++
      count[v]++
    }
    for (const pos of odd) {
      expect(count[pos]).toBe(1)
    }
  })
})

// ---------------------------------------------------------------
// Step 5 — Hierholzer (ported)
// ---------------------------------------------------------------
describe('hierholzer', () => {
  it('circuit length equals edges + 1 and consumes every edge', () => {
    const dist = makeDist([[0, 0], [1, 0], [2, 0], [1, 1], [3, 0]])
    const { mstEdges, matchingEdges, eulerCircuit } = christofidesPipeline(5, dist)
    const totalEdges = mstEdges.length + matchingEdges.length
    expect(eulerCircuit).toHaveLength(totalEdges + 1)
    // every consecutive pair in the circuit is an edge of the multigraph
    const adj = buildMultigraph(5, mstEdges, matchingEdges)
    const edgeSet = new Set<string>()
    for (let v = 0; v < 5; v++) {
      for (const u of adj[v]) {
        edgeSet.add(`${Math.min(u, v)}-${Math.max(u, v)}`)
      }
    }
    for (let t = 0; t < eulerCircuit.length - 1; t++) {
      const [a, b] = [eulerCircuit[t], eulerCircuit[t + 1]].sort((x, y) => x - y)
      expect(edgeSet.has(`${a}-${b}`), `circuit step ${t} must use a real edge`).toBe(true)
    }
  })
})

// ---------------------------------------------------------------
// Step 6 — shortcut (ported)
// ---------------------------------------------------------------
describe('shortcut', () => {
  it('keeps the first occurrence of each city', () => {
    const circuit = [0, 1, 3, 4, 2, 1, 0]
    const tour = shortcut(circuit, 5)
    expect(tour).toHaveLength(5)
    expect(tour.slice().sort((a, b) => a - b)).toEqual([0, 1, 2, 3, 4])
    expect(tour).toEqual([0, 1, 3, 4, 2])
  })
})

// ---------------------------------------------------------------
// End-to-end pipeline
// ---------------------------------------------------------------
describe('christofidesPipeline', () => {
  it('produces a valid Hamiltonian tour on every scenario', () => {
    for (const [key, inst] of Object.entries(SCENARIOS)) {
      const s = makeInitState(inst)
      expect(s.shortcutTour).toHaveLength(s.n)
      expect(s.shortcutTour.slice().sort((a, b) => a - b)).toEqual(
        Array.from({ length: s.n }, (_, i) => i),
      )
      // tour cost matches a recomputation
      expect(closedTourCost(s.shortcutTour, makeDist(inst.cities))).toBeCloseTo(s.tourCost, 6)
      // ratio ≤ 1.5 on every scenario
      expect(s.ratio, `${key} ratio must stay within the 1.5× bound`).toBeLessThanOrEqual(1.5)
    }
  })

  it('ratio stays ≤ 1.5 across random 10-city instances', () => {
    let seed = 12345
    const rnd = () => {
      seed = (seed * 1103515245 + 12345) % 2147483648
      return seed / 2147483648
    }
    for (let t = 0; t < 12; t++) {
      const cities: [number, number][] = []
      for (let i = 0; i < 10; i++) {
        cities.push([Math.round(20 + rnd() * 260), Math.round(20 + rnd() * 260)] as [number, number])
      }
      const s = makeInitState({ label: '', desc: '', cities })
      expect(s.ratio).toBeLessThanOrEqual(1.5)
      expect(s.ratio).toBeGreaterThanOrEqual(1.0)
    }
  })
})

// ---------------------------------------------------------------
// Pinned scenario behaviour
// ---------------------------------------------------------------
describe('scenario pins', () => {
  it('balanced: the classic ring at ~1.11×', () => {
    const s = makeInitState(SCENARIOS.balanced)
    expect(s.ratio).toBeCloseTo(1.107, 2)
  })

  it('near_optimal: the circle layout hits the exact optimum', () => {
    const s = makeInitState(SCENARIOS.near_optimal)
    expect(s.ratio).toBeCloseTo(1.0, 3)
    expect(s.tourCost).toBeCloseTo(s.opt, 3)
  })

  it('matching_heavy: the matching is about half the tour cost', () => {
    const s = makeInitState(SCENARIOS.matching_heavy)
    expect(s.matchingCost / s.tourCost).toBeGreaterThan(0.45)
  })

  it('worst_case: the ratio stretches toward the bound', () => {
    const s = makeInitState(SCENARIOS.worst_case)
    expect(s.ratio).toBeGreaterThan(1.35)
  })

  it('bruteForceOpt is a true lower bound on every scenario', () => {
    for (const inst of Object.values(SCENARIOS)) {
      const s = makeInitState(inst)
      expect(s.opt).toBeLessThanOrEqual(s.tourCost + 1e-6)
      expect(s.opt).toBeGreaterThan(0)
    }
  })
})

// ---------------------------------------------------------------
// Phase machine
// ---------------------------------------------------------------
describe('stepOnce phase machine', () => {
  const s0 = makeInitState(SCENARIOS.balanced)
  const mstSteps = s0.mstEdges.length // reveal all MST edges
  const matchingSteps = s0.matchingEdges.length
  const eulerSteps = s0.eulerCircuit.length - 1
  const shortcutSteps = s0.eulerCircuit.length

  it('starts in the mst phase with nothing revealed', () => {
    expect(s0.phase).toBe('mst')
    expect(s0.mstRevealed).toBe(0)
    expect(s0.step).toBe(0)
  })

  it('mst phase reveals one edge per step, then advances to odd', () => {
    let s = s0
    for (let i = 0; i < mstSteps; i++) {
      s = stepOnce(s)
      expect(s.phase).toBe('mst')
      expect(s.mstRevealed).toBe(i + 1)
    }
    s = stepOnce(s)
    expect(s.phase).toBe('odd')
  })

  it('odd → matching → euler → shortcut → done in order', () => {
    let s = s0
    // walk the whole machine to completion
    let guard = mstSteps + 5 + matchingSteps + 1 + eulerSteps + 1 + shortcutSteps + 2
    while (s.phase !== 'done' && guard-- > 0) s = stepOnce(s)
    expect(s.phase).toBe('done')
    expect(s.kept).toEqual(s.shortcutTour)
    // skipped visits recorded
    expect(s.skipped.length).toBeGreaterThan(0)
  })

  it('shortcut phase builds the tour by skipping repeats', () => {
    let s = s0
    let guard = mstSteps + 2 + matchingSteps + 2 + eulerSteps
    while (s.phase !== 'shortcut' && guard-- > 0) s = stepOnce(s)
    expect(s.phase).toBe('shortcut')
    const initial = s
    s = stepOnce(s) // first shortcut step processes circuit[1]
    expect(s.shortcutStep).toBe(initial.shortcutStep + 1)
  })

  it('done is a no-op apart from the step counter', () => {
    let s = s0
    let guard = mstSteps + 5 + matchingSteps + 1 + eulerSteps + 1 + shortcutSteps + 2
    while (s.phase !== 'done' && guard-- > 0) s = stepOnce(s)
    const again = stepOnce(s)
    expect(again.phase).toBe('done')
    expect(again.tourCost).toBe(s.tourCost)
    expect(again.step).toBe(s.step + 1)
  })
})

// ---------------------------------------------------------------
// Purity & determinism
// ---------------------------------------------------------------
describe('purity & determinism', () => {
  it('stepOnce never mutates its input', () => {
    const s = makeInitState(SCENARIOS.worst_case)
    const snapshot = structuredClone(s)
    let cur = s
    for (let i = 0; i < 15; i++) cur = stepOnce(cur)
    expect(s).toEqual(snapshot)
  })

  it('two runs from the same instance produce identical sequences', () => {
    const a = makeInitState(SCENARIOS.matching_heavy)
    const b = makeInitState(SCENARIOS.matching_heavy)
    for (let i = 0; i < 12; i++) {
      const na = stepOnce(a)
      const nb = stepOnce(b)
      expect(na).toEqual(nb)
      Object.assign(a, na)
      Object.assign(b, nb)
    }
  })

  it('primitives are pure — hierholzer does not mutate its input adjacency', () => {
    const dist = makeDist([[0, 0], [1, 0], [2, 0], [1, 1]])
    const mst = primMst(4, dist)
    const odd = oddDegreeNodes(mst, 4)
    const matching = greedyMatching(odd, dist, 4)
    const adj = buildMultigraph(4, mst, matching)
    const snapshot = structuredClone(adj)
    hierholzer(adj, 0)
    expect(adj).toEqual(snapshot)
  })
})

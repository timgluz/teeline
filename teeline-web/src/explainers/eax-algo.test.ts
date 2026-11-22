import { describe, it, expect } from 'vitest'
import {
  CITIES,
  N_CITIES,
  PARENT_A,
  PARENT_B,
  PINNED_CYCLE,
  DEFAULT_SEED,
  abCycles,
  applyESet,
  bestTwoOpt,
  componentsFromEdges,
  edgeKey,
  inheritance,
  makeEdge,
  selectRand,
  sharedEdgeCount,
  simulate,
  tourEdgeKeys,
  tourEdges,
  tourLength,
  unionEdges,
} from './eax-algo'

function canonicalCycle(vertices: number[]): number[] {
  const rotations: number[][] = []
  for (let i = 0; i < vertices.length; i++) {
    rotations.push([...vertices.slice(i), ...vertices.slice(0, i)])
  }
  const reversed = [...vertices].reverse()
  for (let i = 0; i < reversed.length; i++) {
    rotations.push([...reversed.slice(i), ...reversed.slice(0, i)])
  }
  return rotations.sort((a, b) => (a.join(',') < b.join(',') ? -1 : 1))[0]
}

describe('pinned fixture', () => {
  it('has 16 cities and both parents are permutations', () => {
    expect(CITIES).toHaveLength(16)
    expect(N_CITIES).toBe(16)
    const sorted = (tour: number[]) => [...tour].sort((a, b) => a - b)
    expect(sorted(PARENT_A)).toEqual(Array.from({ length: 16 }, (_, i) => i))
    expect(sorted(PARENT_B)).toEqual(Array.from({ length: 16 }, (_, i) => i))
  })

  it('shares 11 edges (68.75%)', () => {
    expect(sharedEdgeCount(PARENT_A, PARENT_B)).toBe(11)
    expect(sharedEdgeCount(PARENT_A, PARENT_B) / N_CITIES).toBeCloseTo(0.6875)
  })

  it('has difference degree 2 at every active city (unique decomposition)', () => {
    const aOnly = new Set(
      tourEdges(PARENT_A)
        .map(edgeKey)
        .filter((k) => !tourEdgeKeys(PARENT_B).has(k)),
    )
    const bOnly = new Set(
      tourEdges(PARENT_B)
        .map(edgeKey)
        .filter((k) => !tourEdgeKeys(PARENT_A).has(k)),
    )
    for (let v = 0; v < N_CITIES; v++) {
      const count = [...aOnly, ...bOnly].filter((key) => {
        const [u, w] = key.split('-').map(Number)
        return u === v || w === v
      }).length
      expect([0, 2]).toContain(count)
    }
  })

  it('decomposes into exactly two even non-trivial cycles', () => {
    const cycles = abCycles(PARENT_A, PARENT_B)
    expect(cycles).toHaveLength(2)
    for (const cycle of cycles) {
      expect(cycle.vertices.length % 2).toBe(0)
      expect(cycle.vertices.length).toBeGreaterThanOrEqual(4)
      expect(cycle.aEdges).toHaveLength(cycle.vertices.length / 2)
      expect(cycle.bEdges).toHaveLength(cycle.vertices.length / 2)
      // alternating origin
      for (let i = 0; i < cycle.vertices.length; i++) {
        const from = cycle.vertices[i]
        const to = cycle.vertices[(i + 1) % cycle.vertices.length]
        const inA = tourEdgeKeys(PARENT_A).has(edgeKey(makeEdge(from, to)))
        const inB = tourEdgeKeys(PARENT_B).has(edgeKey(makeEdge(from, to)))
        expect(inA !== inB).toBe(true)
      }
    }
    const canon = cycles.map((c) => canonicalCycle(c.vertices).join(',')).sort()
    expect(canon).toEqual(
      [
        canonicalCycle([0, 1, 4, 5]).join(','),
        canonicalCycle([2, 3, 8, 9, 7, 6]).join(','),
      ].sort(),
    )
  })

  it('C1 alone yields two subtours, C2 alone yields one, both yield Parent B', () => {
    const cycles = abCycles(PARENT_A, PARENT_B)
    const c1Index = cycles.findIndex(
      (c) =>
        canonicalCycle(c.vertices).join(',') ===
        canonicalCycle([0, 1, 4, 5]).join(','),
    )
    const c2Index = cycles.findIndex(
      (c) =>
        canonicalCycle(c.vertices).join(',') ===
        canonicalCycle([2, 3, 8, 9, 7, 6]).join(','),
    )
    expect(c1Index).toBeGreaterThanOrEqual(0)
    expect(c2Index).toBeGreaterThanOrEqual(0)

    const c1Comps = componentsFromEdges(applyESet(PARENT_A, cycles, [c1Index]))
    expect(c1Comps).toHaveLength(2)
    expect(c1Comps.map((c) => c.length).sort((a, b) => a - b)).toEqual([4, 12])

    const c2Comps = componentsFromEdges(applyESet(PARENT_A, cycles, [c2Index]))
    expect(c2Comps).toHaveLength(1)

    const both = applyESet(PARENT_A, cycles, [c1Index, c2Index])
    expect([...both].sort()).toEqual([...tourEdgeKeys(PARENT_B)].sort())
  })
})

describe('union graph', () => {
  it('classifies shared / A-only / B-only and excludes shared from the difference', () => {
    const union = unionEdges(PARENT_A, PARENT_B)
    expect(union).toHaveLength(21)
    expect(union.filter((e) => e.kind === 'shared')).toHaveLength(11)
    expect(union.filter((e) => e.kind === 'a-only')).toHaveLength(5)
    expect(union.filter((e) => e.kind === 'b-only')).toHaveLength(5)
  })
})

describe('E-set selection', () => {
  it('1-AB pins the subtour-producing cycle', () => {
    const sim = simulate([PINNED_CYCLE])
    expect(sim.selected).toEqual([PINNED_CYCLE])
    expect(sim.subtours).toHaveLength(2)
  })

  it('Rand is reproducible for a seed and never empty', () => {
    expect(selectRand(2, DEFAULT_SEED)).toEqual(selectRand(2, DEFAULT_SEED))
    for (let seed = 0; seed < 50; seed++) {
      const selected = selectRand(2, seed)
      expect(selected.length).toBeGreaterThanOrEqual(1)
      expect(selected.every((id) => id === 0 || id === 1)).toBe(true)
    }
  })

  it('manual selection toggles a cycle', () => {
    const sim = simulate([0, 1])
    expect(sim.stats.eSetSize).toBe(2)
    const toggled = simulate([1])
    expect(toggled.stats.eSetSize).toBe(1)
  })
})

describe('symmetric difference', () => {
  it('preserves degree two at every city', () => {
    const cycles = abCycles(PARENT_A, PARENT_B)
    for (const selected of [[0], [1], [0, 1]]) {
      const edges = applyESet(PARENT_A, cycles, selected)
      const degree = new Array(N_CITIES).fill(0)
      for (const key of edges) {
        const [u, v] = key.split('-').map(Number)
        degree[u]++
        degree[v]++
      }
      for (const d of degree) expect(d).toBe(2)
    }
  })

  it('selecting every cycle reproduces Parent B exactly', () => {
    const cycles = abCycles(PARENT_A, PARENT_B)
    const edges = applyESet(
      PARENT_A,
      cycles,
      cycles.map((c) => c.id),
    )
    expect([...edges].sort()).toEqual([...tourEdgeKeys(PARENT_B)].sort())
  })
})

describe('patching', () => {
  it('merges the small subtour with the pinned cheapest bridge', () => {
    const sim = simulate([PINNED_CYCLE])
    expect(sim.steps).toHaveLength(1)
    const step = sim.steps[0]
    expect(step.small).toHaveLength(4)
    expect(
      new Set([edgeKey(step.removed[0]), edgeKey(step.removed[1])]),
    ).toEqual(new Set(['3-4', '6-7']))
    expect(new Set([edgeKey(step.added[0]), edgeKey(step.added[1])])).toEqual(
      new Set(['3-6', '4-7']),
    )
    expect(step.delta).toBeCloseTo(-71.265, 2)
  })

  it('ends in one Hamiltonian tour', () => {
    const sim = simulate([PINNED_CYCLE])
    expect(sim.stats.bridges).toBe(1)
    expect(sim.child).toHaveLength(N_CITIES)
    expect([...sim.child].sort((a, b) => a - b)).toEqual(
      Array.from({ length: N_CITIES }, (_, i) => i),
    )
    expect(tourLength(sim.child)).toBeCloseTo(993.257, 2)
  })

  it('needs no patching when the E-set already forms one tour', () => {
    const cycles = abCycles(PARENT_A, PARENT_B)
    const c2 = cycles.findIndex(
      (c) =>
        canonicalCycle(c.vertices).join(',') ===
        canonicalCycle([2, 3, 8, 9, 7, 6]).join(','),
    )
    const sim = simulate([c2])
    expect(sim.subtours).toHaveLength(1)
    expect(sim.steps).toHaveLength(0)
    expect(sim.frames.some((f) => f.kind === 'no-patch')).toBe(true)
  })
})

describe('2-opt preview', () => {
  it('finds the pinned best move on the child', () => {
    const sim = simulate([PINNED_CYCLE])
    const move = bestTwoOpt(sim.child)
    expect(move).not.toBeNull()
    expect(
      new Set([edgeKey(move!.removed[0]), edgeKey(move!.removed[1])]),
    ).toEqual(new Set(['0-5', '1-2']))
    expect(new Set([edgeKey(move!.added[0]), edgeKey(move!.added[1])])).toEqual(
      new Set(['0-2', '1-5']),
    )
    expect(move!.delta).toBeCloseTo(-5.21, 2)
  })

  it('returns null for a 2-opt-local tour', () => {
    const local = [0, 1, 15, 14, 13, 12, 11, 10, 9, 8, 7, 4, 3, 6, 5, 2]
    expect(bestTwoOpt(local)).toBeNull()
  })
})

describe('statistics', () => {
  it('reports the pinned costs and edge inheritance', () => {
    const sim = simulate([PINNED_CYCLE])
    expect(sim.stats.sharedEdges).toBe(11)
    expect(sim.stats.sharedRatio).toBeCloseTo(0.6875)
    expect(sim.stats.cycleCount).toBe(2)
    expect(sim.stats.cycleLengths.sort((a, b) => a - b)).toEqual([4, 6])
    expect(sim.stats.eSetSize).toBe(1)
    expect(sim.stats.subtours).toBe(2)
    expect(sim.stats.bridges).toBe(1)
    expect(sim.stats.costA).toBeCloseTo(1063.458, 2)
    expect(sim.stats.costB).toBeCloseTo(1142.741, 2)
    expect(sim.stats.costChild).toBeCloseTo(993.257, 2)
    expect(sim.stats.costChild).toBeLessThan(sim.stats.costA)
    expect(sim.stats.costChild).toBeLessThan(sim.stats.costB)
    expect(sim.stats.inheritance).toEqual({
      shared: 10,
      fromA: 2,
      fromB: 2,
      new: 2,
    })
  })

  it('child inheritance is a partition of the 16 edges', () => {
    const sim = simulate([PINNED_CYCLE])
    const i = inheritance(sim.child)
    expect(i.shared + i.fromA + i.fromB + i.new).toBe(N_CITIES)
  })
})

describe('determinism', () => {
  it('same selection yields the same child, frame sequence, and stats', () => {
    const a = simulate([PINNED_CYCLE])
    const b = simulate([PINNED_CYCLE])
    expect(a.child).toEqual(b.child)
    expect(a.frames).toEqual(b.frames)
    expect(a.stats).toEqual(b.stats)
  })

  it('the frame sequence covers all six stages', () => {
    const sim = simulate([PINNED_CYCLE])
    expect(new Set(sim.frames.map((f) => f.stage))).toEqual(
      new Set([1, 2, 3, 4, 5, 6]),
    )
  })
})

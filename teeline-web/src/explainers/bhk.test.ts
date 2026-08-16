import { describe, it, expect } from 'vitest'
import {
  SCENARIOS, makeInitState, stepOnce, makeDm, popcount, computeFillOrder,
} from './bhk-algo'
import type { SimState } from './bhk-algo'

// Brute-force optimum over the same instance (fixed start at city 0).
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
  let guard = 1000
  while (s.phase !== 'done' && guard-- > 0) s = stepOnce(s)
  expect(s.phase).toBe('done')
  return s
}

// ---------------------------------------------------------------
describe('primitives', () => {
  it('popcount counts set bits', () => {
    expect(popcount(0)).toBe(0)
    expect(popcount(1)).toBe(1)
    expect(popcount(7)).toBe(3)
    expect(popcount(31)).toBe(5)
  })

  it('fill order goes by subset size ascending, then mask, then row', () => {
    const order = computeFillOrder(3)
    const sizes = order.map((o) => popcount(o.mask))
    for (let k = 1; k < sizes.length; k++) {
      expect(sizes[k]).toBeGreaterThanOrEqual(sizes[k - 1])
    }
    // every (mask, row in mask) cell appears exactly once
    const keys = new Set(order.map((o) => `${o.mask}:${o.row}`))
    expect(keys.size).toBe(order.length)
    // sizes 2..3 only: 3×2 + 1×3 = 9 cells (size-1 base cells are pre-filled)
    expect(order).toHaveLength(9)
  })
})

// ---------------------------------------------------------------
describe('makeInitState', () => {
  it('sets the base cells dp[{i}][i] = d(0, i+1)', () => {
    const s = makeInitState(SCENARIOS.grid_6)
    for (let row = 0; row < s.m; row++) {
      expect(s.table[row][1 << row]).toBeCloseTo(s.dm[0][row + 1], 10)
    }
    expect(s.phase).toBe('forward')
    expect(s.fillPtr).toBe(0)
  })
})

// ---------------------------------------------------------------
describe('stepOnce forward', () => {
  it('fills one cell per step and records the predecessor', () => {
    let s = makeInitState(SCENARIOS.grid_6)
    s = stepOnce(s) // first size-2 cell
    expect(s.fillPtr).toBe(1)
    expect(s.lastEvent).toContain('via city')
    const cell = s.fillOrder[0]
    expect(s.table[cell.row][cell.mask]).not.toBe(Infinity)
    expect(s.pred[cell.row][cell.mask]).toBeGreaterThanOrEqual(0)
  })
})

// ---------------------------------------------------------------
describe('readback', () => {
  it('reveals the route from the end back to the start', () => {
    const s0 = makeInitState(SCENARIOS.grid_6)
    let s = s0
    let guard = 100
    while (s.phase !== 'readback' && guard-- > 0) s = stepOnce(s)
    expect(s.phase).toBe('readback')
    expect(s.optCost).not.toBeNull()
    expect(s.route).toHaveLength(6)
    expect(s.route![0]).toBe(0)
    // readback seeds the end city
    expect(s.readback[0]).toBe(s.route![5])
    // stepping reveals the rest in reverse order
    const next = stepOnce(s)
    expect(next.readback).toHaveLength(2)
    expect(next.readback[1]).toBe(s.route![4])
  })
})

// ---------------------------------------------------------------
describe('correctness', () => {
  it('every scenario finds the exact optimum and a valid route', () => {
    for (const [key, sc] of Object.entries(SCENARIOS)) {
      const s = runToDone(sc)
      expect(s.optCost, `${key} must match brute force`).toBeCloseTo(bruteForce(sc.cities), 6)
      expect([...s.route!].sort((a, b) => a - b), `${key} route must be a permutation`).toEqual(
        Array.from({ length: sc.cities.length }, (_, i) => i),
      )
    }
  })

  it('computeRoute on the finished table matches the closed tour length', () => {
    const s = runToDone(SCENARIOS.clusters_6)
    const dm = s.dm
    const route = s.route!
    let d = 0
    for (let k = 0; k < route.length; k++) d += dm[route[k]][route[(k + 1) % route.length]]
    expect(d).toBeCloseTo(s.optCost!, 6)
  })
})

// ---------------------------------------------------------------
describe('purity & determinism', () => {
  it('stepOnce never mutates its input', () => {
    const s = makeInitState(SCENARIOS.clusters_6)
    const snapshot = structuredClone(s)
    let cur = s
    for (let i = 0; i < 20; i++) cur = stepOnce(cur)
    expect(s).toEqual(snapshot)
  })

  it('two runs from the same scenario produce identical sequences', () => {
    const a = makeInitState(SCENARIOS.grid_6)
    const b = makeInitState(SCENARIOS.grid_6)
    for (let i = 0; i < 20; i++) {
      const na = stepOnce(a)
      const nb = stepOnce(b)
      expect(na).toEqual(nb)
      Object.assign(a, na)
      Object.assign(b, nb)
    }
  })
})

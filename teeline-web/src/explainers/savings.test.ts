import { describe, it, expect } from 'vitest'
import {
  N_CITIES, hubIndex,
  components, makeInitState, stepOnce, runToCompletion,
} from './savings-algo'

describe('hubIndex', () => {
  it('returns a valid city index', () => {
    const h = hubIndex()
    expect(h).toBeGreaterThanOrEqual(0)
    expect(h).toBeLessThan(N_CITIES)
  })

  it('is deterministic (same cities = same hub)', () => {
    expect(hubIndex()).toBe(hubIndex())
  })
})

describe('makeInitState', () => {
  it('starts with every city in its own component', () => {
    const s = makeInitState()
    expect(components(s.parent)).toHaveLength(N_CITIES)
  })

  it('starts with zero accepted edges and zero degree everywhere', () => {
    const s = makeInitState()
    expect(s.accepted).toHaveLength(0)
    expect(s.degree.every(d => d === 0)).toBe(true)
    expect(s.done).toBe(false)
    expect(s.step).toBe(0)
  })

  it('sorted edges are descending by savings (val field)', () => {
    const s = makeInitState()
    const e = s.sortedEdges
    for (let i = 1; i < e.length; i++) {
      expect(e[i].val).toBeLessThanOrEqual(e[i - 1].val + 1e-9)
    }
  })
})

describe('stepOnce', () => {
  it('the first accepted edge has the highest saving', () => {
    const s = stepOnce(makeInitState())
    expect(s.accepted).toHaveLength(1)
    expect(s.lastEvent).toBe('accepted')
  })

  it('never lets a city exceed degree 2', () => {
    let s = makeInitState()
    for (let i = 0; i < 200 && !s.done; i++) s = stepOnce(s)
    for (const d of s.degree) expect(d).toBeLessThanOrEqual(2)
  })

  it('rejected edges are tagged with a valid reason', () => {
    let s = makeInitState()
    for (let i = 0; i < 200 && !s.done; i++) s = stepOnce(s)
    for (const r of s.rejected) {
      expect(['degree', 'cycle']).toContain(r.reason)
    }
  })
})

describe('runToCompletion (the Rust select_edges invariants)', () => {
  it('always terminates', () => {
    const s = runToCompletion(makeInitState())
    expect(s.done).toBe(true)
  })

  it('places exactly n accepted edges', () => {
    const s = runToCompletion(makeInitState())
    expect(s.accepted).toHaveLength(N_CITIES)
  })

  it('leaves every city with degree exactly 2', () => {
    const s = runToCompletion(makeInitState())
    for (const d of s.degree) expect(d).toBe(2)
  })

  it('collapses to a single connected component', () => {
    const s = runToCompletion(makeInitState())
    expect(components(s.parent)).toHaveLength(1)
  })

  it('produces a valid Hamiltonian path visiting every city once', () => {
    const s = runToCompletion(makeInitState())
    expect(s.tour).not.toBeNull()
    expect(s.tour!).toHaveLength(N_CITIES)
    expect(s.tour!.slice().sort((a, b) => a - b)).toEqual(
      Array.from({ length: N_CITIES }, (_, i) => i),
    )
  })

  it('the last accepted edge is the closing edge', () => {
    const s = runToCompletion(makeInitState())
    expect(s.lastEvent).toBe('closing')
  })
})

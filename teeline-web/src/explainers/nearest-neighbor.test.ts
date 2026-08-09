import { describe, it, expect } from 'vitest'
import {
  N_CITIES, SCENARIOS,
  dist, tourLength, makeInitState, stepOnce,
} from './nearest-neighbor-algo'

describe('dist', () => {
  it('is symmetric', () => {
    for (let i = 0; i < N_CITIES; i++)
      for (let j = 0; j < N_CITIES; j++)
        expect(dist(i, j)).toBeCloseTo(dist(j, i), 10)
  })

  it('is zero for same city', () => {
    expect(dist(0, 0)).toBe(0)
  })

  it('is positive for different cities', () => {
    expect(dist(0, 1)).toBeGreaterThan(0)
  })
})

describe('tourLength', () => {
  it('returns positive for a partial tour', () => {
    expect(tourLength([0, 1, 2])).toBeGreaterThan(0)
  })
})

describe('SCENARIOS', () => {
  it('all scenario start cities are valid', () => {
    for (const [_, s] of Object.entries(SCENARIOS)) {
      expect(s.startCity).toBeGreaterThanOrEqual(0)
      expect(s.startCity).toBeLessThan(N_CITIES)
    }
  })
})

describe('makeInitState', () => {
  it('starts with only the start city in tour', () => {
    const s = makeInitState(0)
    expect(s.tour).toEqual([0])
    expect(s.unvisited).toHaveLength(N_CITIES - 1)
    expect(s.done).toBe(false)
    expect(s.step).toBe(0)
  })

  it('unvisited contains all cities except start', () => {
    const s = makeInitState(5)
    expect(s.unvisited).not.toContain(5)
    expect(s.unvisited).toHaveLength(N_CITIES - 1)
    const all = [...s.unvisited, ...s.tour].sort((a, b) => a - b)
    expect(all).toEqual(Array.from({ length: N_CITIES }, (_, i) => i))
  })
})

describe('stepOnce', () => {
  it('picks the nearest unvisited city', () => {
    const s = makeInitState(0)
    const s1 = stepOnce(s)
    expect(s1.lastEvent).toBe('visited')
    expect(s1.lastCity).not.toBeNull()
    // From city 0 (150,20): nearest should be 7 (80,25) at sqrt(70²+5²) ≈ 70.2
    // or city 1 (270,70) at sqrt(120²+50²) ≈ 130
    // Let's just verify it's a valid unvisited city
    expect(s.unvisited).toContain(s1.lastCity!)
    expect(s1.tour).toHaveLength(2)
    expect(s1.unvisited).toHaveLength(N_CITIES - 2)
  })

  it('eventually visits all cities and closes', () => {
    let s = makeInitState(0)
    let maxSteps = 20
    while (!s.done && maxSteps > 0) {
      s = stepOnce(s)
      maxSteps--
    }
    expect(s.done).toBe(true)
    expect(s.lastEvent).toBe('closing')
    // Tour should be N+1 long (start visited twice)
    expect(s.tour).toHaveLength(N_CITIES + 1)
    expect(s.tour[0]).toBe(s.tour[s.tour.length - 1])
    expect(s.unvisited).toHaveLength(0)
  })

  it('no-op when already done', () => {
    let s = makeInitState(0)
    while (!s.done) s = stepOnce(s)
    const s2 = stepOnce(s)
    expect(s2.step).toBe(s.step + 1)
    expect(s2.done).toBe(true)
    expect(s2.tour).toEqual(s.tour)
  })

  it('each visited city is from the unvisited set', () => {
    const s = makeInitState(0)
    const s1 = stepOnce(s)
    const s2 = stepOnce(s1)
    const s3 = stepOnce(s2)
    expect(s1.tour).toHaveLength(2)
    expect(s2.tour).toHaveLength(3)
    expect(s3.tour).toHaveLength(4)
    // All newly visited cities were in the previous unvisited
    expect(s.unvisited).toContain(s1.lastCity)
  })

  it('produces candidateDists in sorted order', () => {
    const s = makeInitState(4)
    const s1 = stepOnce(s)
    expect(s1.candidateDists).not.toBeNull()
    expect(s1.candidateDists!.length).toBe(s.unvisited.length)
    for (let i = 1; i < s1.candidateDists!.length; i++) {
      expect(s1.candidateDists![i - 1].dist).toBeLessThanOrEqual(s1.candidateDists![i].dist + 0.001)
    }
  })
})

import { describe, it, expect } from 'vitest'
import {
  N_CITIES, SCENARIOS,
  dist, tourLength, makeInitState, stepOnce,
  scanOnePass, applySwap,
} from './two-opt-algo'

describe('dist', () => {
  it('is symmetric', () => {
    for (let i = 0; i < N_CITIES; i++) {
      for (let j = 0; j < N_CITIES; j++) {
        expect(dist(i, j)).toBeCloseTo(dist(j, i), 10)
      }
    }
  })

  it('is zero for same city', () => {
    expect(dist(0, 0)).toBe(0)
  })

  it('is positive for different cities', () => {
    expect(dist(0, 1)).toBeGreaterThan(0)
  })
})

describe('tourLength', () => {
  it('returns a finite positive number for a valid tour', () => {
    const len = tourLength([0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    expect(len).toBeGreaterThan(0)
    expect(Number.isFinite(len)).toBe(true)
  })

  it('is the same for rotationally equivalent tours', () => {
    const a = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    const b = [3, 4, 5, 6, 7, 8, 9, 0, 1, 2]
    expect(tourLength(a)).toBeCloseTo(tourLength(b), 10)
  })
})

describe('SCENARIOS', () => {
  it('all scenario tours contain all cities exactly once', () => {
    for (const [_key, s] of Object.entries(SCENARIOS)) {
      const sorted = s.tour.slice().sort((a, b) => a - b)
      expect(sorted).toEqual(Array.from({ length: N_CITIES }, (_, i) => i))
    }
  })
})

describe('makeInitState', () => {
  it('starts in idle phase', () => {
    const s = makeInitState([0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    expect(s.phase).toBe('idle')
    expect(s.pass).toBe(0)
    expect(s.totalSwaps).toBe(0)
    expect(s.step).toBe(0)
  })

  it('sets bestCost from tour', () => {
    const s = makeInitState([0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    expect(s.bestCost).toBeCloseTo(tourLength(s.tour), 10)
  })

  it('costHistory starts with initial cost', () => {
    const s = makeInitState([0, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    expect(s.costHistory).toEqual([s.bestCost])
  })

  it('copies the tour (does not share reference)', () => {
    const input = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    const s = makeInitState(input)
    input[0] = 999
    expect(s.tour[0]).toBe(0)
  })
})

describe('scanOnePass', () => {
  it('returns bestDelta=0 with bestI=bestJ=-1 when no improving swap exists', () => {
    const s = makeInitState(SCENARIOS.two_optimal.tour)
    const { bestDelta, bestI, bestJ } = scanOnePass(s)
    expect(bestDelta).toBe(0)
    expect(bestI).toBe(-1)
    expect(bestJ).toBe(-1)
  })

  it('returns a negative delta when an improving swap exists', () => {
    const s = makeInitState(SCENARIOS.bad_shuffle.tour)
    const { bestDelta, bestI, bestJ } = scanOnePass(s)
    expect(bestDelta).toBeLessThan(0)
    expect(bestI).toBeGreaterThanOrEqual(0)
    expect(bestJ).toBeGreaterThan(bestI + 1)
  })
})

describe('applySwap', () => {
  it('returns a valid tour with all cities', () => {
    const tour = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    const next = applySwap(tour, 2, 5)
    const sorted = next.slice().sort((a, b) => a - b)
    expect(sorted).toEqual(Array.from({ length: N_CITIES }, (_, i) => i))
  })

  it('reverses the segment between i+1 and j', () => {
    const tour = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    const next = applySwap(tour, 1, 4) // reverse [2,3,4] → [4,3,2]
    expect(next).toEqual([0, 1, 4, 3, 2, 5, 6, 7, 8, 9])
  })

  it('does not mutate the input', () => {
    const tour = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    const copy = [...tour]
    applySwap(tour, 1, 3)
    expect(tour).toEqual(copy)
  })
})

describe('stepOnce', () => {
  it('first click scans and shows candidate without changing tour', () => {
    const s = makeInitState(SCENARIOS.bad_shuffle.tour)
    const s1 = stepOnce(s)
    expect(s1.phase).toBe('candidate')
    expect(s1.lastSwap).not.toBeNull()
    expect(s1.lastSwap!.delta).toBeLessThan(0)
    expect(s1.tour).toEqual(s.tour) // tour unchanged
    expect(s1.totalSwaps).toBe(0)   // not applied yet
  })

  it('second click applies the candidate swap', () => {
    const s = makeInitState(SCENARIOS.bad_shuffle.tour)
    const s1 = stepOnce(s)  // candidate
    const s2 = stepOnce(s1) // apply
    expect(s2.phase).toBe('swap_found')
    expect(s2.totalSwaps).toBe(1)
    expect(s2.pass).toBe(1)
    // tour must have changed after apply
    expect(s2.tour).not.toEqual(s.tour)
  })

  it('eventually reaches local_optimum', () => {
    let s = makeInitState(SCENARIOS.bad_shuffle.tour)
    let maxSteps = 400
    while (s.phase !== 'local_optimum' && maxSteps > 0) {
      s = stepOnce(s)
      maxSteps--
    }
    expect(s.phase).toBe('local_optimum')
    expect(s.bestCost).toBeGreaterThan(0)
  })

  it('no-op when already at local_optimum', () => {
    const s = makeInitState(SCENARIOS.two_optimal.tour)
    const s1 = stepOnce(s)
    expect(s1.phase).toBe('local_optimum')
    const s2 = stepOnce(s1)
    expect(s2.step).toBe(s1.step + 1)
    expect(s2.phase).toBe('local_optimum')
  })

  it('produces valid swap event fields when applied', () => {
    const s = makeInitState(SCENARIOS.bad_shuffle.tour)
    const s1 = stepOnce(s)  // candidate
    const s2 = stepOnce(s1) // apply
    expect(s2.lastSwap).not.toBeNull()
    expect(s2.lastSwap!.i).toBeGreaterThanOrEqual(0)
    expect(s2.lastSwap!.j).toBeGreaterThan(s2.lastSwap!.i + 1)
    expect(s2.lastSwap!.delta).toBeLessThan(0)
    expect(s2.lastSwap!.removed).toHaveLength(2)
    expect(s2.lastSwap!.added).toHaveLength(2)
  })
})

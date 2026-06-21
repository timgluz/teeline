import { describe, it, expect } from 'vitest'
import {
  N_CITIES, tourLength, swapSequence, applySwaps,
  makeInitState, stepEpoch,
} from './pso-algo'

describe('tourLength', () => {
  it('is positive for a multi-city tour', () => {
    const tour = Array.from({ length: N_CITIES }, (_, i) => i)
    expect(tourLength(tour)).toBeGreaterThan(0)
  })

  it('returns 0 for a single-city tour', () => {
    expect(tourLength([0])).toBe(0)
  })
})

describe('swapSequence', () => {
  it('returns empty for identical tours', () => {
    expect(swapSequence([0, 1, 2, 3], [0, 1, 2, 3])).toEqual([])
  })

  it('applying result of swapSequence(a, b) to a yields b', () => {
    const a = [0, 3, 1, 2]
    const b = [0, 1, 2, 3]
    const swaps = swapSequence(a, b)
    expect(applySwaps(a, swaps, swaps.length)).toEqual(b)
  })

  it('works for fully reversed tours', () => {
    const a = [3, 2, 1, 0]
    const b = [0, 1, 2, 3]
    const swaps = swapSequence(a, b)
    expect(applySwaps(a, swaps, swaps.length)).toEqual(b)
  })
})

describe('applySwaps', () => {
  it('returns original tour when n=0', () => {
    expect(applySwaps([0, 1, 2, 3], [[0, 1], [1, 2]], 0)).toEqual([0, 1, 2, 3])
  })

  it('does not mutate the input tour', () => {
    const tour = [0, 1, 2, 3]
    applySwaps(tour, [[0, 1]], 1)
    expect(tour).toEqual([0, 1, 2, 3])
  })

  it('swaps adjacent elements in correct order', () => {
    expect(applySwaps([0, 1, 2, 3], [[1, 2]], 1)).toEqual([0, 2, 1, 3])
  })

  it('clamps to swaps.length when n exceeds array', () => {
    const swaps: [number, number][] = [[0, 1]]
    expect(applySwaps([0, 1, 2], swaps, 99)).toEqual([1, 0, 2])
  })
})

describe('makeInitState', () => {
  it('creates the requested number of particles', () => {
    expect(makeInitState(5).particles).toHaveLength(5)
  })

  it('each particle position is a valid permutation of 0..N_CITIES-1', () => {
    const expected = Array.from({ length: N_CITIES }, (_, i) => i).sort((a, b) => a - b)
    for (const p of makeInitState(4).particles) {
      expect(p.position.slice().sort((a, b) => a - b)).toEqual(expected)
    }
  })

  it('pbest equals initial position and pbest_cost equals cost', () => {
    for (const p of makeInitState(3).particles) {
      expect(p.pbest).toEqual(p.position)
      expect(p.pbest_cost).toBeCloseTo(p.cost, 1)
    }
  })

  it('gbest_cost equals minimum particle cost', () => {
    const s = makeInitState(6)
    const min = Math.min(...s.particles.map(p => p.cost))
    expect(s.gbest_cost).toBeCloseTo(min, 1)
  })

  it('epoch is 0 and w is W_MAX (0.9)', () => {
    const s = makeInitState(3)
    expect(s.epoch).toBe(0)
    expect(s.w).toBeCloseTo(0.9, 5)
  })
})

describe('stepEpoch', () => {
  it('increments epoch by 1', () => {
    expect(stepEpoch(makeInitState(3)).epoch).toBe(1)
  })

  it('gbest_cost never increases across 20 epochs', () => {
    let s = makeInitState(5)
    for (let i = 0; i < 20; i++) {
      const prev = s.gbest_cost
      s = stepEpoch(s)
      expect(s.gbest_cost).toBeLessThanOrEqual(prev + 0.001)
    }
  })

  it('each particle position remains a valid permutation', () => {
    const expected = Array.from({ length: N_CITIES }, (_, i) => i).sort((a, b) => a - b)
    let s = makeInitState(4)
    s = stepEpoch(s)
    for (const p of s.particles) {
      expect(p.position.slice().sort((a, b) => a - b)).toEqual(expected)
    }
  })

  it('populates lastBreakdown with non-negative counts', () => {
    const next = stepEpoch(makeInitState(3))
    expect(next.lastBreakdown).not.toBeNull()
    expect(next.lastBreakdown!.inertia).toBeGreaterThanOrEqual(0)
    expect(next.lastBreakdown!.cognitive).toBeGreaterThanOrEqual(0)
    expect(next.lastBreakdown!.social).toBeGreaterThanOrEqual(0)
    expect(next.lastBreakdown!.applied).toBeGreaterThanOrEqual(0)
  })

  it('w decreases monotonically across epochs up to MAX_DISPLAY_EPOCHS', () => {
    let s = makeInitState(2)
    let prevW = s.w
    for (let i = 0; i < 50; i++) {
      s = stepEpoch(s)
      expect(s.w).toBeLessThanOrEqual(prevW + 0.0001)
      prevW = s.w
    }
  })
})

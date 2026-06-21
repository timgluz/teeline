import { describe, it, expect } from 'vitest'
import {
  N_CITIES, tourLength, twoOptSwap, moveKey,
  sampleNeighbours, makeInitState, stepOnce,
} from './tabu-algo'

describe('tourLength', () => {
  it('is positive for a full tour', () => {
    expect(tourLength(Array.from({ length: N_CITIES }, (_, i) => i))).toBeGreaterThan(0)
  })
  it('returns 0 for a single city', () => {
    expect(tourLength([0])).toBe(0)
  })
})

describe('twoOptSwap', () => {
  it('reverses segment between i and j', () => {
    expect(twoOptSwap([0, 1, 2, 3, 4], 1, 3)).toEqual([0, 3, 2, 1, 4])
  })
  it('does not mutate the input tour', () => {
    const t = [0, 1, 2, 3]
    twoOptSwap(t, 0, 2)
    expect(t).toEqual([0, 1, 2, 3])
  })
  it('result is a valid permutation', () => {
    const t = Array.from({ length: N_CITIES }, (_, i) => i)
    const result = twoOptSwap(t, 2, 7)
    expect(result.slice().sort((a, b) => a - b)).toEqual(t)
  })
})

describe('moveKey', () => {
  it('always puts the smaller index first', () => {
    expect(moveKey(5, 2)).toBe('2-5')
    expect(moveKey(2, 5)).toBe('2-5')
  })
  it('handles equal indices', () => {
    expect(moveKey(3, 3)).toBe('3-3')
  })
})

describe('sampleNeighbours', () => {
  it('returns exactly k candidates', () => {
    const tour = Array.from({ length: N_CITIES }, (_, i) => i)
    expect(sampleNeighbours(tour, 8)).toHaveLength(8)
  })
  it('each candidate has i < j', () => {
    const tour = Array.from({ length: N_CITIES }, (_, i) => i)
    for (const c of sampleNeighbours(tour, 10)) {
      expect(c.move[0]).toBeLessThan(c.move[1])
    }
  })
  it('each candidate tour is a valid permutation', () => {
    const tour = Array.from({ length: N_CITIES }, (_, i) => i)
    const expected = tour.slice().sort((a, b) => a - b)
    for (const c of sampleNeighbours(tour, 5)) {
      expect(c.tour.slice().sort((a, b) => a - b)).toEqual(expected)
    }
  })
})

describe('makeInitState', () => {
  it('tour is a valid permutation', () => {
    const s = makeInitState(7, 10)
    const expected = Array.from({ length: N_CITIES }, (_, i) => i).sort((a, b) => a - b)
    expect(s.tour.slice().sort((a, b) => a - b)).toEqual(expected)
  })
  it('initial tabu list is empty', () => {
    expect(makeInitState(7, 10).tabuList).toHaveLength(0)
  })
  it('step starts at 0', () => {
    expect(makeInitState(7, 10).step).toBe(0)
  })
  it('bestCost equals tourLength of initial tour', () => {
    const s = makeInitState(7, 10)
    expect(s.bestCost).toBeCloseTo(tourLength(s.tour), 1)
  })
  it('respects tenure and sampleSize params', () => {
    const s = makeInitState(5, 15)
    expect(s.tenure).toBe(5)
    expect(s.sampleSize).toBe(15)
  })
})

describe('stepOnce', () => {
  it('step increments by 1', () => {
    expect(stepOnce(makeInitState(7, 10)).step).toBe(1)
  })
  it('tabu list grows after each step (up to tenure)', () => {
    let s = makeInitState(7, 10)
    s = stepOnce(s)
    expect(s.tabuList.length).toBeGreaterThanOrEqual(1)
  })
  it('tabu list never exceeds tenure', () => {
    let s = makeInitState(3, 10)
    for (let i = 0; i < 20; i++) s = stepOnce(s)
    expect(s.tabuList.length).toBeLessThanOrEqual(s.tenure)
  })
  it('bestCost never increases', () => {
    let s = makeInitState(7, 10)
    for (let i = 0; i < 30; i++) {
      const prev = s.bestCost
      s = stepOnce(s)
      expect(s.bestCost).toBeLessThanOrEqual(prev + 0.001)
    }
  })
  it('tour remains a valid permutation after step', () => {
    const expected = Array.from({ length: N_CITIES }, (_, i) => i).sort((a, b) => a - b)
    let s = makeInitState(7, 10)
    s = stepOnce(s)
    expect(s.tour.slice().sort((a, b) => a - b)).toEqual(expected)
  })
  it('lastMove is set after first step', () => {
    const s = stepOnce(makeInitState(7, 10))
    expect(s.lastMove).not.toBeNull()
    expect(s.lastMove![0]).toBeLessThan(s.lastMove![1])
  })
  it('lastEventMode is set after first step', () => {
    const s = stepOnce(makeInitState(7, 10))
    expect(['improvement', 'admissible', 'aspiration']).toContain(s.lastEventMode)
  })
  it('improvements counter matches improvement events', () => {
    let s = makeInitState(7, 10)
    let counted = 0
    for (let i = 0; i < 20; i++) {
      s = stepOnce(s)
      if (s.lastEventMode === 'improvement') counted++
    }
    expect(s.improvements).toBe(counted)
  })
  it('costHistory grows up to 60 entries', () => {
    let s = makeInitState(7, 10)
    for (let i = 0; i < 70; i++) s = stepOnce(s)
    expect(s.costHistory.length).toBeLessThanOrEqual(60)
    expect(s.costHistory.length).toBeGreaterThan(0)
  })
})

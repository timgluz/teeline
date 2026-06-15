// teeline-web/src/explainers/lk.test.ts
import { describe, it, expect } from 'vitest'
import {
  CITIES, euclidDist, buildDistMatrix, tourDist, DIST, INIT_TOUR,
  doubleBridge, lcgRand, computeLocalSearchFrames, computeILSFrames,
} from './lk'

describe('euclidDist', () => {
  it('returns 5 for 3-4-5 triangle', () => {
    expect(euclidDist([0, 0], [3, 4])).toBeCloseTo(5)
  })
  it('returns 0 for same point', () => {
    expect(euclidDist([10, 20], [10, 20])).toBe(0)
  })
})

describe('buildDistMatrix', () => {
  const cities: [number, number][] = [[0, 0], [3, 4], [6, 8]]
  const m = buildDistMatrix(cities)
  it('is symmetric', () => {
    expect(m[0][1]).toBeCloseTo(m[1][0])
    expect(m[1][2]).toBeCloseTo(m[2][1])
  })
  it('has zero diagonal', () => {
    expect(m[0][0]).toBe(0)
    expect(m[1][1]).toBe(0)
  })
  it('first edge matches euclidDist', () => {
    expect(m[0][1]).toBeCloseTo(euclidDist(cities[0], cities[1]))
  })
})

describe('tourDist', () => {
  it('sums edges correctly for 3-city tour', () => {
    const cities: [number, number][] = [[0, 0], [3, 0], [3, 4]]
    const dist = buildDistMatrix(cities)
    // tour [0,1,2]: 0→1=3, 1→2=4, 2→0=5 → total 12
    expect(tourDist([0, 1, 2], dist)).toBeCloseTo(12)
  })
})

describe('nnTour', () => {
  it('visits all cities', () => {
    const sorted = [...INIT_TOUR].sort((a, b) => a - b)
    expect(sorted).toEqual([...Array(CITIES.length).keys()])
  })
  it('has no duplicates', () => {
    expect(new Set(INIT_TOUR).size).toBe(CITIES.length)
  })
  it('starts from city 0', () => {
    expect(INIT_TOUR[0]).toBe(0)
  })
  it('has correct length', () => {
    expect(INIT_TOUR.length).toBe(CITIES.length)
  })
})

describe('DIST', () => {
  it('is square matrix of size 15', () => {
    expect(DIST.length).toBe(15)
    expect(DIST[0].length).toBe(15)
  })
})

describe('doubleBridge', () => {
  const tour = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]

  it('output has same length as input', () => {
    const rand = lcgRand(42)
    const { result } = doubleBridge(tour, rand)
    expect(result.length).toBe(tour.length)
  })

  it('output contains same cities (just reordered)', () => {
    const rand2 = lcgRand(99)
    const { result } = doubleBridge(tour, rand2)
    expect([...result].sort((a, b) => a - b)).toEqual(tour)
  })

  it('p1 < p2 < p3 and all within bounds', () => {
    const rand3 = lcgRand(7)
    const n = tour.length
    const { p1, p2, p3 } = doubleBridge(tour, rand3)
    expect(p1).toBeGreaterThanOrEqual(1)
    expect(p2).toBeGreaterThan(p1)
    expect(p3).toBeGreaterThan(p2)
    expect(p3).toBeLessThan(n)
  })
})

describe('lcgRand', () => {
  it('returns values in [0,1)', () => {
    const r = lcgRand(1)
    for (let i = 0; i < 20; i++) {
      const v = r()
      expect(v).toBeGreaterThanOrEqual(0)
      expect(v).toBeLessThan(1)
    }
  })

  it('same seed produces same sequence', () => {
    const r1 = lcgRand(42)
    const r2 = lcgRand(42)
    for (let i = 0; i < 5; i++) {
      expect(r1()).toBe(r2())
    }
  })
})

describe('computeLocalSearchFrames', () => {
  const frames = computeLocalSearchFrames(INIT_TOUR, DIST)

  it('last frame has overlay "Local optimum"', () => {
    expect(frames[frames.length - 1].overlay).toBe('Local optimum')
  })

  it('last frame has no scan or swap edges', () => {
    const last = frames[frames.length - 1]
    expect(last.scanEdges).toBeNull()
    expect(last.swapEdges).toBeNull()
  })

  it('swap frames have isScan = false', () => {
    const swapFrames = frames.filter(f => f.swapEdges !== null)
    expect(swapFrames.every(f => !f.isScan)).toBe(true)
  })

  it('scan frames have isScan = true', () => {
    const scanFrames = frames.filter(f => f.scanEdges !== null)
    expect(scanFrames.every(f => f.isScan)).toBe(true)
  })

  it('final tour distance is no worse than initial', () => {
    const initDist = tourDist(INIT_TOUR, DIST)
    const finalDist = frames[frames.length - 1].dist
    expect(finalDist).toBeLessThanOrEqual(initDist + 1e-9)
  })

  it('all swap frames have improving dist (each swap reduces distance)', () => {
    const swapFrames = frames.filter(f => f.swapEdges !== null)
    expect(swapFrames.length).toBeGreaterThan(0) // NN tour should have crossings
    for (let i = 1; i < swapFrames.length; i++) {
      expect(swapFrames[i].dist).toBeLessThan(swapFrames[i - 1].dist + 1e-9)
    }
  })
})

describe('computeILSFrames', () => {
  const rand = lcgRand(42)
  const frames = computeILSFrames(INIT_TOUR, DIST, 30, 5, rand)

  it('last frame has overlay starting with "Done"', () => {
    expect(frames[frames.length - 1].overlay).toMatch(/^Done/)
  })

  it('has at least one bridge_cut frame (highlight === "bridge")', () => {
    expect(frames.some(f => f.highlight === 'bridge')).toBe(true)
  })

  it('bestDist is non-increasing across frames', () => {
    let prev = frames[0].bestDist
    for (const f of frames) {
      expect(f.bestDist).toBeLessThanOrEqual(prev + 1e-9)
      prev = f.bestDist
    }
  })

  it('final bestDist is no worse than initial tour distance', () => {
    const initD = frames[0].currentDist
    expect(frames[frames.length - 1].bestDist).toBeLessThanOrEqual(initD + 1e-9)
  })

  it('all tour arrays have length 15', () => {
    expect(frames.every(f => f.tour.length === 15)).toBe(true)
  })
})

// teeline-web/src/explainers/lk.test.ts
import { describe, it, expect } from 'vitest'
import {
  CITIES, euclidDist, buildDistMatrix, tourDist, nnTour, DIST, INIT_TOUR,
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

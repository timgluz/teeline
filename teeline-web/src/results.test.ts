import { describe, it, expect } from 'vitest'
import { formatGap, formatRuntime, computeRouteLength } from './results'

describe('formatGap', () => {
  it('returns "—" when optimal is not provided', () => {
    expect(formatGap(8000, undefined)).toBe('—')
  })

  it('returns "0.0%" when tour equals optimal', () => {
    expect(formatGap(7542, 7542)).toBe('0.0%')
  })

  it('returns correct percentage with 1 decimal place', () => {
    expect(formatGap(8296, 7542)).toBe('10.0%')
  })

  it('handles large gaps correctly', () => {
    expect(formatGap(15084, 7542)).toBe('100.0%')
  })
})

describe('formatRuntime', () => {
  it('formats sub-second as "<N>ms"', () => {
    expect(formatRuntime(123)).toBe('123ms')
    expect(formatRuntime(0)).toBe('0ms')
    expect(formatRuntime(999)).toBe('999ms')
  })

  it('formats 1000ms or more as "<N.N>s"', () => {
    expect(formatRuntime(1000)).toBe('1.0s')
    expect(formatRuntime(1500)).toBe('1.5s')
    expect(formatRuntime(12300)).toBe('12.3s')
  })
})

describe('computeRouteLength', () => {
  it('returns 0 for a single-city route', () => {
    const cities = [{ id: 1, x: 0, y: 0 }]
    expect(computeRouteLength([1], cities)).toBe(0)
  })

  it('computes Euclidean distances for a simple triangle', () => {
    const cities = [
      { id: 1, x: 0, y: 0 },
      { id: 2, x: 3, y: 0 },
      { id: 3, x: 0, y: 4 },
    ]
    // 1→2: 3, 2→3: 5, 3→1: 4 → total 12
    expect(computeRouteLength([1, 2, 3], cities)).toBeCloseTo(12)
  })

  it('returns 0 for empty route', () => {
    expect(computeRouteLength([], [])).toBe(0)
  })
})

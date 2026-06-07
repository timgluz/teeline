import { describe, it, expect } from 'vitest'
import { scaleCoords, buildPolylinePoints } from './canvas'

const pt = (id: number, x: number, y: number) => ({ id, x, y })

describe('scaleCoords', () => {
  it('returns same number of items as input', () => {
    const cities = [pt(1, 0, 0), pt(2, 10, 10), pt(3, 5, 5)]
    expect(scaleCoords(cities, 100, 100, 10)).toHaveLength(3)
  })

  it('maps min-coord city to padding offset', () => {
    const cities = [pt(1, 0, 0), pt(2, 100, 100)]
    const scaled = scaleCoords(cities, 200, 200, 20)
    const c1 = scaled.find((c) => c.id === 1)!
    expect(c1.x).toBeCloseTo(20)
  })

  it('maps max-coord city to (size - padding) offset', () => {
    const cities = [pt(1, 0, 0), pt(2, 100, 100)]
    const scaled = scaleCoords(cities, 200, 200, 20)
    const c2 = scaled.find((c) => c.id === 2)!
    expect(c2.x).toBeCloseTo(180)
  })

  it('flips Y axis so higher TSPLIB y maps to lower SVG y', () => {
    const cities = [pt(1, 0, 0), pt(2, 0, 100)]
    const scaled = scaleCoords(cities, 200, 200, 20)
    const low = scaled.find((c) => c.id === 1)! // y=0 in TSPLIB → bottom → high SVG y
    const high = scaled.find((c) => c.id === 2)! // y=100 in TSPLIB → top → low SVG y
    expect(low.y).toBeGreaterThan(high.y)
  })

  it('uses uniform scale to preserve aspect ratio', () => {
    // Wide data (x range 200, y range 100) in a square viewport
    const cities = [pt(1, 0, 0), pt(2, 200, 0), pt(3, 0, 100)]
    const scaled = scaleCoords(cities, 300, 300, 0)
    const c2 = scaled.find((c) => c.id === 2)!
    const c3 = scaled.find((c) => c.id === 3)!
    // Uniform scale means x extent = 2 * y extent
    expect(c2.x).toBeCloseTo(c3.y * 2)
  })

  it('places a single city at padding offset (Y-flipped)', () => {
    const cities = [pt(1, 5, 5)]
    const scaled = scaleCoords(cities, 100, 100, 10)
    // Single city: min=max, x = padding, y = height-padding (Y-flip)
    expect(scaled[0].x).toBe(10)
    expect(scaled[0].y).toBe(90)
  })
})

describe('buildPolylinePoints', () => {
  it('builds "x,y x,y …" string from route + scaled city map', () => {
    const scaled = [pt(1, 10, 20), pt(2, 30, 40)]
    const points = buildPolylinePoints([1, 2], scaled)
    expect(points).toContain('10,20')
    expect(points).toContain('30,40')
  })

  it('closes the tour by appending the first city at the end', () => {
    const scaled = [pt(1, 10, 20), pt(2, 30, 40), pt(3, 50, 60)]
    const points = buildPolylinePoints([1, 2, 3], scaled)
    const parts = points.trim().split(/\s+/)
    expect(parts[0]).toBe(parts[parts.length - 1])
  })

  it('returns empty string for empty route', () => {
    expect(buildPolylinePoints([], [])).toBe('')
  })
})

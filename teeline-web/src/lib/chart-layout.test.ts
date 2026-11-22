import { describe, expect, it } from 'vitest'

import {
  axisTop,
  convexHull,
  hullPath,
  niceTicks,
  placeLabels,
  tailCap,
  type LabelRequest,
} from './chart-layout'

const BOUNDS = { width: 720, height: 340 }

describe('niceTicks', () => {
  it('rounds to steps a reader can place a point against', () => {
    // A linear split of 17.53 would give 0, 4.4, 8.8, 13.2, 17.5.
    expect(niceTicks(17.534)).toEqual([0, 5, 10, 15, 20])
    // And a linear split of a tour length would give 0, 8293, 16586, 24879, 33172.
    expect(niceTicks(28844.9)).toEqual([0, 10000, 20000, 30000])
  })

  it('covers the maximum, so the tallest point never pokes past the top tick', () => {
    for (const max of [0.7, 3, 42, 538, 1999, 12345]) {
      const ticks = niceTicks(max)
      expect(ticks[ticks.length - 1]).toBeGreaterThanOrEqual(max)
      expect(ticks[0]).toBe(0)
    }
  })

  it('falls back to a sane axis for a degenerate range', () => {
    expect(niceTicks(0)).toEqual([0, 1])
    expect(niceTicks(Number.NaN)).toEqual([0, 1])
  })
})

describe('tailCap', () => {
  // The berlin52 gaps from before the swarm fix: fifteen comparable solvers, then four that
  // leave a gap several times larger and would otherwise flatten the chart.
  const BERLIN52_GAPS = [
    0, 2.1612, 2.6386, 7.372, 7.6637, 8.4991, 8.8173, 11.0581, 11.1774, 13.3254,
    14.943, 15.4071, 17.2898, 18.4964, 19.0666, 31.9411, 110.1565, 129.3556,
    198.7271, 301.3126,
  ]

  it('cuts above the comparable solvers, not at the largest value', () => {
    expect(tailCap(BERLIN52_GAPS)).toBe(31.9411)
  })

  it('cuts when the tail is a single point', () => {
    expect(tailCap([0, 0.52, 2, 3.2, 4.89, 11.16, 282.5])).toBe(11.16)
  })

  it('leaves a smooth series uncapped', () => {
    expect(tailCap([1, 2, 3, 4, 5])).toBeNull()
    expect(tailCap([7544, 7682, 7743, 8097, 8107, 8318, 8379, 8384])).toBeNull()
  })

  it('ignores a large jump when most of the series sits above it', () => {
    // 1 -> 10 is a 10x jump, but it is the series, not a tail.
    expect(tailCap([1, 10, 11, 12, 13, 14])).toBeNull()
  })

  it('has no opinion on a series too short to have a tail', () => {
    expect(tailCap([1, 2, 100])).toBeNull()
    expect(tailCap([])).toBeNull()
  })
})

describe('axisTop', () => {
  it('rounds the cap up so a point just past it still fits', () => {
    expect(
      axisTop([
        0, 2.1612, 2.6386, 7.372, 7.6637, 8.4991, 8.8173, 11.0581, 11.1774,
        13.3254, 14.943, 15.4071, 17.2898, 18.4964, 19.0666, 31.9411, 110.1565,
        129.3556, 198.7271, 301.3126,
      ]),
    ).toBe(40)
  })

  it('uses the data maximum when no tail needs cutting', () => {
    expect(axisTop([7544, 7682, 7743, 8097, 8107, 8318, 8379, 8384])).toBe(
      10000,
    )
  })

  it('falls back to a sane axis for no data', () => {
    expect(axisTop([])).toBe(1)
  })
})

describe('convexHull', () => {
  it('keeps the outer ring and drops a point inside it', () => {
    const hull = convexHull([
      { x: 0, y: 0 },
      { x: 10, y: 0 },
      { x: 10, y: 10 },
      { x: 0, y: 10 },
      { x: 5, y: 5 },
    ])
    expect(hull).toHaveLength(4)
    expect(hull).not.toContainEqual({ x: 5, y: 5 })
  })

  it('does not invent an area for a line or a pair', () => {
    expect(
      convexHull([
        { x: 0, y: 0 },
        { x: 5, y: 5 },
        { x: 10, y: 10 },
      ]),
    ).toEqual([])
    expect(
      convexHull([
        { x: 0, y: 0 },
        { x: 10, y: 10 },
      ]),
    ).toEqual([])
  })

  it('writes a closed path for a triangle', () => {
    expect(
      hullPath([
        { x: 0, y: 0 },
        { x: 10, y: 0 },
        { x: 0, y: 10 },
      ]),
    ).toBe('M0 0 L10 0 L0 10 Z')
  })
})

describe('placeLabels', () => {
  it('puts a lone label beside its point', () => {
    const [placement] = placeLabels([{ text: 'gr17', x: 100, y: 100 }], BOUNDS)
    expect(placement.anchor).toBe('start')
    expect(placement.x).toBe(108)
  })

  it('moves a label that would overlap one already placed', () => {
    // Two points close together, at almost the same height: the second label cannot sit where
    // the first one is.
    const requests: LabelRequest[] = [
      { text: 'ulysses16', x: 64, y: 112 },
      { text: 'ulysses22', x: 127, y: 115 },
    ]
    const [first, second] = placeLabels(requests, BOUNDS)
    expect(first.x).toBe(72)
    // Dropped a line below its point, which is where it clears the first label's box.
    expect(second.y).toBeGreaterThan(first.y + 13)
    expect(second.anchor).toBe('start')
  })

  it('flips a label that would run past the right edge', () => {
    const [placement] = placeLabels([{ text: 'eil76', x: 700, y: 60 }], BOUNDS)
    expect(placement.anchor).toBe('end')
    expect(placement.x).toBe(692)
  })

  it('keeps a label clear of the top edge', () => {
    const [placement] = placeLabels([{ text: 'a280', x: 300, y: 4 }], BOUNDS)
    expect(placement.y).toBeGreaterThanOrEqual(0)
  })

  it('never drops a label, even when everything collides', () => {
    const requests: LabelRequest[] = Array.from({ length: 8 }, (_, i) => ({
      text: `solver${i}`,
      x: 300,
      y: 150,
    }))
    const placements = placeLabels(requests, BOUNDS)
    expect(placements).toHaveLength(8)
    expect(
      placements.every((placement) => placement.text.startsWith('solver')),
    ).toBe(true)
  })

  it('leaves room between two labels that nearly touch', () => {
    // The width estimate is what stops "ulysses16ulysses22" from rendering as one word.
    const placements = placeLabels(
      [
        { text: 'ulysses16', x: 64, y: 112 },
        { text: 'ulysses22', x: 127, y: 115 },
      ],
      BOUNDS,
    )
    expect(placements[0].y).not.toBe(placements[1].y)
  })
})

import { describe, expect, it } from 'vitest'

import { niceTicks, placeLabels, type LabelRequest } from './chart-layout'

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

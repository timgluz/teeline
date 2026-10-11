import { describe, expect, it } from 'vitest'

import {
  attDistance,
  ceilDistance,
  eucDistance,
  geoDistance,
  nint,
  tourLength,
} from '../../scripts/lib/problem-tour.mjs'

/** A TSPLIB file with `coords` rows and nothing else that matters. */
const tsp = (type, rows, dimension = rows.length) =>
  `NAME: t\nTYPE: TSP\nDIMENSION: ${dimension}\nEDGE_WEIGHT_TYPE: ${type}\nNODE_COORD_SECTION\n${rows
    .map(([id, x, y]) => `${id} ${x} ${y}`)
    .join('\n')}\nEOF\n`

/** A tour file. `perLine` controls how many ids share a line. */
const tour = (ids, perLine = 1) => {
  const lines = []
  for (let i = 0; i < ids.length; i += perLine)
    lines.push(ids.slice(i, i + perLine).join(' '))
  return `NAME: t.opt.tour\nTYPE: TOUR\nTOUR_SECTION\n${lines.join('\n')}\n-1\nEOF\n`
}

describe('eucDistance', () => {
  it('rounds to the nearest integer, halves up — TSPLIB EUC_2D', () => {
    expect(eucDistance({ x: 0, y: 0 }, { x: 3, y: 4 })).toBe(5)
    expect(eucDistance({ x: 0, y: 0 }, { x: 1.5, y: 0 })).toBe(2) // exactly .5 -> up
    expect(eucDistance({ x: 0, y: 0 }, { x: 0.6, y: 0 })).toBe(1)
    expect(eucDistance({ x: 0, y: 0 }, { x: 0.4, y: 0 })).toBe(0)
    expect(nint(0.5)).toBe(1)
  })
})

describe('the other TSPLIB metrics', () => {
  it('ATT never rounds below the pseudo-Euclidean value', () => {
    // r = sqrt(100/10) = 3.162 -> nint 3, which went *down*, so the result is bumped to 4.
    expect(attDistance({ x: 0, y: 0 }, { x: 10, y: 0 })).toBe(4)
    // r = sqrt(25/10) = 1.581 -> nint 2, which went up, so nothing is added.
    expect(attDistance({ x: 0, y: 0 }, { x: 3, y: 4 })).toBe(2)
    expect(attDistance({ x: 0, y: 0 }, { x: 0, y: 0 })).toBe(0)
  })

  it('CEIL_2D rounds up, GEO truncates', () => {
    expect(ceilDistance({ x: 0, y: 0 }, { x: 0.1, y: 0 })).toBe(1)
    expect(ceilDistance({ x: 0, y: 0 }, { x: 2, y: 0 })).toBe(2)
    // TSPLIB's GEO formula adds 1 inside the floor, so even identical points come out as 1 rather
    // than 0. That is the definition, not an off-by-one — and it is why the corpus values match.
    expect(geoDistance({ x: 10, y: 20 }, { x: 10, y: 20 })).toBe(1)
    expect(geoDistance({ x: 16.47, y: 96.1 }, { x: 23.7, y: 96.99 })).toBe(
      geoDistance({ x: 23.7, y: 96.99 }, { x: 16.47, y: 96.1 }),
    )
  })
})

describe('tourLength', () => {
  const square = [
    [1, 0, 0],
    [2, 6, 0],
    [3, 6, 6],
    [4, 0, 6],
  ]

  it('scores an EUC_2D route with the rounded metric, not plain Euclidean', () => {
    // Sides of 6 -> rounded edges are 6 each, total 24. Plain Euclidean would also be 24 here, so
    // the next case is the one that proves rounding: sides of 0.6 round to 1 each.
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 4]))).toBe(24)

    const small = [
      [1, 0, 0],
      [2, 0.6, 0],
      [3, 0.6, 0.6],
      [4, 0, 0.6],
    ]
    // Unrounded: 2.4. Rounded per edge: 4. The published optimum of an EUC_2D instance is the
    // rounded one, which is what made excluding this type wrong after #575.
    expect(tourLength(tsp('EUC_2D', small), tour([1, 2, 3, 4]))).toBe(4)
  })

  it('pays the return edge, which the file does not spell out', () => {
    // Four ids, four edges: the tour is closed implicitly, so 6 + 6 + 6 + 6.
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 4]))).toBe(24)
    // Spelling the return out makes it five ids for a four-city instance, which the completeness
    // check refuses rather than scoring a route that visits city 1 twice.
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 4, 1]))).toBeNull()
  })

  it('reads node ids split across lines, not one per line', () => {
    // pr1002 writes 1002 ids across 64 lines; reading a line at a time scored 11 of rd100's 100
    // cities and published 4344 for a 7910 optimum.
    const onePerLine = tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 4], 1))
    const packed = tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 4], 4))
    expect(packed).toBe(onePerLine)
    expect(packed).toBe(24)
  })

  it('refuses a route that does not name every city', () => {
    // Without the completeness check a subset still produces a total — a *smaller* one, which is
    // the worst kind of wrong number to publish as a proven optimum.
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2]))).toBeNull()
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 4, 4]))).toBeNull()
  })

  it('refuses what it cannot score rather than guessing', () => {
    // A matrix instance has no coordinates to score against.
    expect(tourLength(tsp('EXPLICIT', square), tour([1, 2, 3, 4]))).toBeNull()
    // No DIMENSION to check the route against.
    const noDimension = `NAME: t\nEDGE_WEIGHT_TYPE: EUC_2D\nNODE_COORD_SECTION\n1 0 0\n2 1 0\nEOF\n`
    expect(tourLength(noDimension, tour([1, 2]))).toBeNull()
    // A route naming a city the coordinate section does not have.
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 9]))).toBeNull()
  })
})

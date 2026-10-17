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
    // than 0. That is the definition, not an off-by-one.
    expect(geoDistance({ x: 10, y: 20 }, { x: 10, y: 20 })).toBe(1)
    // A reference value, so a wrong radius, degree/minute conversion or PI constant cannot pass on
    // symmetry alone: ulysses16 cities 1 (38.24, 20.42) and 2 (39.57, 26.15), computed from the
    // TSPLIB definition with full-precision PI — the constant the solver uses.
    expect(geoDistance({ x: 38.24, y: 20.42 }, { x: 39.57, y: 26.15 })).toBe(
      509,
    )
    expect(geoDistance({ x: 16.47, y: 96.1 }, { x: 16.47, y: 94.44 })).toBe(153) // burma14 1-2
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

  it('keeps a GEO tour finite where repeated coordinates are involved', () => {
    // `geoDistance` clamps acos's argument, mirroring `src/tsp/distance_matrix.rs` where f32
    // rounding can push it outside [-1, 1] and NaN poisons the total. f64 keeps this input in range,
    // so the test pins the observable contract — a finite total — rather than the clamp's branch.
    const samePlace = [
      [1, 10, 20],
      [2, 10, 20],
    ]
    // Two edges of 1 each: GEO's formula adds 1 inside the floor, even for identical points.
    expect(tourLength(tsp('GEO', samePlace, 2), tour([1, 2]))).toBe(2)
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
    const packed = tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 4], 3)) // two lines: "1 2 3" / "4"
    expect(packed).toBe(onePerLine)
    expect(packed).toBe(24)
  })

  it('refuses a route that does not name every city', () => {
    // Without the completeness check a subset still produces a total — a *smaller* one, which is
    // the worst kind of wrong number to publish as a proven optimum.
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2]))).toBeNull()
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 4, 4]))).toBeNull()
    // The same *length* as the instance, but city 3 twice and city 4 never: without the permutation
    // check this scored 18 for a square whose every real tour is 24.
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 3]))).toBeNull()
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 2, 3]))).toBeNull()
  })

  it('scores the non-EUC types through the same dispatcher', () => {
    // ATT triangle (0,0) (10,0) (10,10): each edge rounds up to 4, 4 and 5 -> 13. The solver's own
    // --optimal-tour reports 13 for this fixture.
    const att = [
      [1, 0, 0],
      [2, 10, 0],
      [3, 10, 10],
    ]
    expect(tourLength(tsp('ATT', att), tour([1, 2, 3]))).toBe(13)
    expect(tourLength(tsp('CEIL_2D', att), tour([1, 2, 3]))).toBe(35) // 10 + 10 + ceil(14.14)
  })

  it('refuses what it cannot score rather than guessing', () => {
    // A matrix instance has no coordinates to score against.
    expect(tourLength(tsp('EXPLICIT', square), tour([1, 2, 3, 4]))).toBeNull()
    // No DIMENSION to check the route against.
    const noDimension = `NAME: t\nEDGE_WEIGHT_TYPE: EUC_2D\nNODE_COORD_SECTION\n1 0 0\n2 1 0\nEOF\n`
    expect(tourLength(noDimension, tour([1, 2]))).toBeNull()
    // A DIMENSION that disagrees with the coordinate rows, both ways round: the route has to be a
    // tour of the instance the file describes, and the coordinate section is that instance.
    expect(tourLength(tsp('EUC_2D', square, 9), tour([1, 2, 3, 4]))).toBeNull()
    expect(tourLength(tsp('EUC_2D', square, 3), tour([1, 2, 3]))).toBeNull()
    // Coordinates that overflow the metric entirely: a refusal, not `Infinity` on a page.
    const huge = [
      [1, 1e200, 1e200],
      [2, -1e200, -1e200],
    ]
    expect(tourLength(tsp('EUC_2D', huge), tour([1, 2]))).toBeNull()
    // A route naming a city the coordinate section does not have.
    expect(tourLength(tsp('EUC_2D', square), tour([1, 2, 3, 9]))).toBeNull()
  })
})

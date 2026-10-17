import { describe, expect, it } from 'vitest'

import {
  minimapSource,
  minimapPoints,
  parseDistanceMatrix,
  readSection,
  renderMinimapSvg,
} from '../../scripts/lib/problem-minimap.mjs'

/** A 5-city symmetric matrix as d(1,2)=1 … d(4,5)=10, written out in each layout by hand. */
const FOUR_DISTINCT_LAYOUTS = [
  ['FULL_MATRIX', '0 1 2 3 4 1 0 5 6 7 2 5 0 8 9 3 6 8 0 10 4 7 9 10 0'],
  // UPPER_ROW and LOWER_COL are transposes of each other, so a symmetric matrix writes them
  // identically — likewise UPPER_COL/LOWER_ROW and the two pairs of diagonal forms.
  ['UPPER_ROW', '1 2 3 4 5 6 7 8 9 10'],
  ['LOWER_COL', '1 2 3 4 5 6 7 8 9 10'],
  ['UPPER_COL', '1 2 5 3 6 8 4 7 9 10'],
  ['LOWER_ROW', '1 2 5 3 6 8 4 7 9 10'],
  ['UPPER_DIAG_ROW', '0 1 2 3 4 0 5 6 7 0 8 9 0 10 0'],
  ['LOWER_DIAG_COL', '0 1 2 3 4 0 5 6 7 0 8 9 0 10 0'],
  ['LOWER_DIAG_ROW', '0 1 0 2 5 0 3 6 8 0 4 7 9 10 0'],
  ['UPPER_DIAG_COL', '0 1 0 2 5 0 3 6 8 0 4 7 9 10 0'],
]

const matrixTsp = (format, weights) =>
  `NAME: t\nDIMENSION: 5\nEDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: ${format}\nEDGE_WEIGHT_SECTION\n${weights}\nEOF\n`

const PAIRS = [
  [0, 1, 1],
  [0, 2, 2],
  [0, 3, 3],
  [0, 4, 4],
  [1, 2, 5],
  [1, 3, 6],
  [1, 4, 7],
  [2, 3, 8],
  [2, 4, 9],
  [3, 4, 10],
]

describe('parseDistanceMatrix', () => {
  it('reads every EDGE_WEIGHT_FORMAT layout into the same matrix', () => {
    for (const [format, weights] of FOUR_DISTINCT_LAYOUTS) {
      const D = parseDistanceMatrix(matrixTsp(format, weights), 5, format)
      expect(D, `${format} should parse`).not.toBeNull()
      for (const [i, j, want] of PAIRS) {
        expect(D[i][j], `${format}: d(${i},${j})`).toBe(want)
        expect(D[j][i], `${format}: d(${j},${i}) mirrors`).toBe(want)
      }
    }
  })

  it('tells the row and column orders apart at n=5', () => {
    // At n=3 every layout takes the same token sequence, so a transposed mapping would pass. Here
    // UPPER_COL's tokens are also a valid UPPER_ROW section — of a different instance.
    const tokens = '1 2 5 3 6 8 4 7 9 10'
    const asCol = parseDistanceMatrix(
      matrixTsp('UPPER_COL', tokens),
      5,
      'UPPER_COL',
    )
    const asRow = parseDistanceMatrix(
      matrixTsp('UPPER_ROW', tokens),
      5,
      'UPPER_ROW',
    )
    expect(asCol[0][1]).toBe(1)
    expect(asCol[1][2]).toBe(5)
    expect(asRow[1][2]).toBe(6)
    expect(asCol).not.toEqual(asRow)
  })

  it('rejects a token count that does not match the format', () => {
    // 5 tokens for 5 cities is not a valid length in any layout; reading it anyway would shift
    // every distance by one from the first diagonal it assumed.
    expect(
      parseDistanceMatrix(matrixTsp('UPPER_ROW', '1 2 3 4 5'), 5, 'UPPER_ROW'),
    ).toBeNull()
    expect(
      parseDistanceMatrix(
        matrixTsp('UPPER_DIAG_ROW', '0 1 2 3 4'),
        5,
        'UPPER_DIAG_ROW',
      ),
    ).toBeNull()
  })

  it('rejects an unknown format rather than guessing a layout', () => {
    expect(
      parseDistanceMatrix(
        matrixTsp('UPPER_TRIANGLE', '1 2 3 4 5 6 7 8 9 10'),
        5,
        'UPPER_TRIANGLE',
      ),
    ).toBeNull()
  })

  it('rejects a non-numeric section', () => {
    expect(
      parseDistanceMatrix(
        matrixTsp('UPPER_ROW', '1 2 three 4 5 6 7 8 9 10'),
        5,
        'UPPER_ROW',
      ),
    ).toBeNull()
  })
})

describe('readSection', () => {
  it('ends at the next keyword, not at a blank line', () => {
    const txt =
      'NAME: t\nNODE_COORD_SECTION\n1 1.0 2.0\n\n2 3.0 4.0\nDISPLAY_DATA_SECTION\n1 9 9\nEOF\n'
    expect(readSection(txt, 'NODE_COORD_SECTION')).toEqual([
      ['1', '1.0', '2.0'],
      ['2', '3.0', '4.0'],
    ])
  })

  it('matches a header with trailing whitespace', () => {
    const txt = 'NAME: t\nEDGE_WEIGHT_SECTION   \n1 2 3\nEOF\n'
    expect(readSection(txt, 'EDGE_WEIGHT_SECTION')).toEqual([['1', '2', '3']])
  })
})

describe('minimapSource and parsePoints', () => {
  it('reports where the points came from', () => {
    expect(minimapSource('NODE_COORD_SECTION\n1 1 2\nEOF\n')).toBe('coords')
    expect(minimapSource('DISPLAY_DATA_SECTION\n1 1 2\nEOF\n')).toBe('display')
    expect(minimapSource('EDGE_WEIGHT_SECTION\n1 2 3\nEOF\n')).toBe('matrix')
  })

  it('prefers real coordinates when a file has both sections', () => {
    const txt =
      'EDGE_WEIGHT_TYPE: EXPLICIT\nEDGE_WEIGHT_FORMAT: UPPER_ROW\nNODE_COORD_SECTION\n1 10.0 20.0\n2 30.0 40.0\nDISPLAY_DATA_SECTION\n1 9.0 9.0\n2 8.0 8.0\nEOF\n'
    expect(minimapPoints(txt, 2, 'UPPER_ROW')).toEqual({
      points: [
        { x: 10, y: 20 },
        { x: 30, y: 40 },
      ],
      derived: false,
    })
  })

  it('derives a layout when only a matrix is published', () => {
    const { points, derived } = minimapPoints(
      matrixTsp('UPPER_ROW', '1 2 3 4 5 6 7 8 9 10'),
      5,
      'UPPER_ROW',
    )
    expect(derived).toBe(true)
    expect(points).toHaveLength(5)
    for (const p of points) {
      expect(Number.isFinite(p.x)).toBe(true)
      expect(Number.isFinite(p.y)).toBe(true)
    }
  })

  it('is deterministic, so an unchanged instance republishes identical bytes', () => {
    const txt = matrixTsp('UPPER_ROW', '1 2 3 4 5 6 7 8 9 10')
    expect(minimapPoints(txt, 5, 'UPPER_ROW').points).toEqual(
      minimapPoints(txt, 5, 'UPPER_ROW').points,
    )
  })
})

describe('renderMinimapSvg', () => {
  const points = Array.from({ length: 40 }, (_, i) => ({
    x: i % 8,
    y: Math.floor(i / 8),
  }))

  it('is a standalone document that carries its own styling', () => {
    const svg = renderMinimapSvg(points)
    // Fetched through <img>, it gets no page context: no external CSS, no inherited alt.
    expect(svg).toContain('xmlns="http://www.w3.org/2000/svg"')
    expect(svg).toContain('viewBox="0 0 340 200"')
    expect(svg).toContain('width="340"')
    expect(svg).toContain('background:#f6f8fa')
    expect(svg).toContain('aria-label="City map for this problem"')
    expect(svg).not.toContain('width="100%"')
    expect(svg.trimEnd().endsWith('</svg>')).toBe(true)
  })

  it('labels a derived layout differently', () => {
    expect(renderMinimapSvg(points, true)).toContain(
      'derived from the distance matrix',
    )
  })

  it('draws points as dots, and switches to a density grid when they crowd', () => {
    const dots = renderMinimapSvg(points)
    expect(dots).toContain('<circle')
    expect(dots).not.toContain('<rect')

    // 4,000 points in a 300x100 lattice: far past the threshold, and far too many circles for a
    // 340x200 viewBox to keep.
    const crowded = Array.from({ length: 4000 }, (_, i) => ({
      x: i % 300,
      y: Math.floor(i / 300),
    }))
    const grid = renderMinimapSvg(crowded)
    expect(grid).toContain('<rect')
    expect(grid).not.toContain('<circle')
    expect(grid.length).toBeLessThan(dots.length * 60)
  })
})

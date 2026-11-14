import { describe, it, expect } from 'vitest'
import {
  CITIES_SFC,
  N_CITIES_SFC,
  bounds,
  buildLayout,
  countLongJumps,
  curveCells,
  distance,
  hilbertIndex,
  longestEdge,
  mortonIndex,
  normalizeToGrid,
  orderCities,
  seamEdge,
  tourEdges,
} from './sfc-algo'
import type { CurveType, Point } from './sfc-algo'

describe('fixture', () => {
  it('has 24 unique cities', () => {
    expect(N_CITIES_SFC).toBe(24)
    const keys = new Set(CITIES_SFC.map(([x, y]) => `${x},${y}`))
    expect(keys.size).toBe(N_CITIES_SFC)
  })

  it('has a 256 x 254 bounding box', () => {
    const b = bounds(CITIES_SFC)
    expect(b.spanX).toBe(256)
    expect(b.spanY).toBe(254)
    expect(b.span).toBe(256)
  })
})

describe('normalizeToGrid', () => {
  it('fills the larger span and leaves the shorter span under 1% margin', () => {
    const grid = normalizeToGrid(CITIES_SFC, 8)
    const max = (1 << 8) - 1
    const maxX = Math.max(...grid.map(([x]) => x))
    const maxY = Math.max(...grid.map(([, y]) => y))
    expect(maxX).toBe(max)
    expect(maxY).toBeLessThanOrEqual(max)
    expect((max - maxY) / (max + 1)).toBeLessThan(0.01)
  })

  it('clamps the maximum edge', () => {
    expect(
      normalizeToGrid(
        [
          [0, 0],
          [10, 10],
        ],
        4,
      ),
    ).toEqual([
      [0, 0],
      [15, 15],
    ])
  })

  it('handles identical points', () => {
    expect(
      normalizeToGrid(
        [
          [5, 5],
          [5, 5],
        ],
        8,
      ),
    ).toEqual([
      [0, 0],
      [0, 0],
    ])
  })

  it('handles collinear points', () => {
    const grid = normalizeToGrid(
      [
        [0, 0],
        [1, 0],
        [2, 0],
      ],
      4,
    )
    expect(grid[0]).toEqual([0, 0])
    expect(grid[1]).toEqual([8, 0])
    expect(grid[2]).toEqual([15, 0])
  })
})

describe('hilbertIndex', () => {
  it('matches the p=1 reference traversal', () => {
    expect(hilbertIndex(1, 0, 0)).toBe(0)
    expect(hilbertIndex(1, 0, 1)).toBe(1)
    expect(hilbertIndex(1, 1, 1)).toBe(2)
    expect(hilbertIndex(1, 1, 0)).toBe(3)
  })

  it('matches the p=2 reference grid', () => {
    const expected = [
      [0, 1, 14, 15],
      [3, 2, 13, 12],
      [4, 7, 8, 11],
      [5, 6, 9, 10],
    ]
    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 4; x++) {
        expect(hilbertIndex(2, x, y)).toBe(expected[y][x])
      }
    }
  })

  it('is a bijection and keeps consecutive indices adjacent for p <= 6', () => {
    for (let p = 1; p <= 6; p++) {
      const cells = curveCells(p, 'hilbert')
      const n = 1 << p
      expect(new Set(cells.map(([x, y]) => y * n + x)).size).toBe(n * n)
      for (let i = 1; i < cells.length; i++) {
        const [x0, y0] = cells[i - 1]
        const [x1, y1] = cells[i]
        expect(Math.abs(x1 - x0) + Math.abs(y1 - y0)).toBe(1)
      }
    }
  })

  it('handles the p=8 boundary without overflow', () => {
    expect(hilbertIndex(8, 255, 255)).toBeLessThan(65536)
    const cells = curveCells(8, 'hilbert')
    expect(cells).toHaveLength(65536)
    expect(cells[65535]).toEqual([255, 0])
  })
})

describe('mortonIndex', () => {
  it('matches the p=1 reference traversal', () => {
    expect(mortonIndex(0, 0)).toBe(0)
    expect(mortonIndex(1, 0)).toBe(1)
    expect(mortonIndex(0, 1)).toBe(2)
    expect(mortonIndex(1, 1)).toBe(3)
  })

  it('matches the p=2 reference grid', () => {
    const expected = [
      [0, 1, 4, 5],
      [2, 3, 6, 7],
      [8, 9, 12, 13],
      [10, 11, 14, 15],
    ]
    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 4; x++) {
        expect(mortonIndex(x, y)).toBe(expected[y][x])
      }
    }
  })

  it('does not collapse cells when the 64-bit Rust masks are used in 32-bit JS', () => {
    const keys = new Set<number>()
    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 4; x++) {
        keys.add(mortonIndex(x, y))
      }
    }
    expect(keys.size).toBe(16)
  })

  it('is a bijection for p <= 6', () => {
    for (let p = 1; p <= 6; p++) {
      const n = 1 << p
      const keys = new Set<number>()
      for (let y = 0; y < n; y++) {
        for (let x = 0; x < n; x++) {
          keys.add(mortonIndex(x, y))
        }
      }
      expect(keys.size).toBe(n * n)
    }
  })

  it('handles the p=8 boundary', () => {
    expect(mortonIndex(255, 0)).toBe(21845)
    expect(mortonIndex(0, 255)).toBe(43690)
    expect(mortonIndex(255, 255)).toBe(65535)
    const cells = curveCells(8, 'morton')
    expect(cells).toHaveLength(65536)
    expect(cells[65535]).toEqual([255, 255])
    expect(countLongJumps(cells)).toBe(32767)
  })
})

describe('curveCells', () => {
  it('returns 4^p cells for p = 1..6', () => {
    for (let p = 1; p <= 6; p++) {
      expect(curveCells(p, 'hilbert')).toHaveLength(4 ** p)
      expect(curveCells(p, 'morton')).toHaveLength(4 ** p)
    }
  })

  it('has no long jumps for Hilbert', () => {
    for (let p = 1; p <= 6; p++) {
      expect(countLongJumps(curveCells(p, 'hilbert'))).toBe(0)
    }
  })

  it('counts Morton quadrant jumps', () => {
    expect(countLongJumps(curveCells(1, 'morton'))).toBe(1)
    expect(countLongJumps(curveCells(2, 'morton'))).toBe(7)
    expect(countLongJumps(curveCells(3, 'morton'))).toBe(31)
  })
})

describe('buildLayout', () => {
  it('changes order at p = 2..6 for both curve types', () => {
    for (const curveType of ['hilbert', 'morton'] as CurveType[]) {
      const orders = [1, 2, 3, 4, 5, 6].map((p) =>
        buildLayout(CITIES_SFC, p, curveType).order.join(','),
      )
      for (let i = 1; i < orders.length; i++) {
        expect(orders[i], `${curveType} p=${i + 1}`).not.toBe(orders[i - 1])
      }
    }
  })

  it('keeps same-cell cities in input order', () => {
    const cities: Point[] = [
      [0, 0],
      [1, 0],
      [0, 10],
      [10, 10],
    ]
    expect(orderCities(cities, 1, 'hilbert').slice(0, 2)).toEqual([0, 1])

    const reversed: Point[] = [
      [1, 0],
      [0, 0],
      [0, 10],
      [10, 10],
    ]
    expect(orderCities(reversed, 1, 'hilbert').slice(0, 2)).toEqual([0, 1])
  })

  it('selects the closing edge as the seam', () => {
    const layout = buildLayout(CITIES_SFC, 4, 'hilbert')
    expect(layout.seam.closing).toBe(true)
    expect(layout.seam.from).toBe(layout.order[layout.order.length - 1])
    expect(layout.seam.to).toBe(layout.order[0])
  })

  it('breaks longest-edge ties in tour order', () => {
    const cities: Point[] = [
      [0, 0],
      [1, 0],
      [1, 1],
      [0, 1],
    ]
    const edges = tourEdges(cities, [0, 1, 2, 3])
    expect(longestEdge(edges)).toBe(edges[0])
    expect(seamEdge(edges)).toBe(edges[3])
  })

  it('finds a longest edge that is not the seam', () => {
    const cities: Point[] = [
      [0, 0],
      [10, 0],
      [10, 1],
      [0, 1],
    ]
    const edges = tourEdges(cities, [0, 1, 2, 3])
    expect(longestEdge(edges)).toBe(edges[0])
    expect(seamEdge(edges)).toBe(edges[3])
    expect(longestEdge(edges)).not.toBe(seamEdge(edges))
  })

  it('reports the p=1 bucket and jump statistics', () => {
    const hilbert = buildLayout(CITIES_SFC, 1, 'hilbert')
    expect(hilbert.stats.cellsUsed).toBe(4)
    expect(hilbert.stats.largestBucket).toBe(6)
    expect(hilbert.stats.longJumps).toBe(0)

    const morton = buildLayout(CITIES_SFC, 1, 'morton')
    expect(morton.stats.longJumps).toBe(1)
  })

  it('computes tour, seam, and longest lengths in layout units', () => {
    const layout = buildLayout(CITIES_SFC, 4, 'hilbert')
    const seamLength = distance(
      CITIES_SFC[layout.seam.from],
      CITIES_SFC[layout.seam.to],
    )
    expect(layout.stats.seamLength).toBeCloseTo(seamLength, 6)
    expect(layout.stats.longestEdgeLength).toBeGreaterThanOrEqual(
      layout.stats.seamLength,
    )
    expect(layout.stats.tourLength).toBeGreaterThan(layout.stats.seamLength)
  })

  it('is deterministic', () => {
    expect(buildLayout(CITIES_SFC, 4, 'hilbert')).toEqual(
      buildLayout(CITIES_SFC, 4, 'hilbert'),
    )
  })
})

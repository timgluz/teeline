// Pure simulation logic for the Space-Filling Curve explainer.
// Mirrors only the parts of src/tsp/space_filling_curve.rs the demo needs:
// uniform normalization, Hilbert/Morton indexing, and input-order sorting.
// The solver also supports rotation, mirroring, aspect scaling, a
// multi-orientation ensemble, and alternate tie-breakers; those are
// intentionally omitted here so the construction stays legible while p is
// scrubbed.

export type Point = [number, number]
export type Cell = [number, number]
export type CurveType = 'hilbert' | 'morton'

export type Bounds = {
  minX: number
  minY: number
  spanX: number
  spanY: number
  span: number
}

export type TourEdge = {
  from: number
  to: number
  length: number
  closing: boolean
}

export type TwoOptEdge = {
  from: number
  to: number
  length: number
}

export type TwoOptPreview = {
  removed: TourEdge[]
  added: TwoOptEdge[]
  oldLength: number
  newLength: number
  delta: number
}

export type SfcStats = {
  cities: number
  cellsUsed: number
  sharedCells: number
  largestBucket: number
  tourLength: number
  seamLength: number
  longestEdgeLength: number
  longJumps: number
}

export type SfcRow = {
  city: number
  cell: Cell
  index: number
  visit: number
}

export type SfcBucket = {
  cell: Cell
  index: number
  cities: number[] // input order
}

export type SfcLayout = {
  p: number
  curveType: CurveType
  cities: Point[]
  bounds: Bounds
  grid: Cell[] // projected cell per city
  indices: number[] // curve index per city
  cells: Cell[] // curve cells in index order
  order: number[] // city indices in visit order
  rows: SfcRow[] // visit order, with cell and index
  buckets: SfcBucket[] // cells in curve-index order, cities in input order
  edges: TourEdge[]
  seam: TourEdge
  longest: TourEdge
  stats: SfcStats
}

// Below this span the grid collapses; identical or collinear cities must not divide by zero.
const EPSILON = 1e-12

// Fixed teaching layout. Four corner clusters make low-p buckets obvious; the
// 256 x 254 bounding box keeps the order changing through p = 6.
export const CITIES_SFC: Point[] = [
  [22, 208],
  [46, 232],
  [70, 256],
  [94, 280],
  [30, 270],
  [88, 214],
  [206, 208],
  [230, 232],
  [254, 256],
  [278, 280],
  [214, 270],
  [272, 214],
  [22, 26],
  [46, 50],
  [70, 74],
  [94, 98],
  [30, 88],
  [88, 32],
  [206, 26],
  [230, 50],
  [254, 74],
  [278, 98],
  [214, 88],
  [272, 32],
]

export const N_CITIES_SFC = CITIES_SFC.length

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

export function bounds(cities: Point[]): Bounds {
  let minX = Infinity
  let maxX = -Infinity
  let minY = Infinity
  let maxY = -Infinity
  for (const [x, y] of cities) {
    minX = Math.min(minX, x)
    maxX = Math.max(maxX, x)
    minY = Math.min(minY, y)
    maxY = Math.max(maxY, y)
  }
  const spanX = Math.max(maxX - minX, EPSILON)
  const spanY = Math.max(maxY - minY, EPSILON)
  return { minX, minY, spanX, spanY, span: Math.max(spanX, spanY) }
}

function projectToGrid(cities: Point[], p: number, b: Bounds): Cell[] {
  const max = (1 << p) - 1
  return cities.map(([x, y]) => [
    clamp(Math.round(((x - b.minX) / b.span) * max), 0, max),
    clamp(Math.round(((y - b.minY) / b.span) * max), 0, max),
  ])
}

export function normalizeToGrid(cities: Point[], p: number): Cell[] {
  return projectToGrid(cities, p, bounds(cities))
}

// Standard Lam-Shapiro xy2d. The reflection uses the full grid size n - 1 - x,
// not the current quadrant size; consecutive indices are grid neighbours.
export function hilbertIndex(p: number, x: number, y: number): number {
  const n = 1 << p
  let d = 0
  let s = n >> 1
  while (s > 0) {
    const rx = (x & s) > 0 ? 1 : 0
    const ry = (y & s) > 0 ? 1 : 0
    d += s * s * ((3 * rx) ^ ry)
    if (ry === 0) {
      if (rx === 1) {
        x = n - 1 - x
        y = n - 1 - y
      }
      const tmp = x
      x = y
      y = tmp
    }
    s >>= 1
  }
  return d
}

// JavaScript bitwise operators are 32-bit; the Rust spread() uses 64-bit masks.
// The explainer only needs p <= 8, so this bit loop is both simpler and correct.
function spreadBits(value: number): number {
  let spread = 0
  for (let bit = 0; bit < 8; bit++) {
    spread |= ((value >> bit) & 1) << (2 * bit)
  }
  return spread >>> 0
}

export function mortonIndex(x: number, y: number): number {
  return ((spreadBits(y) << 1) | spreadBits(x)) >>> 0
}

export function curveIndex(
  p: number,
  curveType: CurveType,
  x: number,
  y: number,
): number {
  return curveType === 'hilbert' ? hilbertIndex(p, x, y) : mortonIndex(x, y)
}

export function curveCells(p: number, curveType: CurveType): Cell[] {
  const n = 1 << p
  const cells: Cell[] = new Array(n * n)
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      cells[curveIndex(p, curveType, x, y)] = [x, y]
    }
  }
  return cells
}

export function countLongJumps(cells: Cell[]): number {
  let jumps = 0
  for (let i = 1; i < cells.length; i++) {
    const [x0, y0] = cells[i - 1]
    const [x1, y1] = cells[i]
    if (Math.abs(x1 - x0) + Math.abs(y1 - y0) > 1) jumps++
  }
  return jumps
}

export function orderCities(
  cities: Point[],
  p: number,
  curveType: CurveType,
): number[] {
  return buildLayout(cities, p, curveType).order
}

export function distance(a: Point, b: Point): number {
  return Math.hypot(a[0] - b[0], a[1] - b[1])
}

export function tourEdges(cities: Point[], order: number[]): TourEdge[] {
  return order.map((from, index) => {
    const to = order[(index + 1) % order.length]
    return {
      from,
      to,
      length: distance(cities[from], cities[to]),
      closing: index === order.length - 1,
    }
  })
}

export function seamEdge(edges: TourEdge[]): TourEdge {
  return edges[edges.length - 1]
}

export function longestEdge(edges: TourEdge[]): TourEdge {
  let longest = edges[0]
  for (const edge of edges) {
    if (edge.length > longest.length) longest = edge
  }
  return longest
}

function sum(values: number[]): number {
  let total = 0
  for (const value of values) total += value
  return total
}

export function tourLength(cities: Point[], order: number[]): number {
  return sum(tourEdges(cities, order).map((edge) => edge.length))
}

export function buildLayout(
  cities: Point[],
  p: number,
  curveType: CurveType,
): SfcLayout {
  const b = bounds(cities)
  const grid = projectToGrid(cities, p, b)
  const indices = grid.map(([x, y]) => curveIndex(p, curveType, x, y))

  const byCell = new Map<string, SfcBucket>()
  for (let city = 0; city < cities.length; city++) {
    const [x, y] = grid[city]
    const key = `${x},${y}`
    const existing = byCell.get(key)
    if (existing) existing.cities.push(city)
    else byCell.set(key, { cell: [x, y], index: indices[city], cities: [city] })
  }

  const buckets = [...byCell.values()].sort((a, b) => a.index - b.index)
  const order = buckets.flatMap((bucket) => bucket.cities)
  const rows = order.map((city, visit) => ({
    city,
    cell: grid[city],
    index: indices[city],
    visit,
  }))
  const edges = tourEdges(cities, order)
  const seam = seamEdge(edges)
  const longest = longestEdge(edges)
  const cells = curveCells(p, curveType)

  let largestBucket = 0
  let sharedCells = 0
  for (const bucket of buckets) {
    if (bucket.cities.length > largestBucket)
      largestBucket = bucket.cities.length
    if (bucket.cities.length > 1) sharedCells++
  }

  return {
    p,
    curveType,
    cities,
    bounds: b,
    grid,
    indices,
    cells,
    order,
    rows,
    buckets,
    edges,
    seam,
    longest,
    stats: {
      cities: cities.length,
      cellsUsed: buckets.length,
      sharedCells,
      largestBucket,
      tourLength: sum(edges.map((edge) => edge.length)),
      seamLength: seam.length,
      longestEdgeLength: longest.length,
      longJumps: countLongJumps(cells),
    },
  }
}

// Cities whose visit position changed between two layouts. The component uses
// this to explain the effect of a grid-order or curve-type change.
export function orderDiff(
  previous: SfcLayout | null,
  next: SfcLayout,
): number[] {
  if (!previous) return []
  const previousVisit = new Map<number, number>()
  previous.order.forEach((city, visit) => previousVisit.set(city, visit))
  const changed: number[] = []
  next.order.forEach((city, visit) => {
    if (previousVisit.get(city) !== visit) changed.push(city)
  })
  return changed.sort((a, b) => a - b)
}

// The best 2-opt move that removes the closing seam. For each valid partner
// edge, remove (seam, partner) and reconnect (last -> partner.from) and
// (first -> partner.to); this reverses the prefix between the two edges.
export function twoOptSeamPreview(
  cities: Point[],
  order: number[],
): TwoOptPreview | null {
  const n = order.length
  if (n < 4) return null

  const seam: TourEdge = {
    from: order[n - 1],
    to: order[0],
    length: distance(cities[order[n - 1]], cities[order[0]]),
    closing: true,
  }
  const oldLength = tourLength(cities, order)

  let best: { j: number; newLength: number } | null = null
  for (let j = 1; j <= n - 3; j++) {
    const a = order[j]
    const b = order[j + 1]
    const partnerLength = distance(cities[a], cities[b])
    const newLength =
      oldLength -
      seam.length -
      partnerLength +
      distance(cities[order[n - 1]], cities[a]) +
      distance(cities[order[0]], cities[b])
    if (!best || newLength < best.newLength) best = { j, newLength }
  }

  if (!best) return null
  const { j, newLength } = best
  const partner: TourEdge = {
    from: order[j],
    to: order[j + 1],
    length: distance(cities[order[j]], cities[order[j + 1]]),
    closing: false,
  }

  return {
    removed: [seam, partner],
    added: [
      {
        from: order[n - 1],
        to: order[j],
        length: distance(cities[order[n - 1]], cities[order[j]]),
      },
      {
        from: order[0],
        to: order[j + 1],
        length: distance(cities[order[0]], cities[order[j + 1]]),
      },
    ],
    oldLength,
    newLength,
    delta: newLength - oldLength,
  }
}

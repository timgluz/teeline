// Shared city layouts and geometry helpers for the interactive explainers.
//
// Historically every *-algo.ts file carried its own copy of CITIES / N_CITIES /
// dist / tourLength (with the same 10-city ring, a 12-city ring for the
// population-based explainers, and an 8-city ring for stochastic hill
// climbing). This module centralises them so coordinates and the distance
// formula live in one place. Each algo file re-exports the names it always
// exported (aliased to its own layout), so components and tests are untouched.
//
// Not covered here (structurally different):
//   - lk.ts  — uses a 15-city layout with euclidDist(a, b) on coordinate pairs
//              plus a distance matrix, not the index-based dist(i, j) API
//   - som-algo.ts — generates its 12 cities programmatically on a circle

export const CITIES_10: [number, number][] = [
  [150, 20],   // 0 — top
  [270, 70],   // 1 — top-right
  [260, 180],  // 2 — right
  [180, 280],  // 3 — bottom-right
  [120, 290],  // 4 — bottom
  [35, 220],   // 5 — bottom-left
  [25, 80],    // 6 — left
  [80, 25],    // 7 — top-left
  [155, 155],  // 8 — centre
  [90, 140],   // 9 — inner-left
]

export const CITIES_12: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]

export const CITIES_8: [number, number][] = [
  [150, 20],   // 0 — top
  [270, 70],   // 1 — top-right
  [260, 180],  // 2 — right
  [180, 280],  // 3 — bottom-right
  [120, 290],  // 4 — bottom
  [35, 220],   // 5 — bottom-left
  [25, 80],    // 6 — left
  [80, 25],    // 7 — top-left
]

export function makeDist(cities: [number, number][]): (i: number, j: number) => number {
  return (i, j) => {
    const [x1, y1] = cities[i]
    const [x2, y2] = cities[j]
    return Math.hypot(x2 - x1, y2 - y1)
  }
}

// Closed-cycle tour length (the wrap-around edge is included). Returns 0 for
// tours of length ≤ 1, matching the guard the population-based explainers use.
export function makeTourLength(dist: (i: number, j: number) => number): (tour: number[]) => number {
  return (tour) => {
    if (tour.length <= 1) return 0
    let d = 0
    for (let k = 0; k < tour.length; k++) {
      d += dist(tour[k], tour[(k + 1) % tour.length])
    }
    return d
  }
}

export const N_CITIES_10 = CITIES_10.length
export const N_CITIES_12 = CITIES_12.length
export const N_CITIES_8 = CITIES_8.length

export const dist10 = makeDist(CITIES_10)
export const tourLength10 = makeTourLength(dist10)
export const dist12 = makeDist(CITIES_12)
export const tourLength12 = makeTourLength(dist12)
export const dist8 = makeDist(CITIES_8)
export const tourLength8 = makeTourLength(dist8)

// Defaults (the 10-city layout) — the local-search explainers re-export these
// names directly; the 12/8-city explainers alias the *_12 / *_8 bindings.
/** Distance matrix for an arbitrary small city layout (the exact-algorithm
 * explainers — B&B, BHK — run on custom ≤6-city layouts). */
export function makeDm(cities: [number, number][]): number[][] {
  const n = cities.length
  const dm: number[][] = Array.from({ length: n }, () => new Array(n).fill(0))
  for (let i = 0; i < n; i++) {
    for (let j = i + 1; j < n; j++) {
      const d = Math.hypot(cities[i][0] - cities[j][0], cities[i][1] - cities[j][1])
      dm[i][j] = d
      dm[j][i] = d
    }
  }
  return dm
}

export const CITIES = CITIES_10
export const N_CITIES = N_CITIES_10
export const dist = dist10
export const tourLength = tourLength10

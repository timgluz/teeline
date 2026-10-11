// Scoring a TSPLIB optimal route under an instance's own distance definition.
//
// Every pairwise distance here mirrors the one the solvers are scored against
// (`src/tsp/distance_matrix.rs`), because that is what makes the length of a shipped tour
// comparable with a published optimum. EUC_2D was the last exception: it was left out while teeline
// measured it unrounded, so a derived length was neither TSPLIB's optimum nor the optimum of our
// own metric. #575 made the two the same, and this is the follow-up that removes the exception.
//
// Split out of `scripts/gen-problem-pages.mjs` so the corpus is not the only thing that can exercise
// it: every EUC_2D instance that ships a real tour also states its cost in the tour's COMMENT, so
// the fallback this module implements never runs on the data we have.

export const nint = (v) => Math.floor(v + 0.5)

// TSPLIB EUC_2D: Euclidean distance rounded to the nearest integer.
export function eucDistance(a, b) {
  return nint(Math.sqrt((a.x - b.x) ** 2 + (a.y - b.y) ** 2))
}

// TSPLIB ATT: r = sqrt((dx^2 + dy^2) / 10), then nint(r), plus one when that rounding went down.
export function attDistance(a, b) {
  const r = Math.sqrt(((a.x - b.x) ** 2 + (a.y - b.y) ** 2) / 10)
  const t = nint(r)
  return t < r ? t + 1 : t
}

// TSPLIB CEIL_2D: Euclidean distance rounded up.
export function ceilDistance(a, b) {
  return Math.ceil(Math.sqrt((a.x - b.x) ** 2 + (a.y - b.y) ** 2))
}

// TSPLIB GEO: coordinates are (degrees, minutes) pairs, and the result is truncated, not rounded.
export function geoDistance(a, b) {
  const toRad = (v) => {
    const deg = Math.trunc(v)
    return (Math.PI * (deg + (5 * (v - deg)) / 3)) / 180
  }
  // Full-precision PI, not TSPLIB's truncated 3.141592: the solver uses full precision
  // deliberately, and the two differ by one unit on ~0.09% of GEO distances, so matching it is
  // what keeps a length here comparable with a reported cost.
  const [latA, lonA, latB, lonB] = [toRad(a.x), toRad(a.y), toRad(b.x), toRad(b.y)]
  const q1 = Math.cos(lonA - lonB)
  const q2 = Math.cos(latA - latB)
  const q3 = Math.cos(latA + latB)
  // Clamped because rounding can nudge the argument just outside acos's domain, where it returns
  // NaN and poisons the total — `src/tsp/distance_matrix.rs` clamps for the same reason.
  const cosAngle = Math.min(1, Math.max(-1, 0.5 * ((1 + q1) * q2 - (1 - q1) * q3)))
  return Math.floor(6378.388 * Math.acos(cosAngle) + 1)
}

const DISTANCES = {
  EUC_2D: eucDistance,
  ATT: attDistance,
  CEIL_2D: ceilDistance,
  GEO: geoDistance,
}

/**
 * Length of `tourText`'s route over `tspText`'s coordinates, or null when it cannot be scored —
 * an unknown distance type, no coordinate section, or a route naming a city that has none.
 */
export function tourLength(tspText, tourText) {
  // `[ \t]*` around the keyword and colon: TSPLIB headers are padded in the wild, and a header
  // this misses silently turns a derivable cost into Unknown.
  const type = (/(?:^|\n)[ \t]*EDGE_WEIGHT_TYPE[ \t]*:[ \t]*(\S+)/.exec(tspText)?.[1] || '')
    .toUpperCase()
  const distance = DISTANCES[type]
  if (!distance) return null

  const coords = new Map()
  const section = tspText.match(/NODE_COORD_SECTION\s*([\s\S]*?)(?:EOF|$)/)
  if (!section) return null
  for (const line of section[1].split('\n')) {
    const parts = line.trim().split(/\s+/)
    if (parts.length >= 3) {
      const [id, x, y] = [parseInt(parts[0], 10), parseFloat(parts[1]), parseFloat(parts[2])]
      if (Number.isInteger(id) && !isNaN(x) && !isNaN(y)) coords.set(id, { x, y })
    }
  }

  const dimension = parseInt(
    /(?:^|\n)[ \t]*DIMENSION[ \t]*:[ \t]*(\d+)/.exec(tspText)?.[1] ?? '',
    10,
  )

  // Node ids are whitespace-separated, not one per line. Several shipped tours pack them (pr1002
  // writes 1002 ids across 64 lines), and reading a line at a time scored a subset of the route —
  // which is not a wrong-looking number, it is a *smaller* one, and a number from here is published
  // as a proven optimum.
  const route = (tourText.match(/TOUR_SECTION\s*([\s\S]*?)(?:-1|EOF)/)?.[1] || '')
    .split(/\s+/)
    .map((s) => parseInt(s, 10))
    .filter(Number.isInteger)

  // A route that does not name every city is not a tour of this instance. Without this the subset
  // above still produced a total (rd100's 11-of-100 ids scored 4344 against a 7910 optimum).
  if (!Number.isInteger(dimension) || route.length !== dimension) return null
  // Length alone is not enough: `[1, 2, 3, 3]` is four ids for a four-city instance that visits city
  // 3 twice and city 4 never, and it scores *shorter* than any real tour.
  if (new Set(route).size !== dimension) return null
  // The coordinate section is the instance: a file declaring 3 cities but listing 4 has a route of
  // 3 ids that is not a tour of what it describes.
  if (coords.size !== dimension) return null
  if (route.some((id) => !coords.has(id))) return null

  let total = 0
  for (let i = 0; i < route.length; i++) {
    total += distance(coords.get(route[i]), coords.get(route[(i + 1) % route.length]))
  }
  return Number.isFinite(total) ? Math.round(total) : null
}

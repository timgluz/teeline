// Shared geometry for the inline SVG charts: label placement and axis ticks.
//
// Labels: two instances a few cities apart, or twenty solvers in a cluster, otherwise draw their
// labels over each other — and a chart whose labels cannot be read hides the numbers it exists to
// show. Widths are estimated from the monospace advance rather than measured, since these are
// inline SVGs with no layout pass available before paint and the estimate only has to be an upper
// bound for collision purposes.

/**
 * Round axis ticks from zero up to at least `max`.
 *
 * A linear split of the data range produces labels like `4.4%` and `16586` — numbers that carry
 * no more information than the gridline they sit on and make the reader do arithmetic to place a
 * point. Rounded steps (1/2/5 × a power of ten) cost nothing to compute and are read at a glance.
 */
export function niceTicks(max: number, desired = 5): number[] {
  if (!Number.isFinite(max) || max <= 0) return [0, 1]

  const rough = max / Math.max(desired - 1, 1)
  const magnitude = 10 ** Math.floor(Math.log10(rough))
  const normalized = rough / magnitude
  const step =
    (normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10) *
    magnitude

  const count = Math.ceil(max / step)
  const ticks: number[] = []
  for (let index = 0; index <= count; index += 1) {
    // Multiplied rather than accumulated: repeated addition of a step like 0.2 drifts.
    ticks.push(Number((index * step).toPrecision(12)))
  }
  return ticks
}

export interface LabelRequest {
  text: string
  /** The point the label belongs to, in viewBox units. */
  x: number
  y: number
}

export interface ChartBounds {
  width: number
  height: number
}

export interface LabelPlacement {
  text: string
  x: number
  y: number
  anchor: 'start' | 'end'
}

// Tried in order: beside the point, a line below it, on its other side, above it. The second
// step is a full line plus a margin, because two points at almost the same height (ulysses16 and
// ulysses22 differ by 2px vertically) need the drop to clear the first label's box, not just its
// baseline.
const CANDIDATES = [
  { dx: 8, dy: 3, anchor: 'start' },
  { dx: 8, dy: 26, anchor: 'start' },
  { dx: -8, dy: 3, anchor: 'end' },
  { dx: 8, dy: -10, anchor: 'start' },
] as const

const CHAR_WIDTH = 7
const LINE_HEIGHT = 13
const EDGE_PADDING = 2
/** Boxes are inflated so labels end up separated, not merely non-overlapping. */
const LABEL_PADDING = 2
/** Text is drawn on its baseline, so a candidate's box starts above its anchor y. */
const BASELINE_TO_TOP = 8

export function placeLabels(
  requests: LabelRequest[],
  bounds: ChartBounds,
): LabelPlacement[] {
  const placed: { x: number; y: number; w: number; h: number }[] = []

  return requests.map((request) => {
    const width = request.text.length * CHAR_WIDTH

    for (const candidate of CANDIDATES) {
      const x =
        candidate.anchor === 'end'
          ? request.x + candidate.dx - width
          : request.x + candidate.dx
      const y = request.y + candidate.dy - BASELINE_TO_TOP
      const box = { x, y, w: width, h: LINE_HEIGHT }

      const outside =
        box.x < EDGE_PADDING ||
        box.x + box.w > bounds.width - EDGE_PADDING ||
        box.y < EDGE_PADDING ||
        box.y + box.h > bounds.height - EDGE_PADDING
      const collides = placed.some(
        (other) =>
          box.x - LABEL_PADDING < other.x + other.w + LABEL_PADDING &&
          other.x - LABEL_PADDING < box.x + box.w + LABEL_PADDING &&
          box.y - LABEL_PADDING < other.y + other.h + LABEL_PADDING &&
          other.y - LABEL_PADDING < box.y + box.h + LABEL_PADDING,
      )

      if (!outside && !collides) {
        placed.push(box)
        return {
          text: request.text,
          x: request.x + candidate.dx,
          y: request.y + candidate.dy,
          anchor: candidate.anchor,
        }
      }
    }

    // Every candidate collided: keep the label beside its point rather than dropping it.
    const fallback = CANDIDATES[0]
    placed.push({
      x: request.x + fallback.dx,
      y: request.y + fallback.dy - BASELINE_TO_TOP,
      w: width,
      h: LINE_HEIGHT,
    })
    return {
      text: request.text,
      x: request.x + fallback.dx,
      y: request.y + fallback.dy,
      anchor: fallback.anchor,
    }
  })
}

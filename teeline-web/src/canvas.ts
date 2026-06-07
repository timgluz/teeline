import type { City } from 'teeline-wasm'

interface ScaledCity {
  id: number
  x: number
  y: number
}

export function scaleCoords(
  cities: Array<{ id: number; x: number; y: number }>,
  width: number,
  height: number,
  padding: number,
): ScaledCity[] {
  if (cities.length === 0) return []

  const xs = cities.map((c) => c.x)
  const ys = cities.map((c) => c.y)
  const minX = Math.min(...xs), maxX = Math.max(...xs)
  const minY = Math.min(...ys), maxY = Math.max(...ys)
  const rangeX = maxX - minX || 1
  const rangeY = maxY - minY || 1

  const usableW = width - 2 * padding
  const usableH = height - 2 * padding
  const scale = Math.min(usableW / rangeX, usableH / rangeY)

  return cities.map((c) => ({
    id: c.id,
    x: padding + (c.x - minX) * scale,
    y: height - padding - (c.y - minY) * scale, // flip Y: TSPLIB bottom-left → SVG top-left
  }))
}

export function buildPolylinePoints(
  route: number[],
  scaledCities: ScaledCity[],
): string {
  if (route.length === 0) return ''
  const byId = new Map(scaledCities.map((c) => [c.id, c]))
  const closed = [...route, route[0]]
  return closed
    .map((id) => {
      const c = byId.get(id)
      return c ? `${c.x},${c.y}` : ''
    })
    .filter(Boolean)
    .join(' ')
}

const SVG_NS = 'http://www.w3.org/2000/svg'
const W = 600
const H = 400
const PAD = 20

function el<K extends keyof SVGElementTagNameMap>(tag: K): SVGElementTagNameMap[K] {
  return document.createElementNS(SVG_NS, tag)
}

export function renderTour(
  svgEl: SVGSVGElement,
  cities: City[],
  solverRoute: number[],
  optimalRoute?: number[],
): void {
  svgEl.innerHTML = ''
  svgEl.setAttribute('viewBox', `0 0 ${W} ${H}`)

  const scaled = scaleCoords(cities, W, H, PAD)

  // Optimal ghost (drawn first, behind solver tour)
  if (optimalRoute && optimalRoute.length > 0) {
    const poly = el('polyline')
    poly.setAttribute('class', 'tour-optimal')
    poly.setAttribute('points', buildPolylinePoints(optimalRoute, scaled))
    svgEl.appendChild(poly)
  }

  // Solver tour
  if (solverRoute.length > 0) {
    const poly = el('polyline')
    poly.setAttribute('class', 'tour-solver')
    poly.setAttribute('points', buildPolylinePoints(solverRoute, scaled))
    svgEl.appendChild(poly)
  }

  // City dots
  for (const c of scaled) {
    const circle = el('circle')
    circle.setAttribute('class', 'city-dot')
    circle.setAttribute('cx', String(c.x))
    circle.setAttribute('cy', String(c.y))
    circle.setAttribute('r', '3')
    svgEl.appendChild(circle)
  }

  // Legend
  const legend = el('g')
  legend.setAttribute('class', 'tour-legend')
  legend.setAttribute('transform', `translate(${PAD}, ${H - PAD})`)

  const solverLabel = el('text')
  solverLabel.setAttribute('class', 'legend-solver')
  solverLabel.setAttribute('x', '0')
  solverLabel.setAttribute('y', '0')
  solverLabel.textContent = '— solver'
  legend.appendChild(solverLabel)

  if (optimalRoute && optimalRoute.length > 0) {
    const optLabel = el('text')
    optLabel.setAttribute('class', 'legend-optimal')
    optLabel.setAttribute('x', '80')
    optLabel.setAttribute('y', '0')
    optLabel.textContent = '┄ optimal'
    legend.appendChild(optLabel)
  }

  svgEl.appendChild(legend)
}

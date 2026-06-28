// 12 cities on a circle, canvas 300×300
const CX = 150, CY = 150, CITY_R = 110
export const N_CITIES = 12
export const CITIES: [number, number][] = Array.from({ length: N_CITIES }, (_, i) => {
  const theta = (2 * Math.PI * i) / N_CITIES
  return [CX + CITY_R * Math.cos(theta), CY + CITY_R * Math.sin(theta)] as [number, number]
})

export const N_NEURONS = Math.round(N_CITIES * 1.5)  // 18
export const ALPHA0 = 0.8
export const SIGMA0 = 4.0
const SIGMA_FLOOR = 1.0
export const MAX_STEPS = 200
const H_THRESHOLD = 0.01

export type Phase = 'init' | 'expanding' | 'converging' | 'fine-tuning' | 'done'

export type SomState = {
  neurons: [number, number][]
  step: number
  alpha: number
  sigma: number
  bmu: number | null
  neighbors: number[]
  lastCityIdx: number | null
  phase: Phase
  tour: number[] | null
  lastTourLength: number | null
}

export function dist([x1, y1]: [number, number], [x2, y2]: [number, number]): number {
  return Math.hypot(x2 - x1, y2 - y1)
}

export function tourLength(tour: number[]): number {
  const n = tour.length
  if (n <= 1) return 0
  let d = 0
  for (let i = 0; i < n; i++) {
    d += dist(CITIES[tour[i]], CITIES[tour[(i + 1) % n]])
  }
  return d
}

function ringDist(i: number, j: number): number {
  const d = Math.abs(i - j)
  return Math.min(d, N_NEURONS - d)
}

function gaussian(dRing: number, sigma: number): number {
  return Math.exp(-(dRing * dRing) / (2 * sigma * sigma))
}

function extractTour(neurons: [number, number][]): number[] {
  const cityBmu = CITIES.map(city =>
    neurons
      .map((n, ni) => ({ ni, d: dist(n, city) }))
      .sort((a, b) => a.d - b.d || a.ni - b.ni)[0].ni
  )
  return Array.from({ length: N_CITIES }, (_, ci) => ci).sort((a, b) => {
    if (cityBmu[a] !== cityBmu[b]) return cityBmu[a] - cityBmu[b]
    return dist(neurons[cityBmu[a]], CITIES[a]) - dist(neurons[cityBmu[b]], CITIES[b])
  })
}

export function makeInitState(): SomState {
  const neurons: [number, number][] = Array.from({ length: N_NEURONS }, (_, i) => {
    const theta = (2 * Math.PI * i) / N_NEURONS
    return [CX + 15 * Math.cos(theta), CY + 15 * Math.sin(theta)] as [number, number]
  })
  return {
    neurons,
    step: 0,
    alpha: ALPHA0,
    sigma: SIGMA0,
    bmu: null,
    neighbors: [],
    lastCityIdx: null,
    phase: 'init',
    tour: null,
    lastTourLength: null,
  }
}

export function stepOnce(s: SomState): SomState {
  if (s.phase === 'done') return s

  if (s.step >= MAX_STEPS) {
    const tour = extractTour(s.neurons)
    return {
      ...s,
      phase: 'done',
      tour,
      lastTourLength: tourLength(tour),
      bmu: null,
      neighbors: [],
      lastCityIdx: null,
    }
  }

  const t = s.step + 1
  const alpha = ALPHA0 * Math.exp(-t / MAX_STEPS)
  const sigma = Math.max(SIGMA_FLOOR, SIGMA0 * Math.exp(-t / MAX_STEPS))

  const cityIdx = Math.floor(Math.random() * N_CITIES)
  const city = CITIES[cityIdx]

  // Find BMU (closest neuron to city)
  let bmu = 0, bmuD = Infinity
  for (let i = 0; i < N_NEURONS; i++) {
    const d = dist(s.neurons[i], city)
    if (d < bmuD) { bmuD = d; bmu = i }
  }

  // Update all neurons via Gaussian neighbourhood kernel (ring topology)
  const newNeurons: [number, number][] = s.neurons.map(([nx, ny], i) => {
    const h = gaussian(ringDist(i, bmu), sigma)
    if (h < H_THRESHOLD) return [nx, ny]
    return [nx + alpha * h * (city[0] - nx), ny + alpha * h * (city[1] - ny)]
  })

  // Neighbours: h > 0.1 (visually significant pull), excluding BMU itself
  const neighbors: number[] = []
  for (let i = 0; i < N_NEURONS; i++) {
    if (i !== bmu && gaussian(ringDist(i, bmu), sigma) > 0.1) neighbors.push(i)
  }

  const phase: Phase =
    sigma > SIGMA0 * 0.5 ? 'expanding'
    : sigma > SIGMA_FLOOR * 2 ? 'converging'
    : 'fine-tuning'

  return { ...s, neurons: newNeurons, step: t, alpha, sigma, bmu, neighbors, lastCityIdx: cityIdx, phase }
}

// Convert σ (neuron-ring-index units) to canvas pixels for the radius circle.
// Uses average neuron ring radius so the circle scales as the ring expands.
export function neighbourRadiusPx(sigma: number, neurons: [number, number][]): number {
  const cx = neurons.reduce((s, [x]) => s + x, 0) / neurons.length
  const cy = neurons.reduce((s, [, y]) => s + y, 0) / neurons.length
  const avgR = neurons.reduce((s, [x, y]) => s + Math.hypot(x - cx, y - cy), 0) / neurons.length
  const arcPerNeuron = (2 * Math.PI * Math.max(avgR, 10)) / N_NEURONS
  return sigma * arcPerNeuron
}

export function phaseLabel(phase: Phase, lastTourLength: number | null): string {
  switch (phase) {
    case 'init':        return 'Neurons initialised in a small ring around the centroid.'
    case 'expanding':   return 'Wide neighbourhood — ring expands and rotates to cover city clusters.'
    case 'converging':  return 'Neighbourhood shrinking — neurons approach individual cities.'
    case 'fine-tuning': return 'Fine-tuning — neurons lock onto city positions.'
    case 'done':        return `Tour extracted from ring order. Length: ${lastTourLength?.toFixed(0) ?? '—'}`
  }
}

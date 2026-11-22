import { berlin52, type HeroCity } from './hero-cities'

/** How long an ordinary 2-opt frame stays on screen. */
export const FRAME_DWELL_MS = 240
/** The break on the converged tour before a fresh random run starts. */
export const HOLD_AT_OPTIMUM_MS = 5000
/** A shorter look at the random seed before the improvements start. */
export const HOLD_AT_SEED_MS = 900

const ROUTE_FROM = '#6366f1'
const ROUTE_TO = '#22d3a5'
const CITY_FILL = '#6366f1'
const CITY_STROKE = '#0a0a0b'
const CITY_DRAGGING = '#22d3a5'

/**
 * The dwell for a frame before the loop advances.
 *
 * Endpoints read as a break in the loop rather than another step: the converged tour gets the
 * longest hold, so the eye can see what the solver settled on before a fresh random tour begins.
 */
export function frameDwell(frameIdx: number, optimumIndex: number): number {
  if (frameIdx === optimumIndex) return HOLD_AT_OPTIMUM_MS
  if (frameIdx === 0) return HOLD_AT_SEED_MS
  return FRAME_DWELL_MS
}

interface Frame {
  route: number[]
  cost: number
}

export function initHeroCanvas(): void {
  const svg = document.getElementById(
    'hero-canvas',
  ) as unknown as SVGSVGElement | null
  const lengthEl = document.getElementById('hero-tour-length')
  const iterEl = document.getElementById('hero-iterations')
  const bestEl = document.getElementById('hero-best')
  const statusEl = document.getElementById('hero-solve-status')
  if (!svg || !lengthEl || !iterEl || !bestEl || !statusEl) return
  const root = svg
  const length = lengthEl
  const iterations = iterEl
  const bestNode = bestEl
  const status = statusEl

  function setStatus(solved: boolean): void {
    status.textContent = solved ? 'solved' : 'solving'
    status.classList.toggle('text-positive', solved)
    status.classList.toggle('text-accent', !solved)
  }

  const NS = 'http://www.w3.org/2000/svg'
  const cities: HeroCity[] = berlin52.map((c) => ({ id: c.id, x: c.x, y: c.y }))

  function dist(a: number, b: number): number {
    const dx = cities[a].x - cities[b].x
    const dy = cities[a].y - cities[b].y
    return Math.sqrt(dx * dx + dy * dy)
  }

  function routeCost(route: number[]): number {
    let cost = 0
    for (let i = 0; i < route.length; i++) {
      cost += dist(route[i], route[(i + 1) % route.length])
    }
    return cost
  }

  function randomSeed(): number[] {
    const order = cities.map((_, i) => i)
    for (let i = order.length - 1; i > 0; i -= 1) {
      const j = Math.floor(Math.random() * (i + 1))
      const swap = order[i]
      order[i] = order[j]
      order[j] = swap
    }
    return order
  }

  function buildFrames(seed: number[]): {
    frames: Frame[]
    optimumIndex: number
  } {
    const route = seed.slice()
    const frames: Frame[] = []
    frames.push({ route: route.slice(), cost: routeCost(route) })
    let improved = true
    while (improved) {
      improved = false
      for (let i = 0; i < route.length - 1; i++) {
        for (let k = i + 1; k < route.length; k++) {
          const d = route[i]
          const c = route[(i + 1) % route.length]
          const b = route[k]
          const a = route[(k + 1) % route.length]
          if (dist(d, c) + dist(b, a) > dist(d, b) + dist(c, a)) {
            route.splice(i + 1, k - i, ...route.slice(i + 1, k + 1).reverse())
            improved = true
            frames.push({ route: route.slice(), cost: routeCost(route) })
          }
        }
      }
    }
    return { frames, optimumIndex: frames.length - 1 }
  }

  let currentSeed = randomSeed()
  let built = buildFrames(currentSeed)
  let frames = built.frames
  let optimumIndex = built.optimumIndex
  let best = Math.min(...frames.map((f) => f.cost))

  const reduceMotion =
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false

  // The gradient spans the viewBox rather than the path's bounding box, so the colours stay put
  // as the tour changes shape between frames.
  const defs = document.createElementNS(NS, 'defs')
  const gradient = document.createElementNS(NS, 'linearGradient')
  gradient.setAttribute('id', 'hero-route-gradient')
  gradient.setAttribute('gradientUnits', 'userSpaceOnUse')
  gradient.setAttribute('x1', '0')
  gradient.setAttribute('y1', '0')
  gradient.setAttribute('x2', '1800')
  gradient.setAttribute('y2', '1200')
  for (const [offset, colour] of [
    ['0%', ROUTE_FROM],
    ['100%', ROUTE_TO],
  ] as const) {
    const stop = document.createElementNS(NS, 'stop')
    stop.setAttribute('offset', offset)
    stop.setAttribute('stop-color', colour)
    gradient.appendChild(stop)
  }
  defs.appendChild(gradient)
  root.appendChild(defs)

  const pathEl = document.createElementNS(NS, 'path')
  pathEl.setAttribute('fill', 'none')
  pathEl.setAttribute('stroke', 'url(#hero-route-gradient)')
  pathEl.setAttribute('stroke-width', '3')
  pathEl.setAttribute('stroke-linecap', 'round')
  pathEl.setAttribute('stroke-linejoin', 'round')
  root.appendChild(pathEl)

  const nodeG = document.createElementNS(NS, 'g')
  root.appendChild(nodeG)

  const nodeEls: (SVGCircleElement | null)[] = []
  cities.forEach((c) => {
    const circle = document.createElementNS(NS, 'circle')
    circle.setAttribute('cx', String(c.x))
    circle.setAttribute('cy', String(c.y))
    circle.setAttribute('r', '10')
    circle.setAttribute('fill', CITY_FILL)
    circle.setAttribute('stroke', CITY_STROKE)
    circle.setAttribute('stroke-width', '2')
    circle.setAttribute('cursor', 'grab')
    nodeG.appendChild(circle)
    nodeEls.push(circle)
  })

  function renderRoute(route: number[]): void {
    let d = ''
    route.forEach((idx, i) => {
      const c = cities[idx]
      d += (i === 0 ? 'M' : 'L') + c.x + ' ' + c.y
    })
    d += 'Z'
    pathEl.setAttribute('d', d)
    // Keep drawn circles above the path
    nodeG.parentNode?.appendChild(nodeG)
  }

  let dragging: number | null = null
  function svgPoint(evt: PointerEvent): { x: number; y: number } {
    const rect = root.getBoundingClientRect()
    const scaleX = 1800 / rect.width
    const scaleY = 1200 / rect.height
    return {
      x: (evt.clientX - rect.left) * scaleX,
      y: (evt.clientY - rect.top) * scaleY,
    }
  }

  function onDown(i: number, evt: PointerEvent): void {
    evt.preventDefault()
    dragging = i
    root.setPointerCapture(evt.pointerId)
    nodeEls[i]?.setAttribute('cursor', 'grabbing')
    nodeEls[i]?.setAttribute('fill', CITY_DRAGGING)
  }

  nodeEls.forEach((el, i) => {
    el?.addEventListener('pointerdown', (e) => onDown(i, e))
  })

  let frameIdx = reduceMotion ? optimumIndex : 0
  let iterationCount = reduceMotion ? optimumIndex : 0
  let lastFrame = frames[frameIdx].cost
  let lastAdvance = performance.now()

  root.addEventListener('pointermove', (evt) => {
    if (dragging === null) return
    const p = svgPoint(evt)
    cities[dragging].x = Math.max(0, Math.min(1800, p.x))
    cities[dragging].y = Math.max(0, Math.min(1200, p.y))
    nodeEls[dragging]?.setAttribute('cx', String(cities[dragging].x))
    nodeEls[dragging]?.setAttribute('cy', String(cities[dragging].y))
    const rebuilt = buildFrames(currentSeed)
    frames = rebuilt.frames
    optimumIndex = rebuilt.optimumIndex
    best = Math.min(best, ...frames.map((f) => f.cost))
    bestNode.textContent = best.toLocaleString('en-US', {
      maximumFractionDigits: 0,
    })
    frameIdx = reduceMotion ? optimumIndex : 0
    iterationCount = reduceMotion ? optimumIndex : 0
    lastFrame = frames[frameIdx].cost
    lastAdvance = performance.now()
    renderRoute(frames[frameIdx].route)
    length.textContent = Math.round(frames[frameIdx].cost).toLocaleString(
      'en-US',
    )
    iterations.textContent = String(iterationCount)
    setStatus(reduceMotion)
  })

  function onUp(): void {
    dragging = null
    nodeEls.forEach((el) => {
      el?.setAttribute('cursor', 'grab')
      el?.setAttribute('fill', CITY_FILL)
    })
  }
  root.addEventListener('pointerup', onUp)
  root.addEventListener('pointercancel', onUp)

  function step(now: number): void {
    if (
      dragging === null &&
      now - lastAdvance >= frameDwell(frameIdx, optimumIndex)
    ) {
      if (frameIdx === optimumIndex) {
        // The run converged and the break is over: start a fresh random tour.
        currentSeed = randomSeed()
        const rebuilt = buildFrames(currentSeed)
        frames = rebuilt.frames
        optimumIndex = rebuilt.optimumIndex
        best = Math.min(best, ...frames.map((f) => f.cost))
        frameIdx = 0
        iterationCount = 0
        lastFrame = frames[0].cost
        renderRoute(frames[0].route)
        length.textContent = Math.round(frames[0].cost).toLocaleString('en-US')
        bestNode.textContent = Math.round(best).toLocaleString('en-US')
        iterations.textContent = '0'
        setStatus(false)
      } else {
        frameIdx += 1
        const f = frames[frameIdx]
        renderRoute(f.route)
        length.textContent = Math.round(f.cost).toLocaleString('en-US')
        if (f.cost < lastFrame) {
          iterationCount++
        }
        if (f.cost <= best) {
          best = f.cost
        }
        bestNode.textContent = Math.round(best).toLocaleString('en-US')
        iterations.textContent = String(iterationCount)
        lastFrame = f.cost
        setStatus(frameIdx === optimumIndex)
      }
      lastAdvance = now
    }
    requestAnimationFrame(step)
  }

  renderRoute(frames[frameIdx].route)
  length.textContent = Math.round(frames[frameIdx].cost).toLocaleString('en-US')
  bestNode.textContent = Math.round(best).toLocaleString('en-US')
  iterations.textContent = String(iterationCount)
  setStatus(frameIdx === optimumIndex)
  if (!reduceMotion) requestAnimationFrame(step)
}

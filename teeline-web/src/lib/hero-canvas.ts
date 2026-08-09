import { berlin52, type HeroCity } from './hero-cities'

export function initHeroCanvas(): void {
  const svg = document.getElementById('hero-canvas') as unknown as SVGSVGElement | null
  const lengthEl = document.getElementById('hero-tour-length')
  const iterEl = document.getElementById('hero-iterations')
  const bestEl = document.getElementById('hero-best')
  if (!svg || !lengthEl || !iterEl || !bestEl) return
  const root = svg
  const length = lengthEl
  const iterations = iterEl
  const bestNode = bestEl

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

  function nearestNeighborSeed(): number[] {
    const unvisited = cities.map((_, i) => i)
    const route = [unvisited.shift()!]
    let cur = route[0]
    while (unvisited.length) {
      let bestIdx = 0
      let bestD = Infinity
      unvisited.forEach((c, i) => {
        const d = dist(cur, c)
        if (d < bestD) {
          bestD = d
          bestIdx = i
        }
      })
      const next = unvisited.splice(bestIdx, 1)[0]
      route.push(next)
      cur = next
    }
    return route
  }

  function buildFrames(): { route: number[]; cost: number }[] {
    let route = nearestNeighborSeed()
    const frames: { route: number[]; cost: number }[] = []
    frames.push({ route: route.slice(), cost: routeCost(route) })
    let improved = true
    while (improved && frames.length < 120) {
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
            if (frames.length >= 120) break
          }
        }
        if (frames.length >= 120) break
      }
    }
    const reversed = frames.slice(1, -1).reverse()
    return frames.concat(reversed)
  }

  let frames = buildFrames()
  let best = Math.min(...frames.map((f) => f.cost))

  const pathEl = document.createElementNS(NS, 'path')
  pathEl.setAttribute('fill', 'none')
  pathEl.setAttribute('stroke', '#6366F1')
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
    circle.setAttribute('fill', '#141416')
    circle.setAttribute('stroke', '#8A8A92')
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
  }

  nodeEls.forEach((el, i) => {
    el?.addEventListener('pointerdown', (e) => onDown(i, e))
  })

  root.addEventListener('pointermove', (evt) => {
    if (dragging === null) return
    const p = svgPoint(evt)
    cities[dragging].x = Math.max(0, Math.min(1800, p.x))
    cities[dragging].y = Math.max(0, Math.min(1200, p.y))
    nodeEls[dragging]?.setAttribute('cx', String(cities[dragging].x))
    nodeEls[dragging]?.setAttribute('cy', String(cities[dragging].y))
    frames = buildFrames()
    best = Math.min(best, ...frames.map((f) => f.cost))
    bestNode.textContent = best.toLocaleString('en-US', { maximumFractionDigits: 0 })
    renderRoute(frames[0].route)
    length.textContent = Math.round(frames[0].cost).toLocaleString('en-US')
    iterations.textContent = '0'
    frameIdx = 0
  })

  function onUp(): void {
    dragging = null
    nodeEls.forEach((el) => el?.setAttribute('cursor', 'grab'))
  }
  root.addEventListener('pointerup', onUp)
  root.addEventListener('pointercancel', onUp)

  let frameIdx = 0
  let iterationCount = 0
  let lastFrame = frames[0].cost

  function step(): void {
    if (dragging === null) {
      frameIdx = (frameIdx + 1) % frames.length
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
      if (frameIdx === 0) iterationCount = 0
    }
    requestAnimationFrame(step)
  }

  renderRoute(frames[0].route)
  length.textContent = Math.round(frames[0].cost).toLocaleString('en-US')
  bestNode.textContent = Math.round(best).toLocaleString('en-US')
  iterations.textContent = '0'
  requestAnimationFrame(step)
}

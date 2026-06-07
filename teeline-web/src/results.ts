import type { City } from 'teeline-wasm'
import { renderTour } from './canvas'

export interface RunRecord {
  solver: string
  total: number
  optTotal?: number
  runtime: number
  route: number[]
}

// ---- Pure helpers ----

export function formatGap(total: number, optimal: number | undefined): string {
  if (optimal === undefined) return '—'
  const pct = ((total - optimal) / optimal) * 100
  return `${pct.toFixed(1)}%`
}

export function formatRuntime(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

export function computeRouteLength(
  route: number[],
  cities: Array<{ id: number; x: number; y: number }>,
): number {
  if (route.length < 2) return 0
  const byId = new Map(cities.map((c) => [c.id, c]))
  let total = 0
  for (let i = 0; i < route.length; i++) {
    const a = byId.get(route[i])
    const b = byId.get(route[(i + 1) % route.length])
    if (!a || !b) continue
    const dx = b.x - a.x
    const dy = b.y - a.y
    total += Math.sqrt(dx * dx + dy * dy)
  }
  return total
}

// ---- DOM module ----

let citiesRef: City[] = []
let optRouteRef: number[] | undefined
let runHistory: RunRecord[] = []

export function initResults(
  cities: City[],
  getOptRoute: () => number[] | null,
  onTryAgain: () => void,
): void {
  citiesRef = cities
  runHistory = []

  document.getElementById('btn-try-again')!.addEventListener('click', () => {
    const step04 = document.getElementById('step-04') as HTMLElement
    const step02 = document.getElementById('step-02') as HTMLElement
    step04.hidden = true
    step02.hidden = false
    resetStepperToStep2()
    onTryAgain()
  })

  optRouteRef = getOptRoute() ?? undefined
}

export function updateOptRoute(route: number[] | null): void {
  optRouteRef = route ?? undefined
}

export function showRunning(): void {
  const overlay = document.getElementById('solving-overlay') as HTMLElement
  overlay.hidden = false
}

export function showResult(record: RunRecord): void {
  const overlay = document.getElementById('solving-overlay') as HTMLElement
  overlay.hidden = true

  const svgEl = document.getElementById('tour-svg') as unknown as SVGSVGElement
  renderTour(svgEl, citiesRef, record.route, record.optTotal !== undefined ? optRouteRef : undefined)

  const totalEl = document.getElementById('result-total')!
  const gapEl = document.getElementById('result-gap')!
  const runtimeEl = document.getElementById('result-runtime')!

  totalEl.textContent = record.total.toFixed(1)
  gapEl.textContent = formatGap(record.total, record.optTotal)
  runtimeEl.textContent = formatRuntime(record.runtime)

  runHistory.unshift(record)
  renderHistoryRow(record)
}

function renderHistoryRow(record: RunRecord): void {
  const list = document.getElementById('run-history-list')!
  const li = document.createElement('li')
  li.className = 'run-history-item'
  li.textContent = `${record.solver} — ${record.total.toFixed(1)} ${formatGap(record.total, record.optTotal)} ${formatRuntime(record.runtime)}`
  li.setAttribute('role', 'button')
  li.setAttribute('tabindex', '0')
  li.addEventListener('click', () => replayRecord(record))
  li.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') replayRecord(record) })
  list.prepend(li)
}

function replayRecord(record: RunRecord): void {
  const svgEl = document.getElementById('tour-svg') as unknown as SVGSVGElement
  renderTour(svgEl, citiesRef, record.route, record.optTotal !== undefined ? optRouteRef : undefined)

  document.getElementById('result-total')!.textContent = record.total.toFixed(1)
  document.getElementById('result-gap')!.textContent = formatGap(record.total, record.optTotal)
  document.getElementById('result-runtime')!.textContent = formatRuntime(record.runtime)
}

function resetStepperToStep2(): void {
  const steps = document.querySelectorAll<HTMLElement>('.stepper-step')
  steps.forEach((step, i) => {
    if (i < 1) {
      step.classList.add('stepper-step--done')
      step.classList.remove('stepper-step--active')
      step.removeAttribute('aria-current')
    } else if (i === 1) {
      step.classList.add('stepper-step--active')
      step.classList.remove('stepper-step--done')
      step.setAttribute('aria-current', 'step')
    } else {
      step.classList.remove('stepper-step--active', 'stepper-step--done')
      step.removeAttribute('aria-current')
    }
  })
}

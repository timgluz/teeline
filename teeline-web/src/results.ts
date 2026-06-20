import type { City } from 'teeline-wasm'
import { renderTour } from './canvas'
import type { ComparisonStats } from './worker'

export interface RunRecord {
  solver: string
  total: number
  comparison?: ComparisonStats
  runtime: number
  route: number[]
}

// ---- Pure helpers ----

export function formatRuntime(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function formatGapPct(c: ComparisonStats | undefined): string {
  return c !== undefined ? `${c.gapPct.toFixed(1)}%` : '—'
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
  renderTour(svgEl, citiesRef, record.route, record.comparison !== undefined ? optRouteRef : undefined)

  document.getElementById('result-total')!.textContent = record.total.toFixed(1)
  document.getElementById('result-gap')!.textContent = formatGapPct(record.comparison)
  document.getElementById('result-runtime')!.textContent = formatRuntime(record.runtime)
  applyComparisonCells(record.comparison)

  runHistory.unshift(record)
  renderHistoryRow(record)
}

export function patchComparison(record: RunRecord, stats: ComparisonStats): void {
  record.comparison = stats
  document.getElementById('result-gap')!.textContent = formatGapPct(stats)
  applyComparisonCells(stats)
  const svgEl = document.getElementById('tour-svg') as unknown as SVGSVGElement
  renderTour(svgEl, citiesRef, record.route, optRouteRef)
}

function applyComparisonCells(c: ComparisonStats | undefined): void {
  const dash = '—'
  document.getElementById('result-shared-edges')!.textContent = c !== undefined ? String(c.sharedEdges) : dash
  document.getElementById('result-solver-only-edges')!.textContent = c !== undefined ? String(c.solverOnlyEdges) : dash
  document.getElementById('result-optimal-only-edges')!.textContent = c !== undefined ? String(c.optimalOnlyEdges) : dash
}

function renderHistoryRow(record: RunRecord): void {
  const list = document.getElementById('run-history-list')!
  const li = document.createElement('li')
  li.className = 'run-history-item'
  li.textContent = `${record.solver} — ${record.total.toFixed(1)} ${formatGapPct(record.comparison)} ${formatRuntime(record.runtime)}`
  li.setAttribute('role', 'button')
  li.setAttribute('tabindex', '0')
  li.addEventListener('click', () => replayRecord(record))
  li.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') replayRecord(record) })
  list.prepend(li)
}

function replayRecord(record: RunRecord): void {
  const svgEl = document.getElementById('tour-svg') as unknown as SVGSVGElement
  renderTour(svgEl, citiesRef, record.route, record.comparison !== undefined ? optRouteRef : undefined)

  document.getElementById('result-total')!.textContent = record.total.toFixed(1)
  document.getElementById('result-gap')!.textContent = formatGapPct(record.comparison)
  document.getElementById('result-runtime')!.textContent = formatRuntime(record.runtime)
  applyComparisonCells(record.comparison)
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

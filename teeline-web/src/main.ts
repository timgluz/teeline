import './sentry.js'
import 'htmx.org'
import '@picocss/pico/css/pico.min.css'
import './main.css'
import type { ParsedProblem } from 'teeline-wasm'
import { type SolveOptions } from './solver-options'
import type { SolveResult, SolveError, ParseResult } from './worker'
import { initUpload, resetUpload } from './upload'
import { initSolverConfig } from './solver-form'
import { initResults, updateOptRoute, showRunning, showResult, computeRouteLength } from './results'
import { buildTourText, buildCsvText, buildJsonText, serializeSvg, triggerDownload } from './download'

const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' })

export function runSolver(
  solver: string,
  cities: Array<{ id: number; x: number; y: number }>,
  options: Partial<SolveOptions> = {},
): Promise<{ total: number; route: number[] }> {
  return new Promise((resolve, reject) => {
    const handler = (e: MessageEvent<SolveResult | SolveError>) => {
      worker.removeEventListener('message', handler)
      if (e.data.type === 'result') {
        resolve(e.data.solution)
        document.dispatchEvent(
          new CustomEvent('solver:result', { detail: e.data.solution, bubbles: true }),
        )
      } else {
        const err = new Error(e.data.message)
        reject(err)
        document.dispatchEvent(
          new CustomEvent('solver:error', { detail: e.data.message, bubbles: true }),
        )
      }
    }
    worker.addEventListener('message', handler)
    worker.postMessage({ type: 'solve', solver, cities, options })
  })
}

export function parseFile(input: string): Promise<ParsedProblem> {
  return new Promise((resolve, reject) => {
    const handler = (e: MessageEvent<ParseResult | SolveError>) => {
      worker.removeEventListener('message', handler)
      if (e.data.type === 'parsed') {
        resolve(e.data.problem)
      } else if (e.data.type === 'error') {
        reject(new Error(e.data.message))
      }
    }
    worker.addEventListener('message', handler)
    worker.postMessage({ type: 'parse', input })
  })
}

// Expose for browser console smoke test and HTMX scripts
declare global {
  interface Window {
    runSolver: typeof runSolver
    parseFile: typeof parseFile
  }
}
window.runSolver = runSolver
window.parseFile = parseFile

// ---- App state ----

let parsedProblem: ParsedProblem | null = null
let optTourRoute: number[] | null = null

// ---- Results (init before solver config so showRunning is ready) ----

initResults(
  [],          // cities injected per-run via showResult; updated below
  () => optTourRoute,
  () => {
    // "try another solver" — solver-form re-shows step-02 via its own stepper logic
  },
)

// ---- Solver config ----

const solverConfig = initSolverConfig(
  () => parsedProblem !== null,
  (solver, options) => {
    if (!parsedProblem) return

    // Re-init results with current cities so renderTour has the right data
    initResults(
      parsedProblem.cities,
      () => optTourRoute,
      () => {
        const step04 = document.getElementById('step-04') as HTMLElement
        const step02 = document.getElementById('step-02') as HTMLElement
        step04.hidden = true
        step02.hidden = false
        ;(document.getElementById('download-actions') as HTMLElement).hidden = true
      },
    )

    showRunning()

    const start = Date.now()
    runSolver(solver, parsedProblem.cities, options)
      .then((result) => {
        const runtime = Date.now() - start
        const optTotal = optTourRoute
          ? computeRouteLength(optTourRoute, parsedProblem!.cities)
          : undefined
        const record = { solver, total: result.total, optTotal, runtime, route: result.route }
        showResult(record)
        showDownloadButtons(record, parsedProblem!)
      })
      .catch((err: Error) => {
        const overlay = document.getElementById('solving-overlay') as HTMLElement
        overlay.hidden = true
        console.error('Solver error:', err)
      })
  },
)

// ---- Upload (after solverConfig so refresh is available) ----

initUpload(
  parseFile,
  (p) => { parsedProblem = p; solverConfig.refresh() },
  (route) => {
    optTourRoute = route.length > 0 ? route : null
    updateOptRoute(optTourRoute)
  },
)

// ---- Reset / new dataset ----

function resetToStep01(): void {
  parsedProblem = null
  optTourRoute = null
  resetUpload()
  ;(document.getElementById('step-01') as HTMLElement).hidden = false
  ;(document.getElementById('step-02') as HTMLElement).hidden = true
  ;(document.getElementById('step-04') as HTMLElement).hidden = true
  ;(document.getElementById('download-actions') as HTMLElement).hidden = true
}

document.getElementById('btn-change-file')!.addEventListener('click', resetToStep01)
document.getElementById('btn-new-dataset')!.addEventListener('click', resetToStep01)

// ---- Download wiring ----

function showDownloadButtons(record: import('./results').RunRecord, problem: ParsedProblem): void {
  const actions = document.getElementById('download-actions') as HTMLElement
  actions.hidden = false

  const { name, cities } = problem
  const { route } = record
  const svgEl = document.getElementById('tour-svg') as unknown as SVGSVGElement
  const ts = Date.now()
  const sessionId = crypto.randomUUID()

  document.getElementById('btn-download-tour')!.onclick = () =>
    triggerDownload(buildTourText(name, route), `${sessionId}.tour`, 'text/plain')

  document.getElementById('btn-download-csv')!.onclick = () =>
    triggerDownload(buildCsvText(route, cities), `${sessionId}.csv`, 'text/csv')

  document.getElementById('btn-download-json')!.onclick = () =>
    triggerDownload(buildJsonText(name, record, cities, ts), `${sessionId}.json`, 'application/json')

  document.getElementById('btn-download-svg')!.onclick = () =>
    triggerDownload(serializeSvg(svgEl), `${sessionId}.svg`, 'image/svg+xml')
}

import '@picocss/pico/css/pico.min.css'
import './main.css'
import './docs.css'
import { initTopbar } from './topbar'
import type { ParsedProblem } from 'teeline-wasm'
import { type SolveOptions } from './solver-options'
import type { SolveResult, SolveError, ParseResult, AlgorithmsResult, VersionResult, WorkerReadyMessage, CompareToursResult } from './worker'
import { initUpload, resetUpload } from './upload'
import { initSolverConfig } from './solver-form'
import { initResults, updateOptRoute, showRunning, showResult, patchComparison } from './results'
import { buildTourText, buildCsvText, buildJsonText, serializeSvg, triggerDownload } from './download'

initTopbar()
window.addEventListener('load', () => import('./sentry'), { once: true })

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
let solverConfig: ReturnType<typeof initSolverConfig> | null = null

// ---- Footer status helpers ----

function setWasmStatus(state: 'loading' | 'ready' | 'error'): void {
  const dot = document.getElementById('wasm-status')
  if (!dot) return
  dot.className = `status-dot status-dot--${state}`
}

// ---- Results (init before solver config so showRunning is ready) ----

initResults(
  [],          // cities injected per-run via showResult; updated below
  () => optTourRoute,
  () => {
    // "try another solver" — solver-form re-shows step-02 via its own stepper logic
  },
)

// ---- WASM init: wait for worker-ready, then fetch algorithm list + version ----

setWasmStatus('loading')
const versionEl = document.getElementById('wasm-version')
if (versionEl) versionEl.textContent = 'Connecting…'

worker.addEventListener('error', () => {
  setWasmStatus('error')
  if (versionEl) versionEl.textContent = 'WASM failed to load — try refreshing the page'
})

let gotAlgorithms = false
let gotVersion = false

worker.addEventListener('message', function onInit(e: MessageEvent<WorkerReadyMessage | AlgorithmsResult | VersionResult>) {
  const data = e.data

  if (data.type === 'worker-ready') {
    // WASM is fully initialised — safe to call listAlgorithms / getVersion now
    worker.postMessage({ type: 'list-algorithms' })
    worker.postMessage({ type: 'get-version' })
    return
  }

  if (data.type === 'algorithms') {
    solverConfig = initSolverConfig(
      data.algorithms,
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
            const record = { solver, total: result.total, runtime, route: result.route }
            showResult(record)
            showDownloadButtons(record, parsedProblem!)
            clearCompareError()

            if (optTourRoute) {
              const compareId = crypto.randomUUID()
              const compareHandler = (e: MessageEvent<CompareToursResult>) => {
                if (e.data.type !== 'compare-tours-result') return
                if (e.data.id !== compareId) return
                worker.removeEventListener('message', compareHandler)
                if (e.data.stats) {
                  patchComparison(record, e.data.stats)
                } else {
                  showCompareError(e.data.error ?? 'Comparison failed')
                }
              }
              worker.addEventListener('message', compareHandler)
              worker.postMessage({
                type: 'compare-tours',
                id: compareId,
                solverRoute: result.route,
                optRoute: optTourRoute,
                cities: parsedProblem!.cities,
              })
            }
          })
          .catch((err: Error) => {
            const overlay = document.getElementById('solving-overlay') as HTMLElement
            overlay.hidden = true
            console.error('Solver error:', err)
          })
      },
    )

    // Upload depends on solverConfig.refresh — must be wired after solverConfig exists
    initUpload(
      parseFile,
      (p) => { parsedProblem = p; solverConfig!.refresh() },
      (route) => {
        optTourRoute = route.length > 0 ? route : null
        updateOptRoute(optTourRoute)
      },
    )

    setWasmStatus('ready')
    gotAlgorithms = true
  }

  if (data.type === 'version') {
    const versionEl = document.getElementById('wasm-version')
    if (versionEl) versionEl.textContent = `teeline-solver ${data.version}`
    gotVersion = true
  }

  if (gotAlgorithms && gotVersion) {
    worker.removeEventListener('message', onInit)
  }
})

// ---- Comparison error banner ----

function showCompareError(msg: string): void {
  const el = document.getElementById('comparison-error') as HTMLElement | null
  if (!el) return
  el.textContent = `Comparison unavailable: ${msg}`
  el.hidden = false
}

function clearCompareError(): void {
  const el = document.getElementById('comparison-error') as HTMLElement | null
  if (el) { el.hidden = true; el.textContent = '' }
}

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

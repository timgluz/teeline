import 'htmx.org'
import '@picocss/pico/css/pico.min.css'
import './main.css'
import { type SolveOptions } from './solver-options'
import type { SolveResult, SolveError } from './worker'

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

// Expose for browser console smoke test and HTMX scripts
declare global {
  interface Window {
    runSolver: typeof runSolver
  }
}
window.runSolver = runSolver

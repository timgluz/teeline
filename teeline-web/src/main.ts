import 'htmx.org'
import '@picocss/pico/css/pico.min.css'
import './main.css'
import type { ParsedProblem } from 'teeline-wasm'
import { type SolveOptions } from './solver-options'
import type { SolveResult, SolveError, ParseResult } from './worker'
import { initUpload } from './upload'
import { initSolverConfig } from './solver-form'

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

let parsedProblem: import('teeline-wasm').ParsedProblem | null = null

initUpload(parseFile, (p) => {
  parsedProblem = p
})

initSolverConfig(
  () => parsedProblem !== null,
  (solver, options) => {
    if (!parsedProblem) return
    const step03 = document.getElementById('step-03') as HTMLElement
    const step04 = document.getElementById('step-04') as HTMLElement
    step03.hidden = true
    step04.hidden = false
    runSolver(solver, parsedProblem.cities, options).catch((err: Error) => {
      console.error('Solver error:', err)
    })
  },
)

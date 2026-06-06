/// <reference lib="webworker" />

import { solve } from 'teeline-wasm'
import { defaultSolveOptions, type SolveOptions } from './solver-options'

export interface SolveRequest {
  type: 'solve'
  solver: string
  cities: Array<{ id: number; x: number; y: number }>
  options: Partial<SolveOptions>
}

export interface SolveResult {
  type: 'result'
  solution: { total: number; route: number[] }
}

export interface SolveError {
  type: 'error'
  message: string
}

self.onmessage = (e: MessageEvent<SolveRequest>) => {
  const { type, solver, cities, options } = e.data
  if (type !== 'solve') return

  const mergedOptions: SolveOptions = { ...defaultSolveOptions(), ...options }

  try {
    const solution = solve(solver, cities, mergedOptions)
    const result: SolveResult = {
      type: 'result',
      solution: {
        total: solution.total,
        route: Array.from(solution.route),  // Uint32Array → plain number[]
      },
    }
    self.postMessage(result)
  } catch (err) {
    const error: SolveError = {
      type: 'error',
      message: err instanceof Error ? err.message : String(err),
    }
    self.postMessage(error)
  }
}

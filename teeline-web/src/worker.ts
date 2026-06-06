/// <reference lib="webworker" />

import { solve, parseAndSolve } from 'teeline-wasm'
import { defaultSolveOptions, type SolveOptions } from './solver-options'

export interface SolveRequest {
  type: 'solve'
  solver: string
  cities: Array<{ id: number; x: number; y: number }>
  options: Partial<SolveOptions>
}

export interface ParseAndSolveRequest {
  type: 'parse-and-solve'
  solver: string
  input: string
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

type WorkerRequest = SolveRequest | ParseAndSolveRequest

export function handleMessage(data: WorkerRequest): SolveResult | SolveError {
  const mergedOptions: SolveOptions = { ...defaultSolveOptions(), ...data.options }
  try {
    let solution: ReturnType<typeof solve>
    if (data.type === 'solve') {
      solution = solve(data.solver, data.cities, mergedOptions)
    } else {
      solution = parseAndSolve(data.solver, data.input, mergedOptions)
    }
    return {
      type: 'result',
      solution: {
        total: solution.total,
        route: Array.from(solution.route),  // Uint32Array → plain number[]
      },
    }
  } catch (err) {
    return {
      type: 'error',
      message: err instanceof Error ? err.message : String(err),
    }
  }
}

// Only register in Web Worker context (not during Vitest runs)
if (typeof DedicatedWorkerGlobalScope !== 'undefined' && self instanceof DedicatedWorkerGlobalScope) {
  self.onmessage = (e: MessageEvent<WorkerRequest>) => {
    self.postMessage(handleMessage(e.data))
  }
}

/// <reference lib="webworker" />

import { solve, parseAndSolve, parse, type ParsedProblem } from 'teeline-wasm'
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

export interface ParseRequest {
  type: 'parse'
  input: string
}

export interface SolveResult {
  type: 'result'
  solution: { total: number; route: number[] }
}

export interface ParseResult {
  type: 'parsed'
  problem: ParsedProblem
}

export interface SolveError {
  type: 'error'
  message: string
}

type WorkerRequest = SolveRequest | ParseAndSolveRequest | ParseRequest
type WorkerResponse = SolveResult | ParseResult | SolveError

export function handleMessage(data: WorkerRequest): WorkerResponse {
  try {
    if (data.type === 'parse') {
      const problem = parse(data.input)
      return { type: 'parsed', problem }
    }
    const mergedOptions: SolveOptions = { ...defaultSolveOptions(), ...data.options }
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

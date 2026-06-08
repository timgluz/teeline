/// <reference lib="webworker" />

import { solve, parseAndSolve, parse, listAlgorithms, getVersion, type ParsedProblem } from 'teeline-wasm'
import type { AlgorithmInfo } from 'teeline-wasm'
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

export interface ListAlgorithmsRequest {
  type: 'list-algorithms'
}

export interface GetVersionRequest {
  type: 'get-version'
}

export interface SolveResult {
  type: 'result'
  solution: { total: number; route: number[] }
}

export interface ParseResult {
  type: 'parsed'
  problem: ParsedProblem
}

export interface AlgorithmsResult {
  type: 'algorithms'
  algorithms: AlgorithmInfo[]
}

export interface VersionResult {
  type: 'version'
  version: string
}

export interface SolveError {
  type: 'error'
  message: string
}

export interface WorkerReadyMessage {
  type: 'worker-ready'
}

type WorkerRequest = SolveRequest | ParseAndSolveRequest | ParseRequest | ListAlgorithmsRequest | GetVersionRequest
type WorkerResponse = SolveResult | ParseResult | AlgorithmsResult | VersionResult | SolveError | WorkerReadyMessage

export function handleMessage(data: WorkerRequest): WorkerResponse {
  try {
    if (data.type === 'parse') {
      const problem = parse(data.input)
      return { type: 'parsed', problem }
    }
    if (data.type === 'list-algorithms') {
      return { type: 'algorithms', algorithms: listAlgorithms() }
    }
    if (data.type === 'get-version') {
      return { type: 'version', version: getVersion() }
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
  // Signal readiness BEFORE registering onmessage so main.ts knows WASM is
  // initialized and won't send messages that deadlock the jco task scheduler.
  self.postMessage({ type: 'worker-ready' } satisfies WorkerReadyMessage)
  self.onmessage = (e: MessageEvent<WorkerRequest>) => {
    self.postMessage(handleMessage(e.data))
  }
}

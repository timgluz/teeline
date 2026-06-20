/// <reference lib="webworker" />

import { solve, parseAndSolve, parse, listAlgorithms, getVersion, compareTours, type ParsedProblem } from 'teeline-wasm'
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

export interface ComparisonStats {
  optimalCost: number
  solverCost: number
  gapPct: number
  sharedEdges: number
  solverOnlyEdges: number
  optimalOnlyEdges: number
}

export interface CompareToursRequest {
  type: 'compare-tours'
  id: string
  solverRoute: number[]
  optRoute: number[]
  cities: Array<{ id: number; x: number; y: number }>
}

export interface CompareToursResult {
  type: 'compare-tours-result'
  id: string
  stats?: ComparisonStats
  error?: string
}

export interface WebMCPSolveRequest {
  type: 'webmcp-solve'
  id: string
  solver: string
  input: string
  options: Partial<SolveOptions>
}
export interface WebMCPSolveResult {
  type: 'webmcp-result'
  id: string
  solution?: { total: number; route: number[] }
  error?: string
}

export interface WebMCPListAlgorithmsRequest {
  type: 'webmcp-list-algorithms'
  id: string
}
export interface WebMCPAlgorithmsResult {
  type: 'webmcp-algorithms'
  id: string
  algorithms?: AlgorithmInfo[]
  error?: string
}

export interface WebMCPParseRequest {
  type: 'webmcp-parse'
  id: string
  input: string
}
export interface WebMCPParseResult {
  type: 'webmcp-parsed'
  id: string
  problem?: ParsedProblem
  error?: string
}

type WorkerRequest = SolveRequest | ParseAndSolveRequest | ParseRequest | ListAlgorithmsRequest | GetVersionRequest | CompareToursRequest | WebMCPSolveRequest | WebMCPListAlgorithmsRequest | WebMCPParseRequest
type WorkerResponse = SolveResult | ParseResult | AlgorithmsResult | VersionResult | SolveError | WorkerReadyMessage | CompareToursResult | WebMCPSolveResult | WebMCPAlgorithmsResult | WebMCPParseResult

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
    if (data.type === 'compare-tours') {
      const req = data as CompareToursRequest
      try {
        // compareTours returns ComparisonStats directly (jco throws on WIT Err)
        const s = compareTours(new Uint32Array(req.solverRoute), new Uint32Array(req.optRoute), req.cities)
        return {
          type: 'compare-tours-result',
          id: req.id,
          stats: {
            optimalCost: s.optimalCost,
            solverCost: s.solverCost,
            gapPct: s.gapPct,
            sharedEdges: s.sharedEdges,
            solverOnlyEdges: s.solverOnlyEdges,
            optimalOnlyEdges: s.optimalOnlyEdges,
          },
        }
      } catch (err) {
        return {
          type: 'compare-tours-result',
          id: req.id,
          error: err instanceof Error ? err.message : String(err),
        }
      }
    }
    if (data.type === 'webmcp-solve') {
      const req = data as WebMCPSolveRequest
      try {
        const opts = { ...defaultSolveOptions(), ...req.options }
        const raw = parseAndSolve(req.solver, req.input, opts)
        return { type: 'webmcp-result', id: req.id, solution: { total: raw.total, route: Array.from(raw.route) } }
      } catch (e) {
        return { type: 'webmcp-result', id: req.id, error: String(e) }
      }
    }
    if (data.type === 'webmcp-list-algorithms') {
      const req = data as WebMCPListAlgorithmsRequest
      try {
        return { type: 'webmcp-algorithms', id: req.id, algorithms: listAlgorithms() }
      } catch (e) {
        return { type: 'webmcp-algorithms', id: req.id, error: String(e) }
      }
    }
    if (data.type === 'webmcp-parse') {
      const req = data as WebMCPParseRequest
      try {
        return { type: 'webmcp-parsed', id: req.id, problem: parse(req.input) }
      } catch (e) {
        return { type: 'webmcp-parsed', id: req.id, error: String(e) }
      }
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

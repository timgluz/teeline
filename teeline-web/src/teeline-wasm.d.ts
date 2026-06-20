declare module 'teeline-wasm' {
  export interface City {
    id: number
    x: number
    y: number
  }
  export interface SolveOptions {
    epochs: number
    platooEpochs: number
    coolingRate: number
    maxTemperature: number
    minTemperature: number
    mutationProbability: number
    nElite: number
    nNearest: number
  }
  export interface Solution {
    total: number
    route: Uint32Array
  }
  export interface ParsedProblem {
    name: string
    comment: string
    distanceType: string
    cities: Array<City>
  }
  export interface ParamSpec {
    key: string
    label: string
    valueType: string
    min?: number
    max?: number
    step?: number
    description: string
  }
  export interface AlgorithmInfo {
    id: string
    name: string
    description: string
    recommendation: string
    kind: string
    params: Array<ParamSpec>
  }
  export function solve(solver: string, cities: Array<City>, options: SolveOptions): Solution
  export function parseAndSolve(solver: string, input: string, options: SolveOptions): Solution
  export function parse(input: string): ParsedProblem
  export function listAlgorithms(): Array<AlgorithmInfo>
  export function getVersion(): string
  export interface ComparisonStats {
    optimalCost: number
    solverCost: number
    gapPct: number
    sharedEdges: number
    solverOnlyEdges: number
    optimalOnlyEdges: number
  }
  export function compareTours(solverRoute: Uint32Array, optRoute: Uint32Array, cities: Array<City>): ComparisonStats
  export type Result<T, E> = { tag: 'ok'; val: T } | { tag: 'err'; val: E }
}

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
  export function solve(solver: string, cities: Array<City>, options: SolveOptions): Solution
  export function parseAndSolve(solver: string, input: string, options: SolveOptions): Solution
  export function parse(input: string): ParsedProblem
  export type Result<T, E> = { tag: 'ok'; val: T } | { tag: 'err'; val: E }
}

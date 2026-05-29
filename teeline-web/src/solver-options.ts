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

export function defaultSolveOptions(): SolveOptions {
  return {
    epochs: 500,
    platooEpochs: 50,
    coolingRate: 0.0001,
    maxTemperature: 1000.0,
    minTemperature: 0.001,
    mutationProbability: 0.05,
    nElite: 3,
    nNearest: 5,
  }
}

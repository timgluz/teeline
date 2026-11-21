export interface SolveOptions {
  epochs: number
  platooEpochs: number
  coolingRate: number
  maxTemperature: number
  minTemperature: number
  mutationProbability: number
  nElite: number
  nNearest: number
  // ACO tunables (ignored by non-ACO solvers, same as nElite/coolingRate).
  alpha: number
  beta: number
  evaporationRate: number
  numAnts: number
  // SFC tunables (ignored by non-SFC solvers).
  sfcOrder: number
  sfcRotation: number
  sfcMirror: number
  sfcMultiOrientation: number
  // Memetic EAX tunables (ignored by non-EAX solvers). These are required fields in
  // the WIT `solve-options` record; outside JS callers that omit them get 0, which the
  // WASM bridge clamps to the smallest valid configuration.
  eaxPopSize: number
  eaxChildrenPerPair: number
  eaxStrategy: number
  eaxStallEpochs: number
  eaxCandidateK: number
}

export function defaultSolveOptions(solverId?: string): SolveOptions {
  return {
    // EAX's solver-specific default is lower than the shared 500: at 500, dsj1000
    // regularly exceeded the 300-second meta benchmark budget.
    epochs: solverId === 'eax' ? 250 : 500,
    platooEpochs: 50,
    coolingRate: 0.0001,
    maxTemperature: 1000.0,
    minTemperature: 0.001,
    mutationProbability: 0.05,
    nElite: 3,
    nNearest: 5,
    alpha: 1.0,
    beta: 2.0,
    evaporationRate: 0.5,
    numAnts: 25,
    sfcOrder: 16,
    sfcRotation: 0,
    sfcMirror: 0,
    sfcMultiOrientation: 8,
    eaxPopSize: 100,
    eaxChildrenPerPair: 15,
    eaxStrategy: 2,
    eaxStallEpochs: 50,
    eaxCandidateK: 20,
  }
}

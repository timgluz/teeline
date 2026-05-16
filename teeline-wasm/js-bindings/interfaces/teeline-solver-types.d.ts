/** @module Interface teeline:solver/types@0.1.0 **/
export interface City {
  id: number,
  x: number,
  y: number,
}
export interface SolveOptions {
  epochs: number,
  platooEpochs: number,
  coolingRate: number,
  maxTemperature: number,
  minTemperature: number,
  mutationProbability: number,
  nElite: number,
  nNearest: number,
}
export interface Solution {
  total: number,
  route: Uint32Array,
}

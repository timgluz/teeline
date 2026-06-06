import { describe, it, expect } from 'vitest'
import { formatMetadata, exampleUrl } from './upload'
import type { ParsedProblem } from 'teeline-wasm'

const cities = (n: number) =>
  Array.from({ length: n }, (_, i) => ({ id: i, x: 0, y: 0 }))

const makeProblem = (overrides: Partial<ParsedProblem> = {}): ParsedProblem => ({
  name: 'berlin52',
  comment: '',
  distanceType: 'EUC_2D',
  cities: cities(52),
  ...overrides,
})

describe('formatMetadata', () => {
  it('includes city count and distance type', () => {
    expect(formatMetadata(makeProblem())).toBe('parsed: 52 cities · EUC_2D')
  })

  it('omits distance type separator when distanceType is empty', () => {
    expect(formatMetadata(makeProblem({ distanceType: '', cities: cities(5) }))).toBe(
      'parsed: 5 cities',
    )
  })

  it('uses the cities array length, not the name', () => {
    expect(formatMetadata(makeProblem({ cities: cities(14) }))).toBe('parsed: 14 cities · EUC_2D')
  })

  it('handles GEO distance type', () => {
    expect(formatMetadata(makeProblem({ distanceType: 'GEO', cities: cities(22) }))).toBe(
      'parsed: 22 cities · GEO',
    )
  })
})

describe('exampleUrl', () => {
  it('returns /examples/<name>.tsp', () => {
    expect(exampleUrl('berlin52')).toBe('/examples/berlin52.tsp')
    expect(exampleUrl('burma14')).toBe('/examples/burma14.tsp')
    expect(exampleUrl('ulysses22')).toBe('/examples/ulysses22.tsp')
  })
})

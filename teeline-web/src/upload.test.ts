import { describe, it, expect } from 'vitest'
import { formatMetadata, exampleUrl, parseOptTour } from './upload'
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

describe('parseOptTour', () => {
  const minimal = `NAME : test\nTOUR_SECTION\n1\n3\n2\n-1\nEOF\n`

  it('extracts city IDs from TOUR_SECTION until -1', () => {
    expect(parseOptTour(minimal)).toEqual([1, 3, 2])
  })

  it('handles leading and trailing whitespace on each line', () => {
    const text = `TOUR_SECTION\n  1  \n  49  \n  32  \n-1\n`
    expect(parseOptTour(text)).toEqual([1, 49, 32])
  })

  it('skips all header lines before TOUR_SECTION', () => {
    const text = `NAME : berlin52\nTYPE : TOUR\nDIMENSION : 3\nTOUR_SECTION\n7\n5\n9\n-1\n`
    expect(parseOptTour(text)).toEqual([7, 5, 9])
  })

  it('returns empty array when TOUR_SECTION is absent', () => {
    expect(parseOptTour('NAME : foo\n1\n2\n3\n')).toEqual([])
  })

  it('stops before EOF line', () => {
    const text = `TOUR_SECTION\n1\n2\n-1\nEOF\n`
    expect(parseOptTour(text)).toEqual([1, 2])
  })
})

describe('exampleUrl', () => {
  it('returns /examples/<name>.tsp', () => {
    expect(exampleUrl('berlin52')).toBe('/examples/berlin52.tsp')
    expect(exampleUrl('burma14')).toBe('/examples/burma14.tsp')
    expect(exampleUrl('ulysses22')).toBe('/examples/ulysses22.tsp')
  })
})

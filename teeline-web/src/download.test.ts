import { describe, it, expect } from 'vitest'
import { buildTourText, buildCsvText, buildJsonText } from './download'
import type { RunRecord } from './results'

const cities = [
  { id: 1, x: 0, y: 0 },
  { id: 2, x: 3, y: 4 },
  { id: 3, x: 6, y: 0 },
]

describe('buildTourText', () => {
  it('starts with NAME header', () => {
    const text = buildTourText('berlin52', [1, 2, 3])
    expect(text.startsWith('NAME: berlin52\n')).toBe(true)
  })

  it('includes TOUR_SECTION marker', () => {
    const text = buildTourText('test', [1, 2, 3])
    expect(text).toContain('TOUR_SECTION\n')
  })

  it('lists city IDs in tour order', () => {
    const text = buildTourText('test', [3, 1, 2])
    const lines = text.split('\n')
    const sectionIdx = lines.indexOf('TOUR_SECTION')
    expect(lines[sectionIdx + 1]).toBe('3')
    expect(lines[sectionIdx + 2]).toBe('1')
    expect(lines[sectionIdx + 3]).toBe('2')
  })

  it('terminates with -1 and EOF', () => {
    const text = buildTourText('test', [1, 2])
    expect(text).toContain('\n-1\nEOF')
  })

  it('handles single-city route', () => {
    const text = buildTourText('tiny', [42])
    expect(text).toContain('TOUR_SECTION\n42\n-1\nEOF')
  })
})

describe('buildCsvText', () => {
  it('starts with id,x,y header', () => {
    const text = buildCsvText([1, 2], cities)
    expect(text.startsWith('id,x,y\n')).toBe(true)
  })

  it('outputs cities in tour order', () => {
    const text = buildCsvText([3, 1, 2], cities)
    const lines = text.split('\n').filter(Boolean)
    expect(lines[1]).toBe('3,6,0')
    expect(lines[2]).toBe('1,0,0')
    expect(lines[3]).toBe('2,3,4')
  })

  it('empty route returns header only', () => {
    const text = buildCsvText([], cities)
    expect(text.trim()).toBe('id,x,y')
  })

  it('includes all cities when route covers all', () => {
    const text = buildCsvText([1, 2, 3], cities)
    const lines = text.split('\n').filter(Boolean)
    expect(lines).toHaveLength(4) // header + 3 cities
  })
})

describe('buildJsonText', () => {
  const record: RunRecord = { solver: 'nn', total: 42.5, runtime: 123, route: [1, 2, 3] }

  it('parses back to a valid object', () => {
    const text = buildJsonText('berlin52', record, cities, 9999)
    const parsed = JSON.parse(text)
    expect(parsed).toBeTruthy()
  })

  it('includes solver name', () => {
    const parsed = JSON.parse(buildJsonText('berlin52', record, cities, 9999))
    expect(parsed.solver).toBe('nn')
  })

  it('includes tour total and runtime', () => {
    const parsed = JSON.parse(buildJsonText('berlin52', record, cities, 9999))
    expect(parsed.total).toBe(42.5)
    expect(parsed.runtime).toBe(123)
  })

  it('includes cities array', () => {
    const parsed = JSON.parse(buildJsonText('berlin52', record, cities, 9999))
    expect(parsed.cities).toHaveLength(3)
  })

  it('includes route array', () => {
    const parsed = JSON.parse(buildJsonText('berlin52', record, cities, 9999))
    expect(parsed.route).toEqual([1, 2, 3])
  })

  it('preserves timestamp', () => {
    const parsed = JSON.parse(buildJsonText('berlin52', record, cities, 12345))
    expect(parsed.timestamp).toBe(12345)
  })

  it('includes problem name', () => {
    const parsed = JSON.parse(buildJsonText('myProblem', record, cities, 0))
    expect(parsed.name).toBe('myProblem')
  })
})

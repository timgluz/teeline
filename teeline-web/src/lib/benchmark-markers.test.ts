import { describe, expect, it } from 'vitest'

import { parseBenchmarkIndex } from './benchmark'
import {
  MARKER_TITLE,
  markerAttribute,
  markerCount,
  publishedIds,
} from './benchmark-markers'

const RAW = {
  schema_version: 2,
  generated_at: '2026-10-11T12:05:56Z',
  tier: 'a',
  budgets: { local: { seconds: 300, agreed: false } },
  solvers: [
    { id: 'lk', family: 'local' },
    { id: '2opt', family: 'local' },
  ],
  problems: [
    { id: 'berlin52', cities: 52 },
    { id: 'att48', cities: 48 },
  ],
}

const INDEX = parseBenchmarkIndex(RAW)

describe('index benchmark markers', () => {
  it('matches each row kind to its own manifest field', () => {
    expect(markerAttribute('dataset')).toBe('data-dataset')
    expect(markerAttribute('solver')).toBe('data-solver')
  })

  it('reads the published datasets and solvers out of the manifest', () => {
    expect([...publishedIds(INDEX, 'dataset')!].sort()).toEqual([
      'att48',
      'berlin52',
    ])
    expect([...publishedIds(INDEX, 'solver')!].sort()).toEqual(['2opt', 'lk'])
  })

  it('tells an unreadable manifest apart from one that publishes nothing', () => {
    // Null keeps every marker hidden; an empty set is a manifest that names nothing, which the
    // page may safely state as a count.
    expect(publishedIds(null, 'dataset')).toBeNull()
    const empty = parseBenchmarkIndex({ ...RAW, problems: [], solvers: [] })
    expect(publishedIds(empty, 'dataset')?.size).toBe(0)
    expect(publishedIds(empty, 'solver')?.size).toBe(0)
  })

  it('states a count only when some rows lack results', () => {
    expect(markerCount(12, 111, 'dataset')).toBe(
      '12 of 111 instances have published benchmark results',
    )
    // Every documented solver has a shard today, so this line would say nothing.
    expect(markerCount(21, 21, 'solver')).toBeNull()
    expect(markerCount(0, 21, 'solver')).toBe(
      '0 of 21 solvers have published benchmark results',
    )
    expect(markerCount(0, 0, 'dataset')).toBeNull()
    expect(markerCount(0, 1, 'dataset')).toBe(
      '0 of 1 instance has published benchmark results',
    )
  })

  it('uses the tooltip the marker shows on hover', () => {
    expect(MARKER_TITLE).toBe('It has benchmark results')
  })
})

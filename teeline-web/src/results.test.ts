import { describe, it, expect } from 'vitest'
import { formatRuntime } from './results'
import type { RunRecord } from './results'
import type { ComparisonStats } from './worker'

describe('formatRuntime', () => {
  it('formats sub-second as "<N>ms"', () => {
    expect(formatRuntime(123)).toBe('123ms')
    expect(formatRuntime(0)).toBe('0ms')
    expect(formatRuntime(999)).toBe('999ms')
  })

  it('formats 1000ms or more as "<N.N>s"', () => {
    expect(formatRuntime(1000)).toBe('1.0s')
    expect(formatRuntime(1500)).toBe('1.5s')
    expect(formatRuntime(12300)).toBe('12.3s')
  })
})

describe('RunRecord type', () => {
  it('accepts comparison field with ComparisonStats', () => {
    const stats: ComparisonStats = {
      optimalCost: 7542,
      solverCost: 8296,
      gapPct: 10.0,
      sharedEdges: 30,
      solverOnlyEdges: 22,
      optimalOnlyEdges: 22,
    }
    const record: RunRecord = {
      solver: 'nn',
      total: 8296,
      comparison: stats,
      runtime: 12,
      route: [1, 2, 3],
    }
    expect(record.comparison?.gapPct).toBe(10.0)
    expect(record.comparison?.sharedEdges).toBe(30)
    expect(record.comparison?.solverOnlyEdges).toBe(22)
    expect(record.comparison?.optimalOnlyEdges).toBe(22)
  })

  it('allows comparison to be absent', () => {
    const record: RunRecord = { solver: 'nn', total: 8296, runtime: 12, route: [1, 2, 3] }
    expect(record.comparison).toBeUndefined()
  })
})

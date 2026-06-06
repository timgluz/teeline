import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('teeline-wasm', () => ({
  solve: vi.fn(),
  parseAndSolve: vi.fn(),
}))

import { solve, parseAndSolve } from 'teeline-wasm'
import { handleMessage, type ParseAndSolveRequest } from './worker'
import { type SolveRequest } from './worker'

const mockSolution = { total: 42.0, route: new Uint32Array([0, 1, 2]) }

beforeEach(() => {
  vi.clearAllMocks()
})

describe('handleMessage — solve', () => {
  it('calls solve and returns SolveResult', () => {
    vi.mocked(solve).mockReturnValue(mockSolution)
    const req: SolveRequest = {
      type: 'solve',
      solver: 'nn',
      cities: [{ id: 0, x: 1, y: 2 }, { id: 1, x: 3, y: 4 }, { id: 2, x: 5, y: 6 }],
      options: {},
    }
    const res = handleMessage(req)
    expect(res.type).toBe('result')
    if (res.type === 'result') {
      expect(res.solution.total).toBe(42.0)
      expect(res.solution.route).toEqual([0, 1, 2])
    }
  })

  it('returns SolveError when solve throws', () => {
    vi.mocked(solve).mockImplementation(() => { throw new Error('solver crashed') })
    const req: SolveRequest = { type: 'solve', solver: 'nn', cities: [], options: {} }
    const res = handleMessage(req)
    expect(res.type).toBe('error')
    if (res.type === 'error') {
      expect(res.message).toContain('solver crashed')
    }
  })
})

describe('handleMessage — parse-and-solve', () => {
  it('calls parseAndSolve with the raw input string and returns SolveResult', () => {
    vi.mocked(parseAndSolve).mockReturnValue(mockSolution)
    const req: ParseAndSolveRequest = {
      type: 'parse-and-solve',
      solver: 'nn',
      input: '[{"id":0,"x":1.0,"y":2.0},{"id":1,"x":3.0,"y":4.0}]',
      options: {},
    }
    const res = handleMessage(req)
    expect(parseAndSolve).toHaveBeenCalledOnce()
    expect(res.type).toBe('result')
    if (res.type === 'result') {
      expect(res.solution.total).toBe(42.0)
      expect(res.solution.route).toEqual([0, 1, 2])
    }
  })

  it('returns SolveError when parseAndSolve throws', () => {
    vi.mocked(parseAndSolve).mockImplementation(() => { throw new Error('bad TSPLIB input') })
    const req: ParseAndSolveRequest = {
      type: 'parse-and-solve',
      solver: 'nn',
      input: 'garbage',
      options: {},
    }
    const res = handleMessage(req)
    expect(res.type).toBe('error')
    if (res.type === 'error') {
      expect(res.message).toContain('bad TSPLIB input')
    }
  })
})

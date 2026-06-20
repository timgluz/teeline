import { describe, it, expect, vi, beforeEach } from 'vitest'

vi.mock('teeline-wasm', () => ({
  solve: vi.fn(),
  parseAndSolve: vi.fn(),
  parse: vi.fn(),
  listAlgorithms: vi.fn(),
  getVersion: vi.fn(),
  compareTours: vi.fn(),
}))

import { solve, parseAndSolve, parse, listAlgorithms, getVersion, compareTours } from 'teeline-wasm'
import {
  handleMessage,
  type ParseAndSolveRequest,
  type ParseRequest,
  type SolveRequest,
  type ListAlgorithmsRequest,
  type GetVersionRequest,
  type CompareToursRequest,
  type ComparisonStats,
} from './worker'

const mockSolution = { total: 42.0, route: new Uint32Array([0, 1, 2]) }

const mockAlgorithm = {
  id: 'nn',
  name: 'Nearest Neighbor',
  description: 'Greedy (O(n log n))',
  recommendation: 'Fastest',
  kind: 'constructive',
  params: [],
}

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

describe('handleMessage — parse', () => {
  const mockProblem = {
    name: 'berlin52',
    comment: 'test',
    distanceType: 'EUC_2D',
    cities: [{ id: 0, x: 1.0, y: 2.0 }],
  }

  it('calls parse and returns ParseResult', () => {
    vi.mocked(parse).mockReturnValue(mockProblem)
    const req: ParseRequest = { type: 'parse', input: 'NAME: berlin52\n' }
    const res = handleMessage(req)
    expect(parse).toHaveBeenCalledWith('NAME: berlin52\n')
    expect(res.type).toBe('parsed')
    if (res.type === 'parsed') {
      expect(res.problem.name).toBe('berlin52')
      expect(res.problem.distanceType).toBe('EUC_2D')
      expect(res.problem.cities).toHaveLength(1)
    }
  })

  it('returns SolveError when parse throws', () => {
    vi.mocked(parse).mockImplementation(() => { throw new Error('bad input') })
    const req: ParseRequest = { type: 'parse', input: '' }
    const res = handleMessage(req)
    expect(res.type).toBe('error')
    if (res.type === 'error') {
      expect(res.message).toContain('bad input')
    }
  })
})

describe('handleMessage — list-algorithms', () => {
  it('calls listAlgorithms and returns AlgorithmsResult', () => {
    vi.mocked(listAlgorithms).mockReturnValue([mockAlgorithm])
    const req: ListAlgorithmsRequest = { type: 'list-algorithms' }
    const res = handleMessage(req)
    expect(listAlgorithms).toHaveBeenCalledOnce()
    expect(res.type).toBe('algorithms')
    if (res.type === 'algorithms') {
      expect(res.algorithms).toHaveLength(1)
      expect(res.algorithms[0].id).toBe('nn')
      expect(res.algorithms[0].kind).toBe('constructive')
    }
  })

  it('returns SolveError when listAlgorithms throws', () => {
    vi.mocked(listAlgorithms).mockImplementation(() => { throw new Error('wasm init failed') })
    const req: ListAlgorithmsRequest = { type: 'list-algorithms' }
    const res = handleMessage(req)
    expect(res.type).toBe('error')
    if (res.type === 'error') {
      expect(res.message).toContain('wasm init failed')
    }
  })
})

describe('handleMessage — get-version', () => {
  it('calls getVersion and returns VersionResult', () => {
    vi.mocked(getVersion).mockReturnValue('0.1.0')
    const req: GetVersionRequest = { type: 'get-version' }
    const res = handleMessage(req)
    expect(getVersion).toHaveBeenCalledOnce()
    expect(res.type).toBe('version')
    if (res.type === 'version') {
      expect(res.version).toBe('0.1.0')
    }
  })

  it('returns SolveError when getVersion throws', () => {
    vi.mocked(getVersion).mockImplementation(() => { throw new Error('version unavailable') })
    const req: GetVersionRequest = { type: 'get-version' }
    const res = handleMessage(req)
    expect(res.type).toBe('error')
    if (res.type === 'error') {
      expect(res.message).toContain('version unavailable')
    }
  })
})

describe('handleMessage — compare-tours', () => {
  const mockStats: ComparisonStats = {
    optimalCost: 100.0,
    solverCost: 110.0,
    gapPct: 10.0,
    sharedEdges: 3,
    solverOnlyEdges: 1,
    optimalOnlyEdges: 1,
  }
  const cities = [
    { id: 1, x: 0.0, y: 0.0 },
    { id: 2, x: 1.0, y: 0.0 },
    { id: 3, x: 1.0, y: 1.0 },
    { id: 4, x: 0.0, y: 1.0 },
  ]
  const solverRoute = [1, 2, 3, 4]
  const optRoute    = [1, 2, 4, 3]

  it('returns compare-tours-result with stats on success', () => {
    vi.mocked(compareTours).mockReturnValue(mockStats)
    const req: CompareToursRequest = {
      type: 'compare-tours',
      id: 'test-id-1',
      solverRoute,
      optRoute,
      cities,
    }
    const res = handleMessage(req)
    expect(res.type).toBe('compare-tours-result')
    if (res.type === 'compare-tours-result') {
      expect(res.id).toBe('test-id-1')
      expect(res.stats?.gapPct).toBe(10.0)
      expect(res.stats?.sharedEdges).toBe(3)
      expect(res.error).toBeUndefined()
    }
  })

  it('echoes the request id in the response', () => {
    vi.mocked(compareTours).mockReturnValue(mockStats)
    const req: CompareToursRequest = {
      type: 'compare-tours',
      id: 'correlation-abc',
      solverRoute,
      optRoute,
      cities,
    }
    const res = handleMessage(req)
    if (res.type === 'compare-tours-result') {
      expect(res.id).toBe('correlation-abc')
    }
  })

  it('returns error in compare-tours-result when compareTours throws', () => {
    vi.mocked(compareTours).mockImplementation(() => {
      throw new Error('dimension mismatch: solver=3 opt=4 cities=4')
    })
    const req: CompareToursRequest = {
      type: 'compare-tours',
      id: 'test-id-err',
      solverRoute: [1, 2, 3],
      optRoute: [1, 2, 3, 4],
      cities,
    }
    const res = handleMessage(req)
    expect(res.type).toBe('compare-tours-result')
    if (res.type === 'compare-tours-result') {
      expect(res.id).toBe('test-id-err')
      expect(res.error).toContain('dimension mismatch')
      expect(res.stats).toBeUndefined()
    }
  })
})

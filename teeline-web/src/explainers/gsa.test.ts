import { describe, it, expect } from 'vitest'
import {
  N_CITIES, tourLength, swapSequence, applySwaps,
  makeInitState, stepAgent,
} from './gsa-algo'

describe('tourLength', () => {
  it('is positive for a multi-city tour', () => {
    expect(tourLength(Array.from({ length: N_CITIES }, (_, i) => i))).toBeGreaterThan(0)
  })
  it('returns 0 for a single-city tour', () => {
    expect(tourLength([0])).toBe(0)
  })
})

describe('swapSequence', () => {
  it('returns empty for identical tours', () => {
    expect(swapSequence([0, 1, 2, 3], [0, 1, 2, 3])).toEqual([])
  })
  it('applying result of swapSequence(a, b) to a yields b', () => {
    const a = [0, 3, 1, 2]; const b = [0, 1, 2, 3]
    expect(applySwaps(a, swapSequence(a, b), swapSequence(a, b).length)).toEqual(b)
  })
})

describe('applySwaps', () => {
  it('does not mutate input', () => {
    const t = [0, 1, 2, 3]; applySwaps(t, [[0, 1]], 1)
    expect(t).toEqual([0, 1, 2, 3])
  })
  it('clamps to swaps.length', () => {
    expect(applySwaps([0, 1, 2], [[0, 1]], 99)).toEqual([1, 0, 2])
  })
})

describe('makeInitState', () => {
  it('creates the requested number of agents', () => {
    expect(makeInitState(5).agents).toHaveLength(5)
  })
  it('each agent position is a valid permutation', () => {
    const expected = Array.from({ length: N_CITIES }, (_, i) => i).sort((a, b) => a - b)
    for (const ag of makeInitState(4).agents)
      expect(ag.position.slice().sort((a, b) => a - b)).toEqual(expected)
  })
  it('epoch is 0, agentIdx is 0', () => {
    const s = makeInitState(4)
    expect(s.epoch).toBe(0)
    expect(s.agentIdx).toBe(0)
  })
  it('gbest_cost equals minimum agent cost', () => {
    const s = makeInitState(6)
    expect(s.gbest_cost).toBeCloseTo(Math.min(...s.agents.map(a => a.cost)), 1)
  })
  it('masses sum to ~1', () => {
    const s = makeInitState(4)
    expect(s.masses.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 1)
  })
})

describe('stepAgent', () => {
  it('agentIdx advances by 1 each call', () => {
    expect(stepAgent(makeInitState(4)).agentIdx).toBe(1)
  })
  it('epoch increments after all agents processed', () => {
    let s = makeInitState(4)
    for (let i = 0; i < 4; i++) s = stepAgent(s)
    expect(s.epoch).toBe(1)
    expect(s.agentIdx).toBe(0)
  })
  it('gbest_cost never increases', () => {
    let s = makeInitState(6)
    for (let i = 0; i < 30; i++) {
      const prev = s.gbest_cost
      s = stepAgent(s)
      expect(s.gbest_cost).toBeLessThanOrEqual(prev + 0.001)
    }
  })
  it('each agent position remains a valid permutation', () => {
    const expected = Array.from({ length: N_CITIES }, (_, i) => i).sort((a, b) => a - b)
    let s = makeInitState(4)
    s = stepAgent(s)
    for (const ag of s.agents)
      expect(ag.position.slice().sort((a, b) => a - b)).toEqual(expected)
  })
  it('populates lastBreakdown after first step', () => {
    const s = stepAgent(makeInitState(4))
    expect(s.lastBreakdown).not.toBeNull()
    expect(s.lastBreakdown!.totalApplied).toBeGreaterThanOrEqual(0)
    expect(s.lastBreakdown!.G).toBeGreaterThan(0)
  })
  it('G decreases over epochs', () => {
    let s = makeInitState(4)
    for (let i = 0; i < 4; i++) s = stepAgent(s)
    const g1 = s.G
    for (let i = 0; i < 4; i++) s = stepAgent(s)
    expect(s.G).toBeLessThanOrEqual(g1)
  })
})

import { describe, test, expect } from 'vitest'
import {
  N_CITIES, N_NEURONS, SIGMA0, MAX_STEPS,
  makeInitState, stepOnce, tourLength, neighbourRadiusPx, phaseLabel,
} from './som-algo'

describe('tourLength', () => {
  test('returns 0 for empty tour', () => {
    expect(tourLength([])).toBe(0)
  })
  test('returns 0 for single-city tour', () => {
    expect(tourLength([0])).toBe(0)
  })
  test('positive for two cities', () => {
    expect(tourLength([0, 1])).toBeGreaterThan(0)
  })
  test('symmetric: reversing tour gives same length', () => {
    const tour = Array.from({ length: N_CITIES }, (_, i) => i)
    expect(tourLength(tour)).toBeCloseTo(tourLength([...tour].reverse()), 5)
  })
})

describe('makeInitState', () => {
  test('produces N_NEURONS neurons', () => {
    expect(makeInitState().neurons).toHaveLength(N_NEURONS)
  })
  test('neurons form a ring of radius ~15 around centroid (150,150)', () => {
    for (const [nx, ny] of makeInitState().neurons) {
      expect(Math.hypot(nx - 150, ny - 150)).toBeCloseTo(15, 0)
    }
  })
  test('phase is init', () => {
    expect(makeInitState().phase).toBe('init')
  })
  test('step is 0', () => {
    expect(makeInitState().step).toBe(0)
  })
  test('sigma equals SIGMA0', () => {
    expect(makeInitState().sigma).toBe(SIGMA0)
  })
})

describe('stepOnce', () => {
  test('advances step by 1', () => {
    expect(stepOnce(makeInitState()).step).toBe(1)
  })
  test('returns a new object (immutable update)', () => {
    const s = makeInitState()
    const next = stepOnce(s)
    expect(next).not.toBe(s)
    expect(next.neurons).not.toBe(s.neurons)
  })
  test('neuron count is unchanged', () => {
    expect(stepOnce(makeInitState()).neurons).toHaveLength(N_NEURONS)
  })
  test('alpha decays monotonically over first 10 steps', () => {
    let s = makeInitState()
    let prev = s.alpha
    for (let i = 0; i < 10; i++) {
      s = stepOnce(s)
      expect(s.alpha).toBeLessThanOrEqual(prev)
      prev = s.alpha
    }
  })
  test('sigma never falls below 1.0 (floor)', () => {
    let s = makeInitState()
    for (let i = 0; i <= MAX_STEPS; i++) s = stepOnce(s)
    // sigma is clamped on each step; check the training steps
    // (after MAX_STEPS, phase is done and sigma is from last training step)
    expect(s.sigma).toBeGreaterThanOrEqual(1.0)
  })
  test('phase becomes done after MAX_STEPS+1 calls', () => {
    let s = makeInitState()
    for (let i = 0; i <= MAX_STEPS; i++) s = stepOnce(s)
    expect(s.phase).toBe('done')
  })
  test('tour contains each city index exactly once when done', () => {
    let s = makeInitState()
    for (let i = 0; i <= MAX_STEPS; i++) s = stepOnce(s)
    expect(s.tour).not.toBeNull()
    const sorted = [...s.tour!].sort((a, b) => a - b)
    expect(sorted).toEqual(Array.from({ length: N_CITIES }, (_, i) => i))
  })
  test('done state is idempotent', () => {
    let s = makeInitState()
    for (let i = 0; i <= MAX_STEPS; i++) s = stepOnce(s)
    expect(stepOnce(s)).toBe(s)
  })
})

describe('neighbourRadiusPx', () => {
  test('returns a positive number', () => {
    expect(neighbourRadiusPx(4, makeInitState().neurons)).toBeGreaterThan(0)
  })
  test('larger sigma gives larger radius', () => {
    const { neurons } = makeInitState()
    expect(neighbourRadiusPx(4, neurons)).toBeGreaterThan(neighbourRadiusPx(2, neurons))
  })
})

describe('phaseLabel', () => {
  test('init phase returns non-empty string', () => {
    const label = phaseLabel('init', null)
    expect(label).toBeTruthy()
    expect(typeof label).toBe('string')
  })
  test('expanding phase returns non-empty string', () => {
    const label = phaseLabel('expanding', null)
    expect(label).toBeTruthy()
    expect(typeof label).toBe('string')
  })
  test('converging phase returns non-empty string', () => {
    const label = phaseLabel('converging', null)
    expect(label).toBeTruthy()
    expect(typeof label).toBe('string')
  })
  test('fine-tuning phase returns non-empty string', () => {
    const label = phaseLabel('fine-tuning', null)
    expect(label).toBeTruthy()
    expect(typeof label).toBe('string')
  })
  test('done phase interpolates lastTourLength when provided', () => {
    const label = phaseLabel('done', 123.456)
    expect(label).toContain('123')
  })
  test('done phase shows — when lastTourLength is null', () => {
    const label = phaseLabel('done', null)
    expect(label).toContain('—')
  })
})

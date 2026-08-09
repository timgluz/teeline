import { describe, it, expect } from 'vitest'
import {
  N_CITIES, tourLength,
  makeInitState, stepOnce, maxPheromone,
} from './aco-algo'

describe('makeInitState', () => {
  it('starts in building phase with antIdx=0', () => {
    const s = makeInitState(1, 2, 0.5, 10)
    expect(s.phase).toBe('building')
    expect(s.antIdx).toBe(0)
    expect(s.epoch).toBe(0)
  })

  it('pheromone matrix is symmetric and tau0 on all off-diagonal edges', () => {
    const s = makeInitState(1, 2, 0.5, 10)
    const tau0 = s.tau0
    expect(tau0).toBeGreaterThan(0)
    for (let i = 0; i < N_CITIES; i++) {
      expect(s.pheromone[i][i]).toBe(0)
      for (let j = i + 1; j < N_CITIES; j++) {
        expect(s.pheromone[i][j]).toBeCloseTo(tau0, 0)
        expect(s.pheromone[i][j]).toBe(s.pheromone[j][i])
      }
    }
  })

  it('best tour is a valid permutation', () => {
    const s = makeInitState(1, 2, 0.5, 10)
    const sorted = s.bestTour.slice().sort((a, b) => a - b)
    expect(sorted).toEqual(Array.from({ length: N_CITIES }, (_, i) => i))
  })

  it('starts with empty lastTours', () => {
    const s = makeInitState(1, 2, 0.5, 10)
    expect(s.lastTours).toHaveLength(0)
    expect(s.lastTour).toBeNull()
  })
})

describe('stepOnce', () => {
  it('transitions to depositing after numAnts ants', () => {
    const numAnts = 5
    let s = makeInitState(1, 2, 0.5, numAnts)
    for (let i = 0; i < numAnts; i++) s = stepOnce(s)
    expect(s.phase).toBe('depositing')
    expect(s.lastTours).toHaveLength(numAnts)
  })

  it('after depositing, advances epoch and returns to building', () => {
    const numAnts = 3
    let s = makeInitState(1, 2, 0.5, numAnts)
    for (let i = 0; i < numAnts; i++) s = stepOnce(s)  // build all ants
    expect(s.phase).toBe('depositing')
    s = stepOnce(s)  // deposit
    expect(s.phase).toBe('building')
    expect(s.epoch).toBe(1)
    expect(s.antIdx).toBe(0)
    expect(s.lastTours).toHaveLength(0)
  })

  it('best cost never increases (monotonic)', () => {
    let s = makeInitState(1, 2, 0.5, 5)
    for (let i = 0; i < 40; i++) {
      const prev = s.bestCost
      s = stepOnce(s)
      expect(s.bestCost).toBeLessThanOrEqual(prev + 0.001)
    }
  })

  it('built tours are valid permutations', () => {
    const expected = Array.from({ length: N_CITIES }, (_, i) => i).sort((a, b) => a - b)
    let s = makeInitState(1, 2, 0.5, 8)
    for (let i = 0; i < 8; i++) s = stepOnce(s)
    for (const tour of s.lastTours) {
      expect(tour.slice().sort((a, b) => a - b)).toEqual(expected)
    }
  })

  it('pheromone floor (tauMin) is respected after evaporation', () => {
    const evap = 0.9
    let s = makeInitState(1, 2, evap, 3)
    for (let i = 0; i < 3; i++) s = stepOnce(s)  // build
    s = stepOnce(s)  // deposit (evaporate happens here)
    for (let i = 0; i < N_CITIES; i++) {
      for (let j = i + 1; j < N_CITIES; j++) {
        expect(s.pheromone[i][j]).toBeGreaterThanOrEqual(s.tauMin - 1e-12)
      }
    }
  })
})

describe('maxPheromone', () => {
  it('is positive after a deposit phase', () => {
    let s = makeInitState(1, 2, 0.5, 5)
    for (let i = 0; i < 5; i++) s = stepOnce(s)
    s = stepOnce(s)
    expect(maxPheromone(s)).toBeGreaterThan(0)
  })
})

describe('tourLength', () => {
  it('computes a closed-loop distance', () => {
    const tour = [0, 1, 2, 3, 4]
    expect(tourLength(tour)).toBeGreaterThan(0)
  })
})

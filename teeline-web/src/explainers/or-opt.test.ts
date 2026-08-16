import { describe, it, expect } from 'vitest'
import {
  N_CITIES, SCENARIOS,
  dist, tourLength, scanBestMove, applyRelocation, makeInitState, stepOnce,
} from './or-opt-algo'
import type { SimState } from './or-opt-algo'
import { scanOnePass, makeInitState as twoOptInit } from './two-opt-algo'

// True optimum for the shared 10-city layout (see stochastic-hill tests).
const OPT = 979.0581791448133

// Run to the local optimum; returns the final state.
function runToOptimum(tour: number[]): SimState {
  let s = makeInitState(tour)
  let guard = 200
  while (s.phase !== 'local_optimum' && guard-- > 0) s = stepOnce(s)
  expect(s.phase, 'runToOptimum must converge').toBe('local_optimum')
  return s
}

// ---------------------------------------------------------------
describe('dist / tourLength', () => {
  it('dist is symmetric and non-negative', () => {
    for (let i = 0; i < N_CITIES; i++) {
      for (let j = 0; j < N_CITIES; j++) {
        expect(dist(i, j)).toBeCloseTo(dist(j, i), 10)
        expect(dist(i, j)).toBeGreaterThanOrEqual(0)
      }
    }
  })

  it('tourLength matches the known optimum', () => {
    expect(tourLength([0, 1, 2, 3, 4, 5, 8, 9, 6, 7])).toBeCloseTo(OPT, 6)
  })
})

// ---------------------------------------------------------------
// applyRelocation — direct ports of the Rust unit tests
// (src/tsp/or_opt.rs apply_relocation_*)
// ---------------------------------------------------------------
describe('applyRelocation (ported from Rust)', () => {
  it('or-1 forward: move city at index 1 to after index 3', () => {
    expect(applyRelocation([0, 1, 2, 3, 4], 1, 1, 3, false)).toEqual([0, 2, 3, 1, 4])
  })

  it('or-1 backward: move city at index 3 to after index 0', () => {
    expect(applyRelocation([0, 1, 2, 3, 4], 3, 1, 0, false)).toEqual([0, 3, 1, 2, 4])
  })

  it('or-2 forward: move pair [1,2] to after index 3', () => {
    expect(applyRelocation([0, 1, 2, 3, 4], 1, 2, 3, false)).toEqual([0, 3, 1, 2, 4])
  })

  it('or-2 reversed: move pair [1,2] as [2,1] to after index 3', () => {
    expect(applyRelocation([0, 1, 2, 3, 4], 1, 2, 3, true)).toEqual([0, 3, 2, 1, 4])
  })

  it('or-3 forward: move triple [1,2,3] to after index 4', () => {
    expect(applyRelocation([0, 1, 2, 3, 4, 5], 1, 3, 4, false)).toEqual([0, 4, 1, 2, 3, 5])
  })

  it('never mutates the input', () => {
    const tour = [0, 1, 2, 3, 4]
    const copy = [...tour]
    applyRelocation(tour, 1, 2, 3, true)
    expect(tour).toEqual(copy)
  })
})

// ---------------------------------------------------------------
describe('scanBestMove', () => {
  it('returns null for the already_optimal tour (the global optimum)', () => {
    expect(scanBestMove(SCENARIOS.already_optimal.tour)).toBeNull()
  })

  it('returns a negative-delta move for improving tours', () => {
    for (const [key, s] of Object.entries(SCENARIOS)) {
      if (key === 'already_optimal') continue
      const move = scanBestMove(s.tour)
      expect(move, `${key} should have an improving move`).not.toBeNull()
      expect(move!.delta).toBeLessThan(-1e-3)
      expect(move!.segLen).toBeGreaterThanOrEqual(1)
      expect(move!.segLen).toBeLessThanOrEqual(3)
      expect(move!.segCities).toHaveLength(move!.segLen)
    }
  })

  it('single_segment: the best move is an Or-1 relocation of the centre city', () => {
    const move = scanBestMove(SCENARIOS.single_segment.tour)!
    expect(move.segLen).toBe(1)
    expect(move.segCities).toEqual([8])
    expect(move.reversed).toBe(false)
    // applying it reaches the optimum
    const after = applyRelocation(SCENARIOS.single_segment.tour, move.i, move.segLen, move.j, move.reversed)
    expect(tourLength(after)).toBeCloseTo(OPT, 3)
  })

  it('triplet_move: the best move is an Or-3 relocation (reversed)', () => {
    const move = scanBestMove(SCENARIOS.triplet_move.tour)!
    expect(move.segLen).toBe(3)
    expect(move.segCities).toEqual([8, 5, 4])
    expect(move.reversed).toBe(true)
    const after = applyRelocation(SCENARIOS.triplet_move.tour, move.i, move.segLen, move.j, move.reversed)
    expect(tourLength(after)).toBeCloseTo(OPT, 3)
  })

  it('reports the cut/paste edges and insertion gap of the best move', () => {
    const tour = SCENARIOS.single_segment.tour
    const move = scanBestMove(tour)!
    expect(move.cutEdges).toHaveLength(3)
    expect(move.pasteEdges).toHaveLength(3)
    expect(move.insertAfter).toBe(tour[move.j])
    expect(move.insertBefore).toBe(tour[(move.j + 1) % N_CITIES])
    // improvingGaps lists positions with some improving move
    expect(move.improvingGaps.length).toBeGreaterThan(0)
    expect(move.improvingGaps).toEqual([...new Set(move.improvingGaps)].sort((a, b) => a - b))
  })

  it('delta matches the measured cost change (Rust debug assertion)', () => {
    for (const [key, s] of Object.entries(SCENARIOS)) {
      if (key === 'already_optimal') continue
      const move = scanBestMove(s.tour)!
      const before = tourLength(s.tour)
      const after = tourLength(applyRelocation(s.tour, move.i, move.segLen, move.j, move.reversed))
      expect(after - before, `${key} delta mismatch`).toBeCloseTo(move.delta, 3)
    }
  })
})

// ---------------------------------------------------------------
describe('two_opt_stuck scenario', () => {
  it('2-opt finds no improving swap, but Or-opt does', () => {
    const tour = SCENARIOS.two_opt_stuck.tour
    const { bestDelta } = scanOnePass(twoOptInit(tour))
    expect(bestDelta).toBeGreaterThanOrEqual(0) // 2-opt is stuck
    const move = scanBestMove(tour)
    expect(move).not.toBeNull() // Or-opt still improves
    expect(move!.delta).toBeLessThan(-1e-3)
  })
})

// ---------------------------------------------------------------
describe('stepOnce phase machine', () => {
  const s0 = makeInitState(SCENARIOS.single_segment.tour)

  it('idle → candidate scans and shows the move without touching the tour', () => {
    const s1 = stepOnce(s0)
    expect(s1.phase).toBe('candidate')
    expect(s1.pending).not.toBeNull()
    expect(s1.tour).toEqual(s0.tour)
    expect(s1.pass).toBe(0)
    expect(s1.moves).toBe(0)
    expect(s1.step).toBe(1)
  })

  it('candidate → move_applied applies the relocation and increments pass/moves', () => {
    const s1 = stepOnce(s0)
    const s2 = stepOnce(s1)
    expect(s2.phase).toBe('move_applied')
    expect(s2.pass).toBe(1)
    expect(s2.moves).toBe(1)
    expect(s2.tour).toEqual(applyRelocation(s0.tour, s1.pending!.i, s1.pending!.segLen, s1.pending!.j, s1.pending!.reversed))
    expect(s2.costHistory).toHaveLength(2)
    expect(s2.step).toBe(2)
  })

  it('already_optimal → local_optimum on the first scan', () => {
    const s = stepOnce(makeInitState(SCENARIOS.already_optimal.tour))
    expect(s.phase).toBe('local_optimum')
    expect(s.pass).toBe(1)
    expect(s.moves).toBe(0)
  })

  it('local_optimum is a no-op apart from the step counter', () => {
    const done = runToOptimum(SCENARIOS.single_segment.tour)
    const again = stepOnce(done)
    expect(again.phase).toBe('local_optimum')
    expect(again.tour).toEqual(done.tour)
    expect(again.step).toBe(done.step + 1)
  })
})

// ---------------------------------------------------------------
describe('convergence', () => {
  it('single_segment converges to the optimum in exactly one move', () => {
    const s = runToOptimum(SCENARIOS.single_segment.tour)
    expect(s.moves).toBe(1)
    expect(s.bestCost).toBeCloseTo(OPT, 3)
  })

  it('triplet_move converges to the optimum in exactly one move', () => {
    const s = runToOptimum(SCENARIOS.triplet_move.tour)
    expect(s.moves).toBe(1)
    expect(s.bestCost).toBeCloseTo(OPT, 3)
  })

  it('two_opt_stuck converges to an optimal tour', () => {
    const s = runToOptimum(SCENARIOS.two_opt_stuck.tour)
    expect(s.bestCost).toBeCloseTo(OPT, 3)
  })

  it('best cost never increases and costHistory is non-increasing', () => {
    const s = runToOptimum(SCENARIOS.triplet_move.tour)
    expect(s.bestCost).toBeLessThanOrEqual(s.costHistory[0])
    for (let k = 1; k < s.costHistory.length; k++) {
      expect(s.costHistory[k]).toBeLessThanOrEqual(s.costHistory[k - 1])
    }
    // initial + one push per applied move + one final push at the local optimum
    expect(s.costHistory).toHaveLength(s.moves + 2)
  })
})

// ---------------------------------------------------------------
describe('purity & determinism', () => {
  it('stepOnce never mutates its input', () => {
    const s = makeInitState(SCENARIOS.two_opt_stuck.tour)
    const snapshot = structuredClone(s)
    let cur = s
    for (let i = 0; i < 12; i++) cur = stepOnce(cur)
    expect(s).toEqual(snapshot)
  })

  it('two runs from the same tour produce identical sequences', () => {
    const a = makeInitState(SCENARIOS.two_opt_stuck.tour)
    const b = makeInitState(SCENARIOS.two_opt_stuck.tour)
    for (let i = 0; i < 12; i++) {
      const na = stepOnce(a)
      const nb = stepOnce(b)
      expect(na).toEqual(nb)
      Object.assign(a, na)
      Object.assign(b, nb)
    }
  })
})

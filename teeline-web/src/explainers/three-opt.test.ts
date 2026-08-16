import { describe, it, expect } from 'vitest'
import {
  N_CITIES, SCENARIOS, CASE_LABELS,
  dist, tourLength, scanBestMove, apply3Opt, makeInitState, stepOnce,
} from './three-opt-algo'
import type { SimState } from './three-opt-algo'
import { scanOnePass, makeInitState as twoOptInit } from './two-opt-algo'

// True optimum for the shared 10-city layout (see stochastic-hill tests).
const OPT = 979.0581791448133

function runToOptimum(tour: number[]): SimState {
  let s = makeInitState(tour)
  let guard = 300
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
// apply3Opt — the 7 reconnection cases
// ---------------------------------------------------------------
describe('apply3Opt (the 7 cases)', () => {
  // tour: 0 → [A=1, B=2, C=3, D=4, E=5] → 6  (i=1, j=3, k=5)
  const tour = [0, 1, 2, 3, 4, 5, 6]

  it('case 1: reverse s1 = [i+1..=j]', () => {
    // s1 = [2,3] → [3,2]
    expect(apply3Opt(tour, 1, 3, 5, 1)).toEqual([0, 1, 3, 2, 4, 5, 6])
  })

  it('case 2: reverse s2 = [j+1..=k]', () => {
    // s2 = [4,5] → [5,4]
    expect(apply3Opt(tour, 1, 3, 5, 2)).toEqual([0, 1, 2, 3, 5, 4, 6])
  })

  it('case 3: reverse both s1 and s2', () => {
    expect(apply3Opt(tour, 1, 3, 5, 3)).toEqual([0, 1, 3, 2, 5, 4, 6])
  })

  it('case 4: swap s2 + s1 (kept in order)', () => {
    expect(apply3Opt(tour, 1, 3, 5, 4)).toEqual([0, 1, 4, 5, 2, 3, 6])
  })

  it('case 5: s2 + rev(s1)', () => {
    expect(apply3Opt(tour, 1, 3, 5, 5)).toEqual([0, 1, 4, 5, 3, 2, 6])
  })

  it('case 6: rev(s2) + s1', () => {
    expect(apply3Opt(tour, 1, 3, 5, 6)).toEqual([0, 1, 5, 4, 2, 3, 6])
  })

  it('case 7: rev(s2) + rev(s1)', () => {
    expect(apply3Opt(tour, 1, 3, 5, 7)).toEqual([0, 1, 5, 4, 3, 2, 6])
  })

  it('never mutates the input', () => {
    const t = [0, 1, 2, 3, 4, 5, 6]
    const copy = [...t]
    apply3Opt(t, 1, 3, 5, 7)
    expect(t).toEqual(copy)
  })

  it('CASE_LABELS covers all 7 cases', () => {
    expect(Object.keys(CASE_LABELS)).toHaveLength(7)
  })
})

// ---------------------------------------------------------------
describe('scanBestMove', () => {
  it('returns null for the already_3optimal tour', () => {
    expect(scanBestMove(SCENARIOS.already_3optimal.tour)).toBeNull()
  })

  it('returns a negative-delta move for improving tours', () => {
    for (const [key, s] of Object.entries(SCENARIOS)) {
      if (key === 'already_3optimal') continue
      const move = scanBestMove(s.tour)
      expect(move, `${key} should have an improving move`).not.toBeNull()
      expect(move!.delta).toBeLessThan(0)
      expect(move!.caseNo).toBeGreaterThanOrEqual(1)
      expect(move!.caseNo).toBeLessThanOrEqual(7)
      expect(move!.removedEdges).toHaveLength(3)
      expect(move!.addedEdges).toHaveLength(3)
    }
  })

  it('single_3opt: the best move is a case-4 segment swap of two 2-city blocks', () => {
    const move = scanBestMove(SCENARIOS.single_3opt.tour)!
    expect(move.caseNo).toBe(4)
    expect(move.i).toBe(0)
    expect(move.j).toBe(2)
    expect(move.k).toBe(4)
    // applying it reaches the optimum
    const after = apply3Opt(SCENARIOS.single_3opt.tour, move.i, move.j, move.k, move.caseNo)
    expect(tourLength(after)).toBeCloseTo(OPT, 3)
  })

  it('beyond_2opt: 2-opt is stuck, 3-opt still improves (case 6)', () => {
    const tour = SCENARIOS.beyond_2opt.tour
    const { bestDelta } = scanOnePass(twoOptInit(tour))
    expect(bestDelta).toBeGreaterThanOrEqual(0) // 2-opt is stuck
    const move = scanBestMove(tour)
    expect(move).not.toBeNull()
    expect(move!.delta).toBeLessThan(0)
    expect(move!.caseNo).toBe(6)
  })

  it('delta matches the measured cost change', () => {
    for (const [key, s] of Object.entries(SCENARIOS)) {
      if (key === 'already_3optimal') continue
      const move = scanBestMove(s.tour)!
      const before = tourLength(s.tour)
      const after = tourLength(apply3Opt(s.tour, move.i, move.j, move.k, move.caseNo))
      expect(after - before, `${key} delta mismatch`).toBeCloseTo(move.delta, 3)
    }
  })
})

// ---------------------------------------------------------------
describe('stepOnce phase machine', () => {
  const s0 = makeInitState(SCENARIOS.single_3opt.tour)

  it('idle → candidate scans without touching the tour', () => {
    const s1 = stepOnce(s0)
    expect(s1.phase).toBe('candidate')
    expect(s1.pending).not.toBeNull()
    expect(s1.tour).toEqual(s0.tour)
    expect(s1.pass).toBe(0)
    expect(s1.swaps).toBe(0)
    expect(s1.step).toBe(1)
  })

  it('candidate → swap_applied applies and increments pass/swaps', () => {
    const s1 = stepOnce(s0)
    const s2 = stepOnce(s1)
    expect(s2.phase).toBe('swap_applied')
    expect(s2.pass).toBe(1)
    expect(s2.swaps).toBe(1)
    expect(s2.tour).toEqual(apply3Opt(s0.tour, s1.pending!.i, s1.pending!.j, s1.pending!.k, s1.pending!.caseNo))
    expect(s2.costHistory).toHaveLength(2)
    expect(s2.step).toBe(2)
  })

  it('already_3optimal → local_optimum on the first scan', () => {
    const s = stepOnce(makeInitState(SCENARIOS.already_3optimal.tour))
    expect(s.phase).toBe('local_optimum')
    expect(s.pass).toBe(1)
    expect(s.swaps).toBe(0)
  })

  it('local_optimum is a no-op apart from the step counter', () => {
    const done = runToOptimum(SCENARIOS.single_3opt.tour)
    const again = stepOnce(done)
    expect(again.phase).toBe('local_optimum')
    expect(again.tour).toEqual(done.tour)
    expect(again.step).toBe(done.step + 1)
  })
})

// ---------------------------------------------------------------
describe('convergence', () => {
  it('single_3opt converges to the optimum in exactly one swap', () => {
    const s = runToOptimum(SCENARIOS.single_3opt.tour)
    expect(s.swaps).toBe(1)
    expect(s.bestCost).toBeCloseTo(OPT, 3)
  })

  it('beyond_2opt converges to the optimum in exactly one swap', () => {
    const s = runToOptimum(SCENARIOS.beyond_2opt.tour)
    expect(s.swaps).toBe(1)
    expect(s.bestCost).toBeCloseTo(OPT, 3)
  })

  it('already_3optimal stays stuck just above the optimum', () => {
    const s = runToOptimum(SCENARIOS.already_3optimal.tour)
    expect(s.swaps).toBe(0)
    expect(s.bestCost).toBeGreaterThan(OPT)
    expect(s.bestCost).toBeLessThan(OPT * 1.01)
  })

  it('deep_sweep needs several passes to converge', () => {
    const s = runToOptimum(SCENARIOS.deep_sweep.tour)
    expect(s.swaps).toBeGreaterThanOrEqual(3)
    expect(s.pass).toBeGreaterThanOrEqual(s.swaps)
    expect(s.bestCost).toBeCloseTo(OPT, 3)
  })

  it('best cost never increases and costHistory is non-increasing', () => {
    const s = runToOptimum(SCENARIOS.deep_sweep.tour)
    expect(s.bestCost).toBeLessThanOrEqual(s.costHistory[0])
    for (let k = 1; k < s.costHistory.length; k++) {
      expect(s.costHistory[k]).toBeLessThanOrEqual(s.costHistory[k - 1])
    }
    // initial + one push per swap + one final push at the local optimum
    expect(s.costHistory).toHaveLength(s.swaps + 2)
  })
})

// ---------------------------------------------------------------
describe('purity & determinism', () => {
  it('stepOnce never mutates its input', () => {
    const s = makeInitState(SCENARIOS.deep_sweep.tour)
    const snapshot = structuredClone(s)
    let cur = s
    for (let i = 0; i < 20; i++) cur = stepOnce(cur)
    expect(s).toEqual(snapshot)
  })

  it('two runs from the same tour produce identical sequences', () => {
    const a = makeInitState(SCENARIOS.deep_sweep.tour)
    const b = makeInitState(SCENARIOS.deep_sweep.tour)
    for (let i = 0; i < 20; i++) {
      const na = stepOnce(a)
      const nb = stepOnce(b)
      expect(na).toEqual(nb)
      Object.assign(a, na)
      Object.assign(b, nb)
    }
  })

  it('all scenario tours are valid permutations', () => {
    for (const [key, s] of Object.entries(SCENARIOS)) {
      const sorted = s.tour.slice().sort((a, b) => a - b)
      expect(sorted, `${key} must be a permutation`).toEqual(Array.from({ length: N_CITIES }, (_, i) => i))
    }
  })
})

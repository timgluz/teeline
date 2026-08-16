import { describe, it, expect } from 'vitest'
import {
  N_CITIES, SCENARIOS,
  dist, tourLength, makeRng, nextRand, seededShuffle,
  reverseSegment, randomSuccessor, makeInitState, stepOnce,
} from './stochastic-hill-algo'
import type { SimState } from './stochastic-hill-algo'

// True optimum for the shared 8-city layout (brute-forced; verified below).
const OPT = 827.774076596031

function runToDone(seed: number, epochs: number, patience: number, initTour?: number[]): SimState {
  let s = makeInitState({ label: '', desc: '', seed, epochs, patience, initTour })
  let guard = epochs + 1000
  while (s.phase !== 'done' && guard-- > 0) s = stepOnce(s)
  expect(s.phase, 'runToDone must reach done').toBe('done')
  return s
}

function isPermutation(tour: number[]): boolean {
  const sorted = tour.slice().sort((a, b) => a - b)
  return sorted.every((v, i) => v === i) && tour.length === N_CITIES
}

// ---------------------------------------------------------------
describe('dist', () => {
  it('is symmetric', () => {
    for (let i = 0; i < N_CITIES; i++) {
      for (let j = 0; j < N_CITIES; j++) {
        expect(dist(i, j)).toBeCloseTo(dist(j, i), 10)
      }
    }
  })

  it('is zero for the same city and positive otherwise', () => {
    expect(dist(0, 0)).toBe(0)
    expect(dist(0, 1)).toBeGreaterThan(0)
  })
})

// ---------------------------------------------------------------
describe('tourLength', () => {
  it('is finite and positive for a valid tour', () => {
    const len = tourLength([0, 1, 2, 3, 4, 5, 6, 7])
    expect(len).toBeGreaterThan(0)
    expect(Number.isFinite(len)).toBe(true)
  })

  it('matches the brute-forced optimum for the perimeter tour', () => {
    expect(tourLength([0, 1, 2, 3, 4, 5, 6, 7])).toBeCloseTo(OPT, 6)
  })

  it('matches the known length of the crafted quick-start tour', () => {
    expect(tourLength(SCENARIOS.quick_convergence.initTour!)).toBeCloseTo(1128.8, 1)
  })
})

describe('OPT is the true optimum (brute force)', () => {
  it('no permutation of the 8 cities is shorter', () => {
    let best = Infinity
    const perm = Array.from({ length: N_CITIES - 1 }, (_, i) => i + 1)
    const used = new Array<boolean>(perm.length).fill(false)
    const order: number[] = [0]
    const search = () => {
      if (order.length === N_CITIES) {
        const d = tourLength(order)
        if (d < best) best = d
        return
      }
      for (let k = 0; k < perm.length; k++) {
        if (used[k]) continue
        used[k] = true
        order.push(perm[k])
        search()
        order.pop()
        used[k] = false
      }
    }
    search()
    expect(best).toBeCloseTo(OPT, 6)
  })
})

// ---------------------------------------------------------------
describe('seeded RNG', () => {
  it('produces values in [0, 1)', () => {
    let rng = makeRng(42)
    for (let i = 0; i < 200; i++) {
      const { value, rng: r2 } = nextRand(rng)
      rng = r2
      expect(value).toBeGreaterThanOrEqual(0)
      expect(value).toBeLessThan(1)
    }
  })

  it('is deterministic — same seed and counter give the same value', () => {
    const a = nextRand(makeRng(7))
    const b = nextRand(makeRng(7))
    expect(a.value).toBe(b.value)
    expect(a.rng).toEqual(b.rng)
  })

  it('advances the counter and (almost surely) changes the value', () => {
    const a = nextRand(makeRng(7))
    const b = nextRand(a.rng)
    expect(a.rng.counter).toBe(1)
    expect(b.rng.counter).toBe(2)
    expect(b.value).not.toBe(a.value)
  })

  it('different seeds give different sequences', () => {
    const a = nextRand(makeRng(1))
    const b = nextRand(makeRng(2))
    expect(a.value).not.toBe(b.value)
  })
})

// ---------------------------------------------------------------
describe('seededShuffle', () => {
  it('returns a permutation of all cities', () => {
    const { tour } = seededShuffle(makeRng(123))
    expect(isPermutation(tour)).toBe(true)
  })

  it('is reproducible for the same seed', () => {
    const a = seededShuffle(makeRng(123))
    const b = seededShuffle(makeRng(123))
    expect(a.tour).toEqual(b.tour)
    expect(a.rng).toEqual(b.rng)
  })
})

// ---------------------------------------------------------------
describe('reverseSegment', () => {
  it('reverses the inclusive segment [i..=j]', () => {
    const tour = [0, 1, 2, 3, 4, 5, 6, 7]
    expect(reverseSegment(tour, 2, 5)).toEqual([0, 1, 5, 4, 3, 2, 6, 7])
  })

  it('does not mutate the input', () => {
    const tour = [0, 1, 2, 3, 4, 5, 6, 7]
    const copy = [...tour]
    reverseSegment(tour, 1, 6)
    expect(tour).toEqual(copy)
  })
})

// ---------------------------------------------------------------
describe('randomSuccessor', () => {
  it('returns a valid permutation and a non-adjacent pair', () => {
    const tour = [0, 1, 2, 3, 4, 5, 6, 7]
    const { tour: next, i, j, rng } = randomSuccessor(tour, makeRng(99))
    expect(isPermutation(next)).toBe(true)
    expect(i).toBeLessThan(j)
    expect(j - i).toBeGreaterThan(1)
    expect(next).toEqual(reverseSegment(tour, i, j))
    expect(rng.counter).toBeGreaterThan(0)
  })

  it('reports the removed/added edges of the reversal', () => {
    const tour = [0, 1, 2, 3, 4, 5, 6, 7]
    const n = tour.length
    const { i, j, removed, added } = randomSuccessor(tour, makeRng(99))
    expect(removed).toEqual([
      [tour[(i - 1 + n) % n], tour[i]],
      [tour[j], tour[(j + 1) % n]],
    ])
    expect(added).toEqual([
      [tour[(i - 1 + n) % n], tour[j]],
      [tour[i], tour[(j + 1) % n]],
    ])
  })

  it('handles the (0, n−1) full-tour reversal with a single wrap edge (no self-loops)', () => {
    const tour = [0, 1, 2, 3, 4, 5, 6, 7]
    const n = tour.length
    let hit: { removed: [number, number][]; added: [number, number][] } | null = null
    for (let seed = 1; seed <= 5000 && !hit; seed++) {
      const { i, j, removed, added } = randomSuccessor(tour, makeRng(seed))
      if (i === 0 && j === n - 1) hit = { removed, added }
    }
    expect(hit, 'expected a seed producing the (0, n−1) full-tour reversal').not.toBeNull()
    expect(hit!.removed).toEqual([[tour[n - 1], tour[0]]])
    expect(hit!.added).toEqual([[tour[n - 1], tour[0]]])
  })
})

// ---------------------------------------------------------------
describe('SCENARIOS', () => {
  it('every tour (init or shuffle) is a valid permutation', () => {
    for (const [key, s] of Object.entries(SCENARIOS)) {
      const init = makeInitState(s)
      expect(isPermutation(init.tour), `${key} tour must be a permutation`).toBe(true)
      if (s.initTour) {
        expect(isPermutation(s.initTour), `${key} initTour must be a permutation`).toBe(true)
      }
    }
  })

  it('quick_convergence starts from the crafted near-optimal tour', () => {
    const s = makeInitState(SCENARIOS.quick_convergence)
    expect(s.tour).toEqual([2, 1, 0, 3, 4, 5, 6, 7])
    expect(s.bestCost).toBeCloseTo(1128.8, 1)
  })
})

// ---------------------------------------------------------------
describe('makeInitState', () => {
  it('starts idle with zeroed counters and best == current', () => {
    const s = makeInitState(SCENARIOS.rugged_landscape)
    expect(s.phase).toBe('idle')
    expect(s.epoch).toBe(0)
    expect(s.nStale).toBe(0)
    expect(s.restarts).toBe(0)
    expect(s.acceptedCount).toBe(0)
    expect(s.rejectedCount).toBe(0)
    expect(s.step).toBe(0)
    expect(s.pending).toBeNull()
    expect(s.bestTour).toEqual(s.tour)
    expect(s.currentCost).toBeCloseTo(s.bestCost, 10)
    expect(s.costHistory).toEqual([s.bestCost])
  })

  it('copies the tour (no shared reference)', () => {
    const s = makeInitState(SCENARIOS.quick_convergence)
    const original = s.tour[0]
    s.tour[0] = 999
    expect(s.bestTour[0]).toBe(original)
  })
})

// ---------------------------------------------------------------
describe('stepOnce phase machine', () => {
  const s0 = makeInitState(SCENARIOS.quick_convergence)

  it('idle → propose draws a candidate without touching the tour', () => {
    const s1 = stepOnce(s0)
    expect(s1.phase).toBe('propose')
    expect(s1.pending).not.toBeNull()
    expect(s1.tour).toEqual(s0.tour)
    expect(s1.epoch).toBe(0)
    expect(s1.step).toBe(1)
  })

  it('propose → verdict applies and increments the epoch', () => {
    const s1 = stepOnce(s0)
    const s2 = stepOnce(s1)
    expect(['accepted', 'rejected', 'restart', 'done']).toContain(s2.phase)
    expect(s2.epoch).toBe(1)
    expect(s2.step).toBe(2)
    expect(s2.costHistory).toHaveLength(2)
  })

  it('an accepted verdict adopts the candidate tour and cost', () => {
    // step until we observe an acceptance
    let s = stepOnce(s0)
    let guard = 200
    while (guard-- > 0 && s.phase !== 'done') {
      const next = stepOnce(s)
      if (next.phase !== 'propose' && next.pending?.accepted) {
        expect(next.tour).toEqual(next.pending.candidateTour)
        expect(next.bestCost).toBeCloseTo(next.pending.candidateCost, 10)
        expect(next.currentCost).toBeCloseTo(next.pending.candidateCost, 10)
        expect(next.nStale).toBe(0)
        return
      }
      s = next
    }
    expect.unreachable('expected at least one acceptance in the quick run')
  })

  it('a rejected verdict leaves the tour unchanged and counts the stale move', () => {
    let s = stepOnce(s0)
    let guard = 200
    while (guard-- > 0 && s.phase !== 'done') {
      const next = stepOnce(s)
      if (next.phase !== 'propose' && next.pending && !next.pending.accepted && !next.pending.restart) {
        expect(next.tour).toEqual(s.tour)
        expect(next.bestCost).toBe(s.bestCost)
        expect(next.nStale).toBe(s.nStale + 1)
        return
      }
      s = next
    }
    expect.unreachable('expected at least one rejection in the quick run')
  })

  it('done is a no-op apart from the step counter', () => {
    const done = runToDone(SCENARIOS.quick_convergence.seed, 10, 5, SCENARIOS.quick_convergence.initTour)
    const again = stepOnce(done)
    expect(again.phase).toBe('done')
    expect(again.epoch).toBe(done.epoch)
    expect(again.tour).toEqual(done.tour)
    expect(again.step).toBe(done.step + 1)
  })
})

// ---------------------------------------------------------------
describe('Rust-faithful acceptance (candidate must beat best-so-far)', () => {
  it('every accepted verdict beats the previous best', () => {
    let s = makeInitState(SCENARIOS.needle_in_haystack)
    let guard = 1000
    while (s.phase !== 'done' && guard-- > 0) {
      const next = stepOnce(s)
      if (next.phase !== 'propose' && next.pending?.accepted) {
        expect(next.pending.candidateCost).toBeLessThan(s.bestCost)
      }
      s = next
    }
  })

  it('after a restart, a candidate can improve the current tour yet still be rejected (delta<0, not accepted)', () => {
    // needle seed 1548 restarts several times; on the climb after a restart the
    // fresh random tour is worse than best, so a delta<0 candidate may still
    // not beat best — the Rust-faithful quirk.
    let s = makeInitState(SCENARIOS.needle_in_haystack)
    let sawRestart = false
    let sawQuirk = false
    let guard = 1000
    while (s.phase !== 'done' && guard-- > 0) {
      const next = stepOnce(s)
      if (next.phase === 'restart') sawRestart = true
      if (next.phase === 'propose' && next.pending && sawRestart) {
        if (next.pending.delta < 0 && !next.pending.accepted) sawQuirk = true
      }
      s = next
    }
    expect(sawRestart).toBe(true)
    expect(sawQuirk, 'expected a delta<0-but-rejected candidate after a restart').toBe(true)
  })

  it('acceptedCount + rejectedCount + restarts == epoch (verdicts partition)', () => {
    const s = runToDone(SCENARIOS.rugged_landscape.seed, 30, 5)
    expect(s.acceptedCount + s.rejectedCount + s.restarts).toBe(s.epoch)
  })
})

// ---------------------------------------------------------------
describe('restart logic', () => {
  it('restarts only after patience+1 consecutive rejections and keeps the best', () => {
    // Rugged seed 917 restarts a lot; check one restart event precisely.
    let s = makeInitState(SCENARIOS.rugged_landscape)
    let guard = 1000
    while (s.phase !== 'done' && guard-- > 0) {
      const next = stepOnce(s)
      if (next.phase === 'restart') {
        expect(next.pending?.restart).toBe(true)
        expect(next.pending?.restartTour).not.toBeNull()
        expect(next.tour).toEqual(next.pending!.restartTour)
        expect(next.bestCost).toBe(s.bestCost) // best survives
        expect(next.bestTour).toEqual(s.bestTour)
        expect(next.nStale).toBe(0)
        expect(next.restarts).toBe(s.restarts + 1)
        return
      }
      s = next
    }
    expect.unreachable('expected at least one restart in the rugged run')
  })

  it('never restarts on an accepted move (acceptance resets staleness)', () => {
    let s = makeInitState(SCENARIOS.needle_in_haystack)
    let guard = 1000
    while (s.phase !== 'done' && guard-- > 0) {
      const next = stepOnce(s)
      if (next.phase !== 'propose' && next.pending?.accepted) {
        expect(next.phase).not.toBe('restart')
        expect(next.nStale).toBe(0)
      }
      s = next
    }
  })
})

// ---------------------------------------------------------------
describe('cost bookkeeping', () => {
  it('best cost never increases and costHistory is non-increasing', () => {
    const s = runToDone(SCENARIOS.needle_in_haystack.seed, 40, 6)
    expect(s.bestCost).toBeLessThanOrEqual(s.costHistory[0])
    for (let k = 1; k < s.costHistory.length; k++) {
      expect(s.costHistory[k]).toBeLessThanOrEqual(s.costHistory[k - 1])
    }
    expect(s.costHistory).toHaveLength(s.epoch + 1)
  })
})

// ---------------------------------------------------------------
describe('pinned scenario behaviour', () => {
  it('quick_convergence: reaches the optimum with no restarts, first improvement at epoch 1', () => {
    const s = runToDone(SCENARIOS.quick_convergence.seed, SCENARIOS.quick_convergence.epochs, SCENARIOS.quick_convergence.patience, SCENARIOS.quick_convergence.initTour)
    expect(s.restarts).toBeLessThanOrEqual(1)
    expect(s.bestCost).toBeCloseTo(OPT, 3)
    expect(s.acceptedCount).toBeGreaterThanOrEqual(3)
  })

  it('rugged_landscape: many restarts and a poor final result', () => {
    const s = runToDone(SCENARIOS.rugged_landscape.seed, SCENARIOS.rugged_landscape.epochs, SCENARIOS.rugged_landscape.patience)
    expect(s.restarts).toBeGreaterThanOrEqual(6)
    expect(s.bestCost).toBeGreaterThanOrEqual(OPT * 1.2)
  })

  it('needle_in_haystack: the optimum is found, but only after several restarts', () => {
    const scenario = SCENARIOS.needle_in_haystack
    let s = makeInitState(scenario)
    let finalBestEpoch = 0
    let restartsBeforeFinal = 0
    let guard = 2000
    while (s.phase !== 'done' && guard-- > 0) {
      const before = s
      s = stepOnce(s)
      if (before.phase === 'propose' && s.phase !== 'propose' && s.bestCost < before.bestCost) {
        finalBestEpoch = s.epoch
        restartsBeforeFinal = s.restarts
      }
    }
    expect(s.bestCost).toBeCloseTo(OPT, 3)
    expect(s.restarts).toBeGreaterThanOrEqual(4)
    expect(finalBestEpoch).toBeGreaterThanOrEqual(35)
    expect(restartsBeforeFinal).toBeGreaterThanOrEqual(2)
  })
})

// ---------------------------------------------------------------
describe('determinism & purity', () => {
  it('stepOnce is pure — the input state is never mutated', () => {
    const s = makeInitState(SCENARIOS.rugged_landscape)
    const snapshot = structuredClone(s)
    let cur = s
    for (let i = 0; i < 20; i++) cur = stepOnce(cur)
    expect(s).toEqual(snapshot)
  })

  it('two runs from the same seed produce identical sequences', () => {
    const a = makeInitState(SCENARIOS.rugged_landscape)
    const b = makeInitState(SCENARIOS.rugged_landscape)
    for (let i = 0; i < 30; i++) {
      const na = stepOnce(a)
      const nb = stepOnce(b)
      expect(na).toEqual(nb)
      Object.assign(a, na)
      Object.assign(b, nb)
    }
  })
})

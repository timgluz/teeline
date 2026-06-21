// Fixed 12-city demo (same layout as SA / CS / FPA / GA explainers)
export const CITIES: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]
export const N_CITIES = CITIES.length

// PSO constants (mirrors particle_swarm.rs)
const W_MAX = 0.9
const W_MIN = 0.4
const C1 = 1.5
const C2 = 1.5
const V_MAX_FACTOR = 0.35
// ω reaches W_MIN at this epoch; stays there indefinitely after
const MAX_DISPLAY_EPOCHS = 100

// ---------------------------------------------------------------
// Types
// ---------------------------------------------------------------
export type Swap = [number, number]

export type Particle = {
  position: number[]
  velocity: Swap[]
  pbest: number[]
  pbest_cost: number
  cost: number
}

export type VelocityBreakdown = {
  inertia: number    // total inertia swaps kept across all particles this epoch
  cognitive: number  // total cognitive swaps added (toward pbest)
  social: number     // total social swaps added (toward gbest)
  applied: number    // total swaps applied after v_max cap
}

export type SimState = {
  particles: Particle[]
  gbest: number[]
  gbest_cost: number
  epoch: number
  w: number
  lastBreakdown: VelocityBreakdown | null
  newGbest: boolean
}

// ---------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------
export function tourLength(tour: number[]): number {
  if (tour.length <= 1) return 0
  let d = 0
  for (let i = 0; i < tour.length; i++) {
    const [x1, y1] = CITIES[tour[i]]
    const [x2, y2] = CITIES[tour[(i + 1) % tour.length]]
    d += Math.hypot(x2 - x1, y2 - y1)
  }
  return d
}

// Adjacent transpositions transforming a into b — mirrors swap_sequence in Rust
export function swapSequence(a: number[], b: number[]): Swap[] {
  const arr = a.slice()
  const swaps: Swap[] = []
  for (let i = 0; i < b.length; i++) {
    const j = arr.indexOf(b[i], i)
    if (j === i) continue
    for (let k = j; k > i; k--) {
      swaps.push([k - 1, k])
      ;[arr[k - 1], arr[k]] = [arr[k], arr[k - 1]]
    }
  }
  return swaps
}

// Apply first n swaps to a copy of tour — mirrors apply_swaps in Rust
export function applySwaps(tour: number[], swaps: Swap[], n: number): number[] {
  const t = tour.slice()
  const lim = Math.min(n, swaps.length)
  for (let i = 0; i < lim; i++) {
    const [a, b] = swaps[i]
    ;[t[a], t[b]] = [t[b], t[a]]
  }
  return t
}

export function shuffle(n: number): number[] {
  const t = Array.from({ length: n }, (_, i) => i)
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[t[i], t[j]] = [t[j], t[i]]
  }
  return t
}

// ---------------------------------------------------------------
// Simulation
// ---------------------------------------------------------------
export function makeInitState(nParticles: number): SimState {
  const particles: Particle[] = Array.from({ length: nParticles }, () => {
    const position = shuffle(N_CITIES)
    const cost = tourLength(position)
    return { position, velocity: [], pbest: position.slice(), pbest_cost: cost, cost }
  })
  const gbestIdx = particles.reduce((b, p, i) => p.cost < particles[b].cost ? i : b, 0)
  return {
    particles,
    gbest: particles[gbestIdx].position.slice(),
    gbest_cost: particles[gbestIdx].cost,
    epoch: 0,
    w: W_MAX,
    lastBreakdown: null,
    newGbest: false,
  }
}

// Advance one full epoch (all particles) — mirrors the epoch loop in particle_swarm.rs
export function stepEpoch(s: SimState): SimState {
  const epoch = s.epoch + 1
  const w = W_MAX - (W_MAX - W_MIN) * Math.min(epoch / MAX_DISPLAY_EPOCHS, 1.0)
  const v_max = Math.max(1, Math.ceil(N_CITIES * V_MAX_FACTOR))

  let gbest = s.gbest.slice()
  let gbest_cost = s.gbest_cost
  let newGbest = false

  let totalInertia = 0, totalCognitive = 0, totalSocial = 0, totalApplied = 0

  const particles: Particle[] = s.particles.map(p => {
    const r1 = Math.random()
    const r2 = Math.random()

    const inertia_keep = Math.round(w * p.velocity.length)
    const cog_diff = swapSequence(p.position, p.pbest)
    const cog_keep = Math.round(C1 * r1 * cog_diff.length)
    const soc_diff = swapSequence(p.position, gbest)
    const soc_keep = Math.round(C2 * r2 * soc_diff.length)

    const new_vel: Swap[] = [
      ...p.velocity.slice(0, inertia_keep),
      ...cog_diff.slice(0, cog_keep),
      ...soc_diff.slice(0, soc_keep),
    ].slice(0, v_max)

    totalInertia += inertia_keep
    totalCognitive += cog_keep
    totalSocial += soc_keep
    totalApplied += new_vel.length

    const new_pos = applySwaps(p.position, new_vel, new_vel.length)
    const cost = tourLength(new_pos)
    const improved = cost < p.pbest_cost

    if (cost < gbest_cost) {
      gbest = new_pos.slice()
      gbest_cost = cost
      newGbest = true
    }

    return {
      position: new_pos,
      velocity: new_vel,
      pbest: improved ? new_pos.slice() : p.pbest,
      pbest_cost: improved ? cost : p.pbest_cost,
      cost,
    }
  })

  return {
    particles, gbest, gbest_cost, epoch, w,
    lastBreakdown: { inertia: totalInertia, cognitive: totalCognitive, social: totalSocial, applied: totalApplied },
    newGbest,
  }
}

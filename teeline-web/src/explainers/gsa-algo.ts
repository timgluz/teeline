export const CITIES: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]
export const N_CITIES = CITIES.length

const G0 = 20.0
const ALPHA = 1.0
const V_MAX_FACTOR = 0.35
// G gauge reference: G decays from G0 toward 0 over TOTAL_EPOCHS; sim runs past this
const TOTAL_EPOCHS = 100

export type Swap = [number, number]

export type Agent = {
  position: number[]
  velocity: Swap[]
  cost: number
  mass: number
}

export type ForcePull = { kbestIdx: number; swaps: number }

export type ForceBreakdown = {
  pulls: ForcePull[]
  totalApplied: number
  G: number
}

export type SimState = {
  agents: Agent[]
  gbest: number[]
  gbest_cost: number
  epoch: number
  agentIdx: number
  G: number
  kbest: number[]
  masses: number[]
  lastBreakdown: ForceBreakdown | null
  newGbest: boolean
}

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

function computeMassesAndKbest(agents: Agent[]): { masses: number[]; kbest: number[] } {
  const worstCost = Math.max(...agents.map(a => a.cost))
  const rawFitness = agents.map(a => worstCost - a.cost)
  const sumFitness = rawFitness.reduce((s, f) => s + f, 0)
  // Uniform mass fallback when all agents share the same cost (avoids NaN)
  const masses = sumFitness < 1e-10
    ? agents.map(() => 1 / agents.length)
    : rawFitness.map(f => f / sumFitness)
  const kSize = Math.ceil(agents.length / 2)
  const kbest = agents.map((_, i) => i).sort((a, b) => masses[b] - masses[a]).slice(0, kSize)
  return { masses, kbest }
}

export function makeInitState(nAgents: number): SimState {
  const agents: Agent[] = Array.from({ length: nAgents }, () => {
    const position = shuffle(N_CITIES)
    return { position, velocity: [], cost: tourLength(position), mass: 0 }
  })
  const { masses, kbest } = computeMassesAndKbest(agents)
  agents.forEach((a, i) => { a.mass = masses[i] })
  const bestIdx = agents.reduce((b, a, i) => a.cost < agents[b].cost ? i : b, 0)
  return {
    agents,
    gbest: agents[bestIdx].position.slice(),
    gbest_cost: agents[bestIdx].cost,
    epoch: 0,
    agentIdx: 0,
    G: G0,
    kbest,
    masses,
    lastBreakdown: null,
    newGbest: false,
  }
}

// Advance one agent — mirrors the inner loop body of gravitational_search.rs
export function stepAgent(s: SimState): SimState {
  const nAgents = s.agents.length
  const v_max = Math.max(1, Math.ceil(N_CITIES * V_MAX_FACTOR))

  // Recompute masses, kbest, and G at the start of each epoch (agentIdx === 0)
  const { masses, kbest, G } = s.agentIdx === 0
    ? (() => {
        const { masses, kbest } = computeMassesAndKbest(s.agents)
        const G = G0 * Math.exp(-ALPHA * s.epoch / TOTAL_EPOCHS)
        return { masses, kbest, G }
      })()
    : { masses: s.masses, kbest: s.kbest, G: s.G }

  const i = s.agentIdx
  const agent = s.agents[i]

  // Compute gravitational pulls from each kbest agent
  const pulls: ForcePull[] = []
  let new_vel: Swap[] = []
  for (const j of kbest) {
    if (j === i) continue
    const r = Math.random()
    const n_swaps = Math.round(r * G * masses[j])
    if (n_swaps === 0) continue
    const pull = swapSequence(agent.position, s.agents[j].position)
    const taken = pull.slice(0, n_swaps)
    new_vel = [...new_vel, ...taken]
    pulls.push({ kbestIdx: j, swaps: taken.length })
  }
  new_vel = new_vel.slice(0, v_max)

  const new_pos = applySwaps(agent.position, new_vel, new_vel.length)
  const new_cost = tourLength(new_pos)

  let gbest = s.gbest
  let gbest_cost = s.gbest_cost
  let newGbest = false
  if (new_cost < gbest_cost) {
    gbest = new_pos.slice()
    gbest_cost = new_cost
    newGbest = true
  }

  const updatedAgents = s.agents.map((a, idx) =>
    idx !== i
      ? { ...a, mass: masses[idx] }
      : { position: new_pos, velocity: new_vel, cost: new_cost, mass: masses[i] }
  )

  const nextAgentIdx = i + 1
  const epochDone = nextAgentIdx >= nAgents
  return {
    agents: updatedAgents,
    gbest,
    gbest_cost,
    epoch: epochDone ? s.epoch + 1 : s.epoch,
    agentIdx: epochDone ? 0 : nextAgentIdx,
    G,
    kbest,
    masses,
    lastBreakdown: { pulls, totalApplied: new_vel.length, G },
    newGbest,
  }
}

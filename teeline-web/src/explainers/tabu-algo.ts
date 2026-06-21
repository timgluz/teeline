export const CITIES: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]
export const N_CITIES = CITIES.length

export type Move = [number, number]

export type TabuEntry = {
  key: string
  move: Move
  addedAtStep: number
}

export type EventMode = 'improvement' | 'admissible' | 'aspiration'

export type SimState = {
  tour: number[]
  best: number[]
  bestCost: number
  currentCost: number
  tabuList: TabuEntry[]
  step: number
  tenure: number
  sampleSize: number
  lastMove: Move | null
  lastEventMode: EventMode | null
  lastDelta: number | null
  aspirationHits: number
  improvements: number
  costHistory: number[]
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

export function shuffle(n: number): number[] {
  const t = Array.from({ length: n }, (_, i) => i)
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[t[i], t[j]] = [t[j], t[i]]
  }
  return t
}

export function twoOptSwap(tour: number[], i: number, j: number): number[] {
  const t = tour.slice()
  let lo = i, hi = j
  while (lo < hi) { ;[t[lo], t[hi]] = [t[hi], t[lo]]; lo++; hi-- }
  return t
}

export function moveKey(i: number, j: number): string {
  return i <= j ? `${i}-${j}` : `${j}-${i}`
}

export function sampleNeighbours(
  tour: number[],
  k: number
): Array<{ tour: number[]; move: Move; cost: number }> {
  const n = tour.length
  const results = []
  for (let s = 0; s < k; s++) {
    let i = Math.floor(Math.random() * (n - 1))
    let j = i + 1 + Math.floor(Math.random() * (n - 1 - i))
    if (j >= n) j = n - 1
    if (i === j) j = Math.min(i + 1, n - 1)
    const newTour = twoOptSwap(tour, i, j)
    results.push({
      tour: newTour,
      move: [Math.min(i, j), Math.max(i, j)] as Move,
      cost: tourLength(newTour),
    })
  }
  return results
}

export function makeInitState(tenure: number, sampleSize: number): SimState {
  const tour = shuffle(N_CITIES)
  const cost = tourLength(tour)
  return {
    tour,
    best: tour.slice(),
    bestCost: cost,
    currentCost: cost,
    tabuList: [],
    step: 0,
    tenure,
    sampleSize,
    lastMove: null,
    lastEventMode: null,
    lastDelta: null,
    aspirationHits: 0,
    improvements: 0,
    costHistory: [],
  }
}

export function stepOnce(s: SimState): SimState {
  const candidates = sampleNeighbours(s.tour, s.sampleSize)
  const tabuKeys = new Set(s.tabuList.map(e => e.key))

  const nonTabu = candidates.filter(c => !tabuKeys.has(moveKey(c.move[0], c.move[1])))
  const tabuCands = candidates.filter(c => tabuKeys.has(moveKey(c.move[0], c.move[1])))

  const aspirationCand = tabuCands
    .filter(c => c.cost < s.bestCost)
    .sort((a, b) => a.cost - b.cost)[0] ?? null

  const bestNonTabu = nonTabu.sort((a, b) => a.cost - b.cost)[0]
    ?? candidates.sort((a, b) => a.cost - b.cost)[0]

  const chosen = aspirationCand ?? bestNonTabu
  const newCost = chosen.cost
  const delta = newCost - s.currentCost
  const isAspiration = chosen === aspirationCand
  const isImprovement = newCost < s.currentCost

  const eventMode: EventMode = isAspiration
    ? 'aspiration'
    : isImprovement
      ? 'improvement'
      : 'admissible'

  const key = moveKey(chosen.move[0], chosen.move[1])
  const newEntry: TabuEntry = { key, move: chosen.move, addedAtStep: s.step }
  const newTabuList = [newEntry, ...s.tabuList].slice(0, s.tenure)

  const newBestCost = newCost < s.bestCost ? newCost : s.bestCost
  const newBest = newCost < s.bestCost ? chosen.tour.slice() : s.best

  return {
    ...s,
    tour: chosen.tour,
    best: newBest,
    bestCost: newBestCost,
    currentCost: newCost,
    tabuList: newTabuList,
    step: s.step + 1,
    lastMove: chosen.move,
    lastEventMode: eventMode,
    lastDelta: delta,
    aspirationHits: s.aspirationHits + (isAspiration ? 1 : 0),
    improvements: s.improvements + (eventMode === 'improvement' ? 1 : 0),
    costHistory: [...s.costHistory.slice(-59), newCost],
  }
}

import { useState, useRef, useEffect, useCallback } from "preact/hooks"

// ---------------------------------------------------------------
// Fixed 12-city demo (same layout as SA / CS / FPA explainers)
// ---------------------------------------------------------------
const CITIES: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]
const N_CITIES = CITIES.length

// ---------------------------------------------------------------
// Algorithm helpers
// ---------------------------------------------------------------
function tourLength(tour: number[]): number {
  let d = 0
  for (let i = 0; i < tour.length; i++) {
    const [x1, y1] = CITIES[tour[i]]
    const [x2, y2] = CITIES[tour[(i + 1) % tour.length]]
    d += Math.hypot(x2 - x1, y2 - y1)
  }
  return d
}

function fitness(tour: number[]): number {
  const d = tourLength(tour)
  return d === 0 ? 0 : 1 / d
}

function shuffle(n: number): number[] {
  const t = Array.from({ length: n }, (_, i) => i)
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[t[i], t[j]] = [t[j], t[i]]
  }
  return t
}

// Random non-adjacent pair (i < j, j − i ≥ 2) — mirrors random_position_pair in Rust
function randomPair(n: number): [number, number] {
  for (let attempt = 0; attempt < 20; attempt++) {
    const i = Math.floor(Math.random() * (n - 2))
    const j = i + 2 + Math.floor(Math.random() * (n - 2 - i))
    if (j < n) return [i, j]
  }
  return [0, n - 1]
}

// Ordered crossover — mirrors ordered_crossover_genes in Rust exactly
// Segment [from..to] is copied from p2; remaining cities filled from p1 in order
function orderedCrossover(p1: number[], p2: number[], from: number, to: number): number[] {
  const n = p1.length
  const child = new Array(n).fill(-1)
  const inSegment = new Set<number>()
  for (let k = from; k <= to; k++) {
    child[k] = p2[k]
    inSegment.add(p2[k])
  }
  let j = (to + 1) % n
  let k = (to + 1) % n
  for (let iter = 0; iter < n; iter++) {
    const city = p1[k]
    if (!inSegment.has(city)) {
      child[j] = city
      j = (j + 1) % n
    }
    k = (k + 1) % n
  }
  return child
}

// Segment reversal mutation — mirrors TspGenotype.mutate
function mutate(tour: number[]): number[] {
  const [from, to] = randomPair(tour.length)
  const next = tour.slice()
  let lo = from, hi = to
  while (lo < hi) { [next[lo], next[hi]] = [next[hi], next[lo]]; lo++; hi-- }
  return next
}

// Fitness-proportionate (roulette wheel) selection — mirrors TspPopulation.random_selection
function rouletteSelect(fitnesses: number[]): number {
  const total = fitnesses.reduce((a, b) => a + b, 0)
  if (total === 0) return Math.floor(Math.random() * fitnesses.length)
  let r = Math.random() * total
  for (let i = 0; i < fitnesses.length; i++) {
    r -= fitnesses[i]
    if (r <= 0) return i
  }
  return fitnesses.length - 1
}

// ---------------------------------------------------------------
// Simulation state (all mutable data lives here to avoid stale closures)
// ---------------------------------------------------------------
type SimState = {
  population: number[][]   // sorted best→worst
  fitnesses: number[]
  best: number[]
  bestCost: number
  generation: number
  stepInGen: number
  pendingChildren: number[][]
  totalCrossovers: number
  totalMutations: number
  genHistory: number[]     // bestCost at each completed generation
}

function initSorted(population: number[][]): { population: number[][]; fitnesses: number[] } {
  const fitnesses = population.map(fitness)
  const order = fitnesses.map((f, i) => ({ f, i })).sort((a, b) => b.f - a.f).map(x => x.i)
  return { population: order.map(i => population[i]), fitnesses: order.map(i => fitnesses[i]) }
}

function makeInitState(popSize: number): SimState {
  const raw = Array.from({ length: popSize }, () => shuffle(N_CITIES))
  const { population, fitnesses } = initSorted(raw)
  return {
    population, fitnesses,
    best: population[0].slice(), bestCost: tourLength(population[0]),
    generation: 0, stepInGen: 0, pendingChildren: [],
    totalCrossovers: 0, totalMutations: 0, genHistory: [],
  }
}

// ---------------------------------------------------------------
// SVG helpers
// ---------------------------------------------------------------
function polyPts(tour: number[]): string {
  const pts = tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ")
  return pts + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}`
}

// ---------------------------------------------------------------
// BestTourSVG — current global best
// ---------------------------------------------------------------
function BestTourSVG({ tour }: { tour: number[] }) {
  return (
    <svg viewBox="0 0 300 300" className="ga-canvas" role="img" aria-label="GA best tour">
      <rect x={0} y={0} width={300} height={300} className="ga-bg" />
      <polyline points={polyPts(tour)} className="ga-best-tour" />
      {CITIES.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y} r={4} className="ga-city" />
      ))}
      {CITIES.map(([x, y], i) => (
        <text key={i} x={x + 6} y={y - 5} className="ga-city-label">{i}</text>
      ))}
    </svg>
  )
}

// ---------------------------------------------------------------
// PopulationHeatmap — adapted from CS NestHeatmap
// ---------------------------------------------------------------
interface PopulationHeatmapProps {
  fitnesses: number[]
  parentAIdx: number
  parentBIdx: number
  eliteCount: number
}
function PopulationHeatmap({ fitnesses, parentAIdx, parentBIdx, eliteCount }: PopulationHeatmapProps) {
  if (!fitnesses.length) return null
  const minF = Math.min(...fitnesses)
  const maxF = Math.max(...fitnesses)
  const range = maxF - minF || 1
  return (
    <div className="ga-heatmap">
      <div className="ga-heatmap-label">POPULATION</div>
      {fitnesses.map((f, i) => {
        const norm = (f - minF) / range
        const hue = norm * 120
        const bg = `hsl(${hue.toFixed(0)}, 55%, 42%)`
        const isElite = i < eliteCount
        const isA = i === parentAIdx
        const isB = i === parentBIdx
        const border = isA ? '2.5px solid #3b82f6'
          : isB ? '2.5px solid #d97706'
          : isElite ? '2.5px solid #f59e0b'
          : '2px solid transparent'
        const statusText = isA && isB ? 'A + B' : isA ? 'parent A' : isB ? 'parent B'
          : isElite ? 'elite' : `fitness ${(norm * 100).toFixed(0)}%`
        return (
          <div key={i} className="ga-heatmap-cell" style={{ background: bg, border }}>
            <span className="ga-heatmap-idx">{i}</span>
            <div className="ga-heatmap-overlay">{statusText}</div>
          </div>
        )
      })}
    </div>
  )
}

// ---------------------------------------------------------------
// MiniTourSVG — for the crossover panel
// ---------------------------------------------------------------
interface MiniTourSVGProps {
  tour: number[]
  segmentCityIds: Set<number>
  edgeColor: string
  label: string
  mutated?: boolean
}
function MiniTourSVG({ tour, segmentCityIds, edgeColor, label, mutated }: MiniTourSVGProps) {
  return (
    <div className="ga-mini-wrap">
      <svg viewBox="0 0 300 300" className="ga-mini-svg">
        <rect x={0} y={0} width={300} height={300} className="ga-bg" />
        <polyline points={polyPts(tour)} fill="none" stroke={edgeColor} strokeWidth={2} strokeLinejoin="round" />
        {CITIES.map(([x, y], i) => {
          const inSeg = segmentCityIds.has(i)
          return (
            <circle key={i} cx={x} cy={y}
              r={inSeg ? 7 : 4}
              fill={inSeg ? "#f97316" : "#f2a154"}
              stroke={inSeg ? "#fff" : "none"}
              strokeWidth={1.5}
            />
          )
        })}
      </svg>
      <div className="ga-mini-label">{label}{mutated ? " (mutated)" : ""}</div>
    </div>
  )
}

// ---------------------------------------------------------------
// CrossoverPanel — three mini SVGs side by side
// ---------------------------------------------------------------
interface CrossoverPanelProps {
  parentA: number[]
  parentB: number[]
  child: number[]
  segment: [number, number]
  childMutated: boolean
}
function CrossoverPanel({ parentA, parentB, child, segment, childMutated }: CrossoverPanelProps) {
  const [from, to] = segment
  // Cities occupying the OX segment positions in parent B
  const segmentCityIds = new Set<number>()
  for (let k = from; k <= to; k++) segmentCityIds.add(parentB[k])
  return (
    <div className="ga-crossover-panel">
      <div className="ga-crossover-title">
        Ordered Crossover — positions [{from}..{to}] inherited from B
      </div>
      <div className="ga-crossover-row">
        <MiniTourSVG tour={parentA} segmentCityIds={segmentCityIds} edgeColor="#3b82f6" label="Parent A" />
        <div className="ga-crossover-arrow">→</div>
        <MiniTourSVG tour={parentB} segmentCityIds={segmentCityIds} edgeColor="#d97706" label="Parent B" />
        <div className="ga-crossover-arrow">→</div>
        <MiniTourSVG tour={child} segmentCityIds={segmentCityIds} edgeColor="#16a34a" label="Child" mutated={childMutated} />
      </div>
    </div>
  )
}

// ---------------------------------------------------------------
// FitnessSparkline — best distance per completed generation
// ---------------------------------------------------------------
function FitnessSparkline({ history }: { history: number[] }) {
  const W = 300, H = 54
  const visible = history.slice(-20)
  if (!visible.length) return <svg viewBox={`0 0 ${W} ${H}`} className="ga-spark" />
  const minV = Math.min(...visible), maxV = Math.max(...visible)
  const range = maxV - minV || 1
  const slotW = W / 20
  const barW = Math.max(1, slotW - 1)
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="ga-spark">
      {visible.map((v, i) => {
        const norm = 1 - (v - minV) / range  // taller bar = lower distance = better
        const bh = Math.max(2, norm * (H - 4))
        return (
          <rect key={i} x={i * slotW} y={H - bh - 2} width={barW} height={bh}
            fill="#16a34a" opacity={0.8} />
        )
      })}
    </svg>
  )
}

// ---------------------------------------------------------------
// Main component
// ---------------------------------------------------------------
export default function GAExplainer() {
  const [popSize, setPopSize] = useState(8)
  const [nElite, setNElite] = useState(2)
  const [mutationProb, setMutationProb] = useState(0.20)
  const [speed, setSpeed] = useState(5)

  // Refs for stable access inside stepOnce (avoids stale closures)
  const popSizeRef = useRef(8)
  const nEliteRef = useRef(2)
  const mutProbRef = useRef(0.20)
  const simRef = useRef<SimState>(makeInitState(8))

  // Display state (synced from simRef after each step)
  const [best, setBest] = useState<number[]>(() => simRef.current.best)
  const [bestCost, setBestCost] = useState(() => simRef.current.bestCost)
  const [fitnesses, setFitnesses] = useState<number[]>(() => simRef.current.fitnesses)
  const [generation, setGeneration] = useState(0)
  const [stepInGen, setStepInGen] = useState(0)
  const [totalCrossovers, setTotalCrossovers] = useState(0)
  const [totalMutations, setTotalMutations] = useState(0)
  const [genHistory, setGenHistory] = useState<number[]>([])

  // Per-step crossover annotation
  const [parentAIdx, setParentAIdx] = useState(-1)
  const [parentBIdx, setParentBIdx] = useState(-1)
  const [lastParentA, setLastParentA] = useState<number[]>([])
  const [lastParentB, setLastParentB] = useState<number[]>([])
  const [lastChild, setLastChild] = useState<number[]>([])
  const [lastSegment, setLastSegment] = useState<[number, number]>([0, 3])
  const [lastChildMutated, setLastChildMutated] = useState(false)
  const [chipText, setChipText] = useState("")
  const [chipMode, setChipMode] = useState<"cross" | "gen" | "">("")
  const [running, setRunning] = useState(false)

  const reinit = useCallback((size: number) => {
    const s = makeInitState(size)
    simRef.current = s
    setBest(s.best.slice())
    setBestCost(s.bestCost)
    setFitnesses(s.fitnesses.slice())
    setGeneration(0); setStepInGen(0)
    setTotalCrossovers(0); setTotalMutations(0)
    setGenHistory([])
    setParentAIdx(-1); setParentBIdx(-1)
    setLastParentA([]); setLastParentB([]); setLastChild([])
    setLastSegment([0, 3]); setLastChildMutated(false)
    setChipText(""); setChipMode("")
    setRunning(false)
  }, [])

  const stepOnce = useCallback(() => {
    const s = simRef.current
    const ne = nEliteRef.current
    const mp = mutProbRef.current
    const stepsPerGen = Math.max(1, Math.floor((s.population.length - ne) / 2))

    // Roulette-select two distinct parents
    const aIdx = rouletteSelect(s.fitnesses)
    let bIdx = rouletteSelect(s.fitnesses)
    if (bIdx === aIdx) bIdx = (aIdx + 1) % s.population.length
    const parentA = s.population[aIdx]
    const parentB = s.population[bIdx]

    // OX crossover — produce two children from swapped parents
    const [from, to] = randomPair(N_CITIES)
    const child1 = orderedCrossover(parentA, parentB, from, to)
    const child2 = orderedCrossover(parentB, parentA, from, to)

    // Optional mutation
    const mut1 = Math.random() < mp
    const mut2 = Math.random() < mp
    const final1 = mut1 ? mutate(child1) : child1
    const final2 = mut2 ? mutate(child2) : child2

    const newMutations = s.totalMutations + (mut1 ? 1 : 0) + (mut2 ? 1 : 0)
    const newCrossovers = s.totalCrossovers + 1
    const newStepInGen = s.stepInGen + 1
    const newPending = [...s.pendingChildren, final1, final2]

    // Track best
    const child1Cost = tourLength(final1)
    const child2Cost = tourLength(final2)
    const parentACost = tourLength(parentA)
    let newBest = s.best
    let newBestCost = s.bestCost
    if (child1Cost < s.bestCost) { newBest = final1.slice(); newBestCost = child1Cost }
    if (child2Cost < newBestCost) { newBest = final2.slice(); newBestCost = child2Cost }

    let newPopulation = s.population
    let newFitnesses = s.fitnesses
    let newGeneration = s.generation
    let newPendingFinal = newPending
    let newGenHistory = s.genHistory
    let genJustCompleted = false

    if (newStepInGen >= stepsPerGen) {
      // Commit generation: keep elite, fill rest with pending children
      const sorted = s.fitnesses.map((f, i) => ({ f, i })).sort((a, b) => b.f - a.f)
      const elites = sorted.slice(0, ne).map(x => s.population[x.i])
      const slots = s.population.length - ne
      const combined = [...elites, ...newPending.slice(0, slots)]
      const { population: nextPop, fitnesses: nextFit } = initSorted(combined)
      newPopulation = nextPop
      newFitnesses = nextFit
      newGeneration = s.generation + 1
      newPendingFinal = []
      newGenHistory = [...s.genHistory, newBestCost]
      genJustCompleted = true
    }

    simRef.current = {
      ...s,
      population: newPopulation, fitnesses: newFitnesses,
      best: newBest, bestCost: newBestCost,
      generation: newGeneration,
      stepInGen: genJustCompleted ? 0 : newStepInGen,
      pendingChildren: newPendingFinal,
      totalCrossovers: newCrossovers, totalMutations: newMutations,
      genHistory: newGenHistory,
    }

    // Sync display state
    setBest(newBest.slice())
    setBestCost(newBestCost)
    setFitnesses(newFitnesses.slice())
    setGeneration(newGeneration)
    setStepInGen(genJustCompleted ? 0 : newStepInGen)
    setTotalCrossovers(newCrossovers)
    setTotalMutations(newMutations)
    if (genJustCompleted) setGenHistory(newGenHistory)

    // Crossover annotation
    setParentAIdx(aIdx)
    setParentBIdx(bIdx)
    setLastParentA(parentA.slice())
    setLastParentB(parentB.slice())
    setLastChild(final1.slice())
    setLastSegment([from, to])
    setLastChildMutated(mut1)

    if (genJustCompleted) {
      setChipText(`⭐ Generation ${newGeneration} complete — best ${newBestCost.toFixed(0)}`)
      setChipMode("gen")
    } else {
      const delta = child1Cost - parentACost
      const sign = delta < 0 ? "✅" : "➡️"
      const mutStr = mut1 ? " · 🔬 mutated" : ""
      setChipText(
        `🧬 [${from}..${to}] from B → child  ${sign} ${delta < 0 ? `−${Math.abs(delta).toFixed(0)}` : `+${delta.toFixed(0)}`}${mutStr}`
      )
      setChipMode("cross")
    }
  }, []) // no deps — all mutable state read from simRef / nEliteRef / mutProbRef

  // Running loop
  useEffect(() => {
    if (!running) return
    const ms = Math.max(50, 660 - speed * 66)
    const id = setInterval(stepOnce, ms)
    return () => clearInterval(id)
  }, [running, speed, stepOnce])

  const stepsPerGen = Math.max(1, Math.floor((popSize - nElite) / 2))
  const hasCrossover = lastParentA.length > 0

  return (
    <div className="ga-explainer">
      <style>{CSS}</style>

      <div className="ga-header">
        <div className="ga-eyebrow">teeline · algorithms/ga</div>
        <h2 className="ga-title">Genetic Algorithm</h2>
        <p className="ga-sub">
          A population of tours evolves step by step. Two parents are chosen by{" "}
          <strong>roulette selection</strong> (fitter = more likely), combined via{" "}
          <strong>ordered crossover (OX)</strong>, and optionally <strong>mutated</strong>.
          The top <code>{nElite}</code> elite individuals survive unchanged each generation.
        </p>
      </div>

      <div className="ga-viz-row">
        <BestTourSVG tour={best} />
        <PopulationHeatmap
          fitnesses={fitnesses}
          parentAIdx={parentAIdx}
          parentBIdx={parentBIdx}
          eliteCount={nElite}
        />
      </div>

      <div className="ga-legend">
        <span className="ga-swatch ga-swatch-best" />best tour
        <span className="ga-swatch ga-swatch-parentA" />parent A
        <span className="ga-swatch ga-swatch-parentB" />parent B
        <span className="ga-swatch ga-swatch-elite" />elite
        <span className="ga-swatch ga-swatch-segment" />OX segment
      </div>

      {hasCrossover && (
        <CrossoverPanel
          parentA={lastParentA}
          parentB={lastParentB}
          child={lastChild}
          segment={lastSegment}
          childMutated={lastChildMutated}
        />
      )}

      {chipText && (
        <div className={`ga-chip ${chipMode === "gen" ? "ga-chip-gen" : "ga-chip-cross"}`}>
          {chipText}
        </div>
      )}

      <div className="ga-spark-wrap">
        <div className="ga-spark-label">best distance per generation</div>
        <FitnessSparkline history={genHistory} />
      </div>

      <div className="ga-stats">
        <div className="ga-stat">
          <span className="ga-stat-val">{generation}</span>
          <span className="ga-stat-label">gen</span>
        </div>
        <div className="ga-stat">
          <span className="ga-stat-val">{stepInGen}/{stepsPerGen}</span>
          <span className="ga-stat-label">step</span>
        </div>
        <div className="ga-stat">
          <span className="ga-stat-val">{bestCost.toFixed(0)}</span>
          <span className="ga-stat-label">best dist</span>
        </div>
        <div className="ga-stat">
          <span className="ga-stat-val">{totalCrossovers}</span>
          <span className="ga-stat-label">crossovers</span>
        </div>
        <div className="ga-stat">
          <span className="ga-stat-val">{totalMutations}</span>
          <span className="ga-stat-label">mutations</span>
        </div>
      </div>

      <div className="ga-config">
        <label className="ga-label">
          population = {popSize}
          <input type="range" min={4} max={16} step={2} value={popSize} className="ga-slider"
            onInput={e => {
              const v = Number((e.target as HTMLInputElement).value)
              setPopSize(v); popSizeRef.current = v; reinit(v)
            }}
          />
        </label>
        <label className="ga-label">
          elite = {nElite}
          <input type="range" min={1} max={4} step={1} value={nElite} className="ga-slider"
            onInput={e => {
              const v = Number((e.target as HTMLInputElement).value)
              setNElite(v); nEliteRef.current = v; reinit(popSizeRef.current)
            }}
          />
        </label>
        <label className="ga-label">
          mutation = {(mutationProb * 100).toFixed(0)}%
          <input type="range" min={0} max={0.5} step={0.05} value={mutationProb} className="ga-slider"
            onInput={e => {
              const v = Number((e.target as HTMLInputElement).value)
              setMutationProb(v); mutProbRef.current = v
            }}
          />
        </label>
        <label className="ga-label">
          speed = {speed}
          <input type="range" min={1} max={10} step={1} value={speed} className="ga-slider"
            onInput={e => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
        </label>
      </div>

      <div className="ga-controls">
        <button className="ga-btn" onClick={stepOnce} disabled={running}>Step</button>
        <button className="ga-btn ga-btn-primary" onClick={() => setRunning(r => !r)}>
          {running ? "Pause" : "Run"}
        </button>
        <button className="ga-btn" onClick={() => reinit(popSizeRef.current)}>Reset</button>
      </div>

      <div className="ga-footer">
        {N_CITIES} cities · population {popSize} · elite {nElite} · OX crossover · reversal mutation
      </div>
    </div>
  )
}

// ---------------------------------------------------------------
// Styles
// ---------------------------------------------------------------
const CSS = `
.ga-explainer {
  --accent: #0969da;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --best: #16a34a;
  --parentA: #3b82f6;
  --parentB: #d97706;
  --segment: #f97316;
  --elite: #f59e0b;
  --city: #f2a154;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 20px;
  max-width: 760px;
}

/* Header */
.ga-header { margin-bottom: 16px; }
.ga-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.ga-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.ga-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.ga-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(9,105,218,0.08);
  color: var(--accent); padding: 1px 4px; border-radius: 4px;
}

/* Viz row: main canvas + heatmap side by side */
.ga-viz-row {
  display: flex; gap: 12px; margin: 14px 0 8px; align-items: flex-start;
}

/* Best tour canvas */
.ga-canvas {
  flex: 1; min-width: 0; height: auto; display: block;
  border: 1px solid var(--line); border-radius: 8px;
}
.ga-bg { fill: var(--panel); }
.ga-best-tour { fill: none; stroke: var(--best); stroke-width: 2.5; stroke-linejoin: round; }
.ga-city { fill: var(--city); }
.ga-city-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: var(--text);
  stroke: #fff; stroke-width: 3; paint-order: stroke fill;
  pointer-events: none; user-select: none;
}

/* Legend */
.ga-legend {
  display: flex; flex-wrap: wrap; gap: 10px; font-size: 0.78rem;
  color: var(--muted); align-items: center; margin-bottom: 12px;
}
.ga-swatch {
  display: inline-block; margin-right: 3px; vertical-align: middle;
  width: 22px; height: 3px; border-radius: 2px;
}
.ga-swatch-best { background: var(--best); }
.ga-swatch-parentA { background: var(--parentA); }
.ga-swatch-parentB { background: var(--parentB); }
.ga-swatch-elite { background: var(--elite); height: 8px; border-radius: 2px; }
.ga-swatch-segment { background: var(--segment); border-radius: 50%; height: 8px; width: 8px; }

/* Population heatmap column */
.ga-heatmap {
  width: 100px; flex-shrink: 0; display: flex; flex-direction: column; gap: 4px;
}
.ga-heatmap-label {
  font-size: 0.65rem; color: var(--muted); text-transform: uppercase;
  letter-spacing: 0.07em; margin-bottom: 2px; text-align: center;
}
.ga-heatmap-cell {
  position: relative; height: 28px; border-radius: 5px;
  display: flex; align-items: center; padding: 0 6px;
  cursor: default; box-sizing: border-box;
}
.ga-heatmap-idx { font-size: 0.75rem; font-weight: 700; color: #fff; }
.ga-heatmap-overlay {
  position: absolute; inset: 0; display: flex; align-items: center;
  justify-content: center; background: rgba(0,0,0,0.48); border-radius: 5px;
  font-size: 0.65rem; font-weight: 700; color: #fff;
  opacity: 0; transition: opacity 0.12s; pointer-events: none;
}
.ga-heatmap-cell:hover .ga-heatmap-overlay { opacity: 1; }

/* Crossover panel */
.ga-crossover-panel {
  background: var(--panel); border: 1px solid var(--line); border-radius: 8px;
  padding: 12px; margin-bottom: 10px;
}
.ga-crossover-title {
  font-size: 0.78rem; color: var(--muted); margin-bottom: 10px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.ga-crossover-row { display: flex; align-items: center; gap: 6px; }
.ga-crossover-arrow { color: var(--muted); font-size: 1.2rem; flex-shrink: 0; }
.ga-mini-wrap { flex: 1; min-width: 0; }
.ga-mini-svg {
  width: 100%; height: auto; display: block;
  border: 1px solid var(--line); border-radius: 6px; background: var(--bg);
}
.ga-mini-label {
  font-size: 0.7rem; color: var(--muted); text-align: center; margin-top: 4px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

/* Event chip */
.ga-chip {
  font-size: 0.82rem; padding: 6px 10px; border-radius: 6px; margin-bottom: 10px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.ga-chip-cross { background: #dbeafe; color: #1e40af; }
.ga-chip-gen { background: #fef9c3; color: #854d0e; }

/* Fitness sparkline */
.ga-spark-wrap { margin-bottom: 10px; }
.ga-spark-label {
  font-size: 0.7rem; color: var(--muted); text-transform: uppercase;
  letter-spacing: 0.06em; margin-bottom: 4px;
}
.ga-spark {
  width: 100%; height: 54px; display: block;
  border: 1px solid var(--line); border-radius: 6px; background: var(--panel);
}

/* Stats row */
.ga-stats { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 14px; }
.ga-stat {
  flex: 1; min-width: 60px;
  background: var(--panel); border: 1px solid var(--line);
  border-radius: 8px; padding: 6px 8px; text-align: center;
}
.ga-stat-val { display: block; font-size: 1rem; font-weight: 700; }
.ga-stat-label {
  font-size: 0.68rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.06em;
}

/* Config sliders */
.ga-config { display: flex; flex-direction: column; gap: 8px; margin-bottom: 14px; }
.ga-label { font-size: 0.82rem; color: var(--muted); display: flex; align-items: center; gap: 8px; }
.ga-slider { flex: 1; accent-color: var(--accent); cursor: pointer; }

/* Controls */
.ga-controls { display: flex; gap: 8px; margin-bottom: 12px; }
.ga-btn {
  padding: 6px 14px; border: 1.5px solid var(--line); border-radius: 6px;
  background: var(--bg); color: var(--text); font-size: 0.85rem;
  cursor: pointer; transition: border-color 0.12s;
}
.ga-btn:hover:not(:disabled) { border-color: var(--accent); }
.ga-btn:disabled { opacity: 0.4; cursor: default; }
.ga-btn-primary { color: var(--accent); border-color: var(--accent); }

/* Footer */
.ga-footer {
  font-size: 0.72rem; color: var(--muted);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  border-top: 1px solid var(--line); padding-top: 8px; margin-top: 4px;
}
`

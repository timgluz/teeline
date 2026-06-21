import { useState, useRef, useEffect, useCallback } from "preact/hooks"

// ---------------------------------------------------------------
// Fixed demo instance: same 18 cities as fpa.tsx (300×300 canvas)
// ---------------------------------------------------------------
const CITIES: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]
const N_CITIES = CITIES.length

// ---------------------------------------------------------------
// Algorithm helpers (shared pattern with fpa.tsx)
// ---------------------------------------------------------------
function randNorm(): number {
  const u1 = Math.max(Math.random(), 1e-12)
  return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * Math.random())
}

const LEVY_SIGMA = 0.7213
function levyStep(): number {
  const u = randNorm() * LEVY_SIGMA
  const v = Math.abs(randNorm())
  if (v < 1e-12) return 1.0
  return Math.abs(u / Math.pow(v, 2 / 3))
}

function tourLength(tour: number[]): number {
  let total = 0
  for (let i = 0; i < tour.length; i++) {
    const [x1, y1] = CITIES[tour[i]]
    const [x2, y2] = CITIES[tour[(i + 1) % tour.length]]
    total += Math.hypot(x2 - x1, y2 - y1)
  }
  return total
}

function shuffle(n: number): number[] {
  const t = Array.from({ length: n }, (_, i) => i)
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[t[i], t[j]] = [t[j], t[i]]
  }
  return t
}

// Apply k random 2-opt reversals — matches cuckoo_search.rs exactly
function applyKRandom2Opt(tour: number[], k: number): number[] {
  const n = tour.length
  const result = tour.slice()
  for (let iter = 0; iter < k; iter++) {
    const i = Math.floor(Math.random() * (n - 1))
    const j = i + 1 + Math.floor(Math.random() * (n - 1 - i))
    result.splice(i, j - i + 1, ...result.slice(i, j + 1).reverse())
  }
  return result
}

// Compute edge sets for highlighting changed edges
type EdgeSet = Set<string>
function edgeKey(a: number, b: number): string {
  return a < b ? `${a}-${b}` : `${b}-${a}`
}
function tourEdgeSet(tour: number[]): EdgeSet {
  const s: EdgeSet = new Set()
  for (let i = 0; i < tour.length; i++) {
    s.add(edgeKey(tour[i], tour[(i + 1) % tour.length]))
  }
  return s
}

type EdgeDiff = {
  removed: [number, number][]   // orange dashed
  added: [number, number][]     // green
  changedCities: Set<number>    // larger circles
}

function computeEdgeDiff(before: number[], after: number[]): EdgeDiff {
  const bSet = tourEdgeSet(before)
  const aSet = tourEdgeSet(after)
  const removed: [number, number][] = []
  const added: [number, number][] = []
  const changedCities = new Set<number>()

  for (let i = 0; i < before.length; i++) {
    const a = before[i], b = before[(i + 1) % before.length]
    const key = edgeKey(a, b)
    if (!aSet.has(key)) {
      removed.push([a, b])
      changedCities.add(a)
      changedCities.add(b)
    }
  }
  for (let i = 0; i < after.length; i++) {
    const a = after[i], b = after[(i + 1) % after.length]
    const key = edgeKey(a, b)
    if (!bSet.has(key)) {
      added.push([a, b])
      changedCities.add(a)
      changedCities.add(b)
    }
  }
  return { removed, added, changedCities }
}

// ---------------------------------------------------------------
// Simulation state
// ---------------------------------------------------------------
type SimState = {
  nests: number[][]
  costs: number[]
  best: number[]
  bestCost: number
  nestIdx: number       // which nest within the current epoch
  epoch: number
  step: number
  replacements: number
  abandonments: number
}

function makeInitState(nNests: number): SimState {
  const nests = Array.from({ length: nNests }, () => shuffle(N_CITIES))
  const costs = nests.map(tourLength)
  const bestIdx = costs.indexOf(Math.min(...costs))
  return {
    nests, costs,
    best: nests[bestIdx].slice(),
    bestCost: costs[bestIdx],
    nestIdx: 0, epoch: 0, step: 0,
    replacements: 0, abandonments: 0,
  }
}

// ---------------------------------------------------------------
// QualityBars — standalone, reusable for population explainers
// ---------------------------------------------------------------
interface NestHeatmapProps {
  costs: number[]
  activeIdx: number
  targetIdx: number
  abandonedIdxs: number[]
}

function NestHeatmap({ costs, activeIdx, targetIdx, abandonedIdxs }: NestHeatmapProps) {
  if (!costs.length) return null
  const minC = Math.min(...costs)
  const maxC = Math.max(...costs)
  const range = maxC - minC || 1

  return (
    <div className="cs-heatmap" aria-label="Nest quality heatmap">
      <div className="cs-heatmap-title">Nests</div>
      {costs.map((c, i) => {
        const norm = (maxC - c) / range          // 1 = best, 0 = worst
        const hue = Math.round(norm * 120)       // 120=green, 0=red
        const isAbandoned = abandonedIdxs.includes(i)
        const bg = isAbandoned ? "#cbd5e1" : `hsl(${hue},55%,42%)`
        const border =
          i === activeIdx ? "2px solid #3b82f6" :
          i === targetIdx ? "2px solid #d97706" : "2px solid transparent"
        return (
          <div
            key={i}
            className="cs-heatmap-cell"
            title={`Nest ${i}: ${c.toFixed(0)}`}
            style={{ background: bg, outline: border, outlineOffset: "1px" }}
          >
            <span className="cs-heatmap-idx">{i}</span>
            <span className="cs-heatmap-cost">{c.toFixed(0)}</span>
          </div>
        )
      })}
      <div className="cs-heatmap-legend">
        <span style={{ color: "#3b82f6" }}>▌</span> cuckoo &nbsp;
        <span style={{ color: "#d97706" }}>▌</span> host
      </div>
    </div>
  )
}

// ---------------------------------------------------------------
// Tour SVG with edge-diff overlay
// ---------------------------------------------------------------
interface TourSVGProps {
  tour: number[]
  best: number[]
  diff: EdgeDiff | null
}

function polylinePoints(tour: number[], close = true): string {
  const pts = tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ")
  return close && tour.length ? pts + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}` : pts
}

function TourSVG({ tour, best, diff }: TourSVGProps) {
  return (
    <svg viewBox="0 0 300 300" className="cs-canvas" role="img" aria-label="Cuckoo search tour">
      <rect x={0} y={0} width={300} height={300} className="cs-bg" />

      {/* Best-tour underlay (faint green dashed) */}
      {best.length > 0 && (
        <polyline className="cs-best" points={polylinePoints(best)} />
      )}

      {/* Removed edges — orange dashed */}
      {diff?.removed.map(([a, b], i) => (
        <line
          key={i}
          x1={CITIES[a][0]} y1={CITIES[a][1]}
          x2={CITIES[b][0]} y2={CITIES[b][1]}
          className="cs-removed"
        />
      ))}

      {/* Current tour — blue */}
      <polyline className="cs-tour" points={polylinePoints(tour)} />

      {/* Added edges — green, drawn on top of tour */}
      {diff?.added.map(([a, b], i) => (
        <line
          key={i}
          x1={CITIES[a][0]} y1={CITIES[a][1]}
          x2={CITIES[b][0]} y2={CITIES[b][1]}
          className="cs-added"
        />
      ))}

      {/* City dots */}
      {CITIES.map(([x, y], i) => {
        const changed = diff?.changedCities.has(i)
        return changed ? (
          <g key={i}>
            <circle cx={x} cy={y} r={8} className="cs-city-ring" />
            <circle cx={x} cy={y} r={4.5} className="cs-city-active" />
          </g>
        ) : (
          <circle key={i} cx={x} cy={y} r={4} className="cs-city" />
        )
      })}

      {/* City labels — rendered last so they sit above edges */}
      {CITIES.map(([x, y], i) => (
        <text key={i} x={x + 6} y={y - 5} className="cs-city-label">{i}</text>
      ))}
    </svg>
  )
}

// ---------------------------------------------------------------
// Lévy sparkline — colored by step outcome (mirrors FPA pattern)
// ---------------------------------------------------------------
type SparkEntry = { value: number; mode: "levy-hit" | "levy-miss" | "abandon" }

function LevySparkline({ history }: { history: SparkEntry[] }) {
  const W = 180, H = 54
  const visible = history.slice(-30)
  if (!visible.length) return <svg viewBox={`0 0 ${W} ${H}`} className="cs-spark" />
  const maxVal = Math.min(Math.max(...visible.map(h => h.value), 0.5), 4)
  const slotW = W / 30
  const barW = Math.max(1, slotW - 1)
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="cs-spark">
      {visible.map((h, i) => {
        const bh = Math.max(2, (Math.min(h.value, maxVal) / maxVal) * (H - 4))
        const fill =
          h.mode === "levy-hit" ? "#3b82f6" :
          h.mode === "abandon" ? "#d97706" : "#94a3b8"
        return (
          <rect
            key={i}
            x={i * slotW}
            y={H - bh - 2}
            width={barW}
            height={bh}
            fill={fill}
            opacity={0.85}
          />
        )
      })}
    </svg>
  )
}

// ---------------------------------------------------------------
// Main component
// ---------------------------------------------------------------
type Mode = "levy-hit" | "levy-miss" | "abandon" | null

export default function CSExplainer() {
  // Config
  const [pa, setPa] = useState(0.25)
  const [nNests, setNNests] = useState(8)
  const [speed, setSpeed] = useState(5)

  // Refs for stable step callback
  const paRef = useRef(0.25)
  const simRef = useRef<SimState>(makeInitState(8))

  // Display state
  const [tour, setTour] = useState<number[]>(() => simRef.current.nests[0])
  const [best, setBest] = useState<number[]>(() => simRef.current.best)
  const [bestCost, setBestCost] = useState(() => simRef.current.bestCost)
  const [costs, setCosts] = useState<number[]>(() => simRef.current.costs)
  const [epoch, setEpoch] = useState(0)
  const [step, setStep] = useState(0)
  const [replacements, setReplacements] = useState(0)
  const [abandonments, setAbandonments] = useState(0)

  // Per-step annotation
  const [mode, setMode] = useState<Mode>(null)
  const [activeIdx, setActiveIdx] = useState(0)
  const [targetIdx, setTargetIdx] = useState(-1)
  const [abandonedIdxs, setAbandonedIdxs] = useState<number[]>([])
  const [diff, setDiff] = useState<EdgeDiff | null>(null)
  const [lastLevy, setLastLevy] = useState<number | null>(null)
  const [lastK, setLastK] = useState<number | null>(null)
  const [levyHistory, setLevyHistory] = useState<SparkEntry[]>([])

  const [running, setRunning] = useState(false)

  const reinit = useCallback((n: number) => {
    const s = makeInitState(n)
    simRef.current = s
    setTour(s.nests[0].slice())
    setBest(s.best.slice())
    setBestCost(s.bestCost)
    setCosts(s.costs.slice())
    setEpoch(0)
    setStep(0)
    setReplacements(0)
    setAbandonments(0)
    setMode(null)
    setActiveIdx(0)
    setTargetIdx(-1)
    setAbandonedIdxs([])
    setDiff(null)
    setLastLevy(null)
    setLastK(null)
    setLevyHistory([] as SparkEntry[])
    setRunning(false)
  }, [])

  const stepOnce = useCallback(() => {
    const sim = simRef.current
    const currentPa = paRef.current
    const n = sim.nests.length

    // --- Cuckoo event for sim.nestIdx ---
    const ci = sim.nestIdx
    const levy = levyStep()
    const k = Math.max(1, Math.min(
      Math.ceil(levy * N_CITIES * 0.1),
      Math.floor(N_CITIES / 2)
    ))

    const beforeTour = sim.nests[ci].slice()
    const newTour = applyKRandom2Opt(beforeTour, k)
    const newCost = tourLength(newTour)

    // Pick random target ≠ ci
    let ti: number
    do { ti = Math.floor(Math.random() * n) } while (ti === ci && n > 1)

    const hit = newCost < sim.costs[ti]
    const newNests = sim.nests.slice()
    const newCosts = sim.costs.slice()

    if (hit) {
      newNests[ti] = newTour
      newCosts[ti] = newCost
    }

    let newBest = sim.best
    let newBestCost = sim.bestCost
    if (hit && newCost < sim.bestCost) {
      newBest = newTour.slice()
      newBestCost = newCost
    }

    const newReplacements = sim.replacements + (hit ? 1 : 0)
    let nextNestIdx = ci + 1

    // --- End-of-epoch abandonment ---
    let abandonedList: number[] = []
    let newAbandonments = sim.abandonments
    let newEpoch = sim.epoch

    if (nextNestIdx >= n) {
      for (let idx = 0; idx < n; idx++) {
        if (Math.random() < currentPa) {
          const fresh = shuffle(N_CITIES)
          newNests[idx] = fresh
          newCosts[idx] = tourLength(fresh)
          abandonedList.push(idx)
          // Check if fresh tour beats best (rare but possible)
          if (newCosts[idx] < newBestCost) {
            newBest = newNests[idx].slice()
            newBestCost = newCosts[idx]
          }
        }
      }
      newAbandonments = sim.abandonments + abandonedList.length
      newEpoch = sim.epoch + 1
      nextNestIdx = 0
    }

    simRef.current = {
      nests: newNests,
      costs: newCosts,
      best: newBest,
      bestCost: newBestCost,
      nestIdx: nextNestIdx,
      epoch: newEpoch,
      step: sim.step + 1,
      replacements: newReplacements,
      abandonments: newAbandonments,
    }

    // Update display
    const displayTour = hit ? newTour : beforeTour
    setTour(displayTour)
    setBest(newBest)
    setBestCost(newBestCost)
    setCosts(newCosts.slice())
    setEpoch(newEpoch)
    setStep(sim.step + 1)
    setReplacements(newReplacements)
    setAbandonments(newAbandonments)
    setActiveIdx(ci)
    setTargetIdx(ti)
    setAbandonedIdxs(abandonedList)
    setLastLevy(levy)
    setLastK(k)
    const stepMode: "levy-hit" | "levy-miss" | "abandon" =
      abandonedList.length > 0 ? "abandon" : hit ? "levy-hit" : "levy-miss"
    setLevyHistory(h => [...h.slice(-99), { value: levy, mode: stepMode }])
    setDiff(computeEdgeDiff(beforeTour, newTour))

    if (abandonedList.length > 0) {
      setMode("abandon")
    } else if (hit) {
      setMode("levy-hit")
    } else {
      setMode("levy-miss")
    }
  }, [])

  const delay = Math.max(50, 660 - speed * 66)
  useEffect(() => {
    if (!running) return
    const id = setInterval(stepOnce, delay)
    return () => clearInterval(id)
  }, [running, stepOnce, delay])

  // Mode chip content
  const modeLabel = (() => {
    if (!mode) return "Press Step or Run to begin"
    if (mode === "abandon") {
      const n = abandonedIdxs.length
      return `💨 Epoch ${epoch}: ${n} nest${n !== 1 ? "s" : ""} abandoned and re-seeded`
    }
    if (mode === "levy-hit") return `🐦 k=${lastK} reversal${lastK !== 1 ? "s" : ""} → beat host #${targetIdx}`
    return `❌ k=${lastK} reversal${lastK !== 1 ? "s" : ""} → host #${targetIdx} not beaten`
  })()

  const modeClass =
    mode === "levy-hit" ? "cs-mode-hit" :
    mode === "levy-miss" ? "cs-mode-miss" :
    mode === "abandon" ? "cs-mode-abandon" : "cs-mode-idle"

  return (
    <div className="cs-root">
      <style>{CSS}</style>

      <header className="cs-header">
        <div className="cs-eyebrow">teeline · algorithms/cs</div>
        <h2 className="cs-title">Cuckoo Search</h2>
        <p className="cs-sub">
          Each step a cuckoo applies <strong>k random 2-opt reversals</strong> (k drawn from a
          Lévy distribution) and competes with a random host nest — winner keeps the slot.
          At epoch end, each nest is independently{" "}
          <strong>abandoned with probability <code>pa</code></strong> and re-seeded to
          maintain diversity.
        </p>
      </header>

      {/* Tour canvas + nest heatmap side by side */}
      <div className="cs-viz-row">
        <TourSVG tour={tour} best={best} diff={diff} />
        <NestHeatmap
          costs={costs}
          activeIdx={activeIdx}
          targetIdx={targetIdx}
          abandonedIdxs={abandonedIdxs}
        />
      </div>

      {/* Canvas legend — directly under the visualization */}
      <div className="cs-canvas-legend">
        <span><span className="cs-swatch cs-swatch-tour">—</span> current tour</span>
        <span><span className="cs-swatch cs-swatch-best">- -</span> best tour</span>
        <span><span className="cs-swatch cs-swatch-added">—</span> added edges</span>
        <span><span className="cs-swatch cs-swatch-removed">- -</span> removed edges</span>
        <span><span className="cs-ring-demo">◎</span> reversal endpoint</span>
      </div>

      {/* Event chip */}
      <div className={`cs-mode ${modeClass}`}>{modeLabel}</div>

      {/* Lévy sparkline + k */}
      <div className="cs-spark-row">
        <div className="cs-spark-wrap">
          <LevySparkline history={levyHistory} />
          <div className="cs-spark-caption">
            Lévy step (last 30) &nbsp;
            <span style={{ color: "#3b82f6" }}>■</span> hit &nbsp;
            <span style={{ color: "#94a3b8" }}>■</span> miss &nbsp;
            <span style={{ color: "#d97706" }}>■</span> abandon
          </div>
        </div>
        <div className="cs-stepval-wrap">
          <div className="cs-statlabel">levy draw</div>
          <div className="cs-mono cs-stepval">{lastLevy !== null ? lastLevy.toFixed(3) : "—"}</div>
          <div className="cs-statlabel" style={{ marginTop: "6px" }}>k reversals</div>
          <div className="cs-mono cs-kval">{lastK !== null ? lastK : "—"}</div>
        </div>
      </div>

      {/* Stats */}
      <div className="cs-statgrid">
        <div>
          <div className="cs-statlabel">epoch</div>
          <div className="cs-mono">{epoch}</div>
        </div>
        <div>
          <div className="cs-statlabel">step</div>
          <div className="cs-mono">{step}</div>
        </div>
        <div>
          <div className="cs-statlabel">best distance</div>
          <div className="cs-mono">{bestCost.toFixed(1)}</div>
        </div>
        <div>
          <div className="cs-statlabel">replacements</div>
          <div className="cs-mono cs-hit-val">{replacements}</div>
        </div>
        <div>
          <div className="cs-statlabel">abandonments</div>
          <div className="cs-mono cs-abandon-val">{abandonments}</div>
        </div>
      </div>

      {/* Config sliders */}
      <div className="cs-config">
        <div className="cs-config-row">
          <label className="cs-config-label">
            Abandon prob <code>pa</code> = <strong>{pa.toFixed(2)}</strong>
          </label>
          <input
            type="range" min={0} max={0.5} step={0.05}
            value={pa} className="cs-slider"
            onInput={(e) => {
              const v = Number((e.target as HTMLInputElement).value)
              setPa(v)
              paRef.current = v
            }}
          />
          <div className="cs-hint">
            {pa === 0
              ? "No abandonment — population never diversifies"
              : pa >= 0.4
                ? "High abandonment — lots of re-seeding, slower convergence"
                : `~${Math.round(pa * 100)}% of nests replaced each epoch`}
          </div>
        </div>
        <div className="cs-config-row">
          <label className="cs-config-label">
            Nests (population) = <strong>{nNests}</strong>
          </label>
          <input
            type="range" min={3} max={15} step={1}
            value={nNests} className="cs-slider"
            onInput={(e) => {
              const n = Number((e.target as HTMLInputElement).value)
              setNNests(n)
              reinit(n)
            }}
          />
        </div>
        <div className="cs-config-row">
          <label className="cs-config-label">Speed</label>
          <input
            type="range" min={1} max={10} step={1}
            value={speed} className="cs-slider"
            onInput={(e) => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
        </div>
      </div>

      {/* Controls */}
      <div className="cs-controls">
        <button className="cs-btn" onClick={stepOnce} disabled={running}>◀ Step</button>
        <button
          className={`cs-btn ${!running ? "cs-btn-primary" : ""}`}
          onClick={() => setRunning(r => !r)}
        >
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="cs-btn" onClick={() => reinit(nNests)}>↺ Reset</button>
      </div>

      <footer className="cs-footer">
        <span className="cs-mono">cities: {N_CITIES}</span>
        <span className="cs-mono">nests: {nNests}</span>
        <span className="cs-mono">pa: {pa.toFixed(2)}</span>
        <span className="cs-mono">β (Lévy): 1.5</span>
      </footer>
    </div>
  )
}

// ---------------------------------------------------------------
// Styles — self-contained, scoped via cs- prefix
// ---------------------------------------------------------------
const CSS = `
.cs-root {
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --blue: #3b82f6;
  --green: #16a34a;
  --orange: #d97706;
  --red: #dc2626;
  --city: #f2a154;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 20px;
  max-width: 760px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.cs-root * { box-sizing: border-box; }
.cs-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

/* Header */
.cs-header { margin-bottom: 2px; }
.cs-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--blue); margin-bottom: 6px;
}
.cs-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.cs-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.cs-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(59,130,246,0.08);
  padding: 1px 4px; border-radius: 4px;
}

/* Viz row: canvas fills space, heatmap sits beside it */
.cs-viz-row { display: flex; gap: 10px; align-items: stretch; }

/* Tour canvas */
.cs-canvas {
  flex: 1; min-width: 0; display: block;
  border-radius: 8px; border: 1px solid var(--line);
}
.cs-bg { fill: var(--panel); }
.cs-best {
  fill: none; stroke: var(--green); stroke-width: 1;
  stroke-dasharray: 5 4; stroke-opacity: 0.28; stroke-linejoin: round;
}
.cs-removed {
  stroke: #ea8c00; stroke-width: 2; stroke-dasharray: 4 3; stroke-opacity: 0.85;
}
.cs-tour {
  fill: none; stroke: var(--blue); stroke-width: 1.8; stroke-linejoin: round;
}
.cs-added {
  stroke: var(--green); stroke-width: 2.5; stroke-opacity: 0.9;
}
.cs-city { fill: var(--city); stroke: var(--bg); stroke-width: 1.5; }
.cs-city-active { fill: #f97316; stroke: var(--bg); stroke-width: 1.5; }
.cs-city-ring { fill: none; stroke: #f97316; stroke-width: 1.5; stroke-opacity: 0.45; }
.cs-city-label {
  font-size: 8px; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  fill: #374151; stroke: white; stroke-width: 2.5; paint-order: stroke fill;
  dominant-baseline: auto; pointer-events: none; user-select: none;
}

/* Nest heatmap */
.cs-heatmap {
  width: 100px; flex-shrink: 0;
  display: flex; flex-direction: column; gap: 3px;
}
.cs-heatmap-title {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--muted); margin-bottom: 2px;
}
.cs-heatmap-cell {
  flex: 1; border-radius: 5px; min-height: 20px;
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 7px;
  transition: background 0.35s ease;
}
.cs-heatmap-idx {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.72rem; font-weight: 700; color: rgba(255,255,255,0.9);
}
.cs-heatmap-cost {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.68rem; color: rgba(255,255,255,0.7);
}
.cs-heatmap-legend {
  font-size: 0.7rem; color: var(--muted); margin-top: 4px; line-height: 1.4;
}

/* Event chip */
.cs-mode {
  font-size: 0.92rem; font-weight: 600;
  padding: 8px 14px; border-radius: 8px;
  border: 1px solid transparent;
}
.cs-mode-idle { background: var(--panel); color: var(--muted); border-color: var(--line); }
.cs-mode-hit { background: rgba(59,130,246,0.1); color: #1d4ed8; border-color: rgba(59,130,246,0.3); }
.cs-mode-miss { background: var(--panel); color: var(--muted); border-color: var(--line); font-weight: 400; }
.cs-mode-abandon { background: rgba(217,119,6,0.1); color: #92400e; border-color: rgba(217,119,6,0.3); }

/* Sparkline row */
.cs-spark-row { display: flex; gap: 16px; align-items: flex-start; }
.cs-spark-wrap { display: flex; flex-direction: column; gap: 4px; }
.cs-spark {
  width: 180px; height: 54px;
  border: 1px solid var(--line); border-radius: 6px;
  background: var(--panel); display: block;
}
.cs-spark-caption { font-size: 0.72rem; color: var(--muted); }
.cs-stepval-wrap { display: flex; flex-direction: column; }
.cs-stepval { font-size: 1.25rem; font-weight: 600; }
.cs-kval { font-size: 1.1rem; font-weight: 600; color: var(--blue); }
.cs-statlabel {
  font-size: 0.7rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}

/* Stats */
.cs-statgrid {
  display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px;
}
@media (max-width: 520px) {
  .cs-statgrid { grid-template-columns: repeat(3, 1fr); }
}
.cs-hit-val { color: var(--blue); }
.cs-abandon-val { color: var(--orange); }

/* Config */
.cs-config { display: flex; flex-direction: column; gap: 10px; }
.cs-config-row { display: flex; flex-direction: column; gap: 4px; }
.cs-config-label { font-size: 0.88rem; }
.cs-config-label code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.9em;
}
.cs-slider { width: 100%; accent-color: var(--blue); cursor: pointer; }
.cs-hint { font-size: 0.78rem; color: var(--muted); }

/* Controls */
.cs-controls { display: flex; gap: 8px; }
.cs-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer;
  font-family: inherit;
}
.cs-btn:hover:not(:disabled) { border-color: var(--blue); }
.cs-btn:disabled { opacity: 0.45; cursor: default; }
.cs-btn-primary { color: var(--blue); border-color: var(--blue); }

/* Canvas legend */
.cs-canvas-legend {
  display: flex; gap: 12px; flex-wrap: wrap;
  font-size: 0.75rem; color: var(--muted);
}
.cs-swatch { font-family: ui-monospace, Consolas, monospace; font-size: 0.85em; }
.cs-swatch-tour { color: var(--blue); }
.cs-swatch-best { color: var(--green); }
.cs-swatch-added { color: var(--green); font-weight: 700; }
.cs-swatch-removed { color: #ea8c00; }
.cs-ring-demo { font-size: 0.9em; color: #f97316; }

/* Footer */
.cs-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 10px; border-top: 1px solid var(--line);
  color: var(--muted);
}
`

import { useState, useRef, useEffect, useCallback } from "preact/hooks"

// ---------------------------------------------------------------
// Fixed 12-city demo (same layout as CS / FPA explainers)
// ---------------------------------------------------------------
const CITIES: [number, number][] = [
  [45, 45], [155, 18], [265, 45], [285, 150],
  [255, 265], [150, 285], [40, 260], [18, 150],
  [110, 115], [200, 95], [220, 210], [95, 215],
]
const N_CITIES = CITIES.length
const T_MIN = 0.5

// ---------------------------------------------------------------
// Algorithm helpers
// ---------------------------------------------------------------
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

// Random 2-opt segment reversal — matches route.rs random_successor
function twoOptSwap(tour: number[]): { tour: number[]; i: number; j: number } {
  const n = tour.length
  let i = Math.floor(Math.random() * (n - 2))
  let j = i + 2 + Math.floor(Math.random() * (n - 2 - i))
  if (j >= n) j = n - 1
  const next = tour.slice()
  // reverse segment [i..j]
  let lo = i, hi = j
  while (lo < hi) {
    ;[next[lo], next[hi]] = [next[hi], next[lo]]
    lo++; hi--
  }
  return { tour: next, i, j }
}

// Edge diff for highlighting changed edges
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
  removed: [number, number][]
  added: [number, number][]
  changedCities: Set<number>
}
function computeEdgeDiff(before: number[], after: number[]): EdgeDiff {
  const bSet = tourEdgeSet(before)
  const aSet = tourEdgeSet(after)
  const removed: [number, number][] = []
  const added: [number, number][] = []
  const changedCities = new Set<number>()
  for (let i = 0; i < before.length; i++) {
    const a = before[i], b = before[(i + 1) % before.length]
    if (!aSet.has(edgeKey(a, b))) { removed.push([a, b]); changedCities.add(a); changedCities.add(b) }
  }
  for (let i = 0; i < after.length; i++) {
    const a = after[i], b = after[(i + 1) % after.length]
    if (!bSet.has(edgeKey(a, b))) { added.push([a, b]); changedCities.add(a); changedCities.add(b) }
  }
  return { removed, added, changedCities }
}

// ---------------------------------------------------------------
// Simulation state
// ---------------------------------------------------------------
type MoveMode = "improvement" | "accepted" | "rejected" | "converged"

type SimState = {
  tour: number[]
  best: number[]
  bestCost: number
  currentCost: number
  temperature: number
  iter: number
  accepted: number
  rejected: number
  recentOutcomes: boolean[]  // last 100 for acceptance rate
}

function makeInitState(initTemp: number): SimState {
  const tour = shuffle(N_CITIES)
  const cost = tourLength(tour)
  return {
    tour, best: tour.slice(), bestCost: cost, currentCost: cost,
    temperature: initTemp, iter: 0, accepted: 0, rejected: 0,
    recentOutcomes: [],
  }
}

// ---------------------------------------------------------------
// SVG helpers
// ---------------------------------------------------------------
function polyPts(tour: number[]): string {
  const pts = tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ")
  return tour.length > 0 ? pts + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}` : pts
}

// ---------------------------------------------------------------
// TourSVG
// ---------------------------------------------------------------
interface TourSVGProps {
  tour: number[]
  best: number[]
  diff: EdgeDiff | null
  converged: boolean
}
function TourSVG({ tour, best, diff, converged }: TourSVGProps) {
  return (
    <svg viewBox="0 0 300 300" className="sa-canvas" role="img" aria-label="SA tour">
      <rect x={0} y={0} width={300} height={300} className="sa-bg" />
      {/* best tour underlay */}
      <polyline
        points={polyPts(best)}
        className={converged ? "sa-best-converged" : "sa-best"}
      />
      {/* removed edges from proposed swap */}
      {diff && diff.removed.map(([a, b], i) => (
        <line key={i}
          x1={CITIES[a][0]} y1={CITIES[a][1]}
          x2={CITIES[b][0]} y2={CITIES[b][1]}
          className="sa-edge-removed"
        />
      ))}
      {/* current tour */}
      <polyline points={polyPts(tour)} className="sa-tour" />
      {/* added edges from proposed swap */}
      {diff && diff.added.map(([a, b], i) => (
        <line key={i}
          x1={CITIES[a][0]} y1={CITIES[a][1]}
          x2={CITIES[b][0]} y2={CITIES[b][1]}
          className="sa-edge-added"
        />
      ))}
      {/* city dots */}
      {CITIES.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y}
          r={diff && diff.changedCities.has(i) ? 7 : 4}
          className={diff && diff.changedCities.has(i) ? "sa-city sa-city-changed" : "sa-city"}
        />
      ))}
      {/* city labels */}
      {CITIES.map(([x, y], i) => (
        <text key={i} x={x + 6} y={y - 5} className="sa-city-label">{i}</text>
      ))}
    </svg>
  )
}

// ---------------------------------------------------------------
// TDecayCurve — shows theoretical cooling arc + actual T history
// ---------------------------------------------------------------
interface TDecayCurveProps {
  tHistory: number[]
  initTemp: number
  alpha: number
}
function TDecayCurve({ tHistory, initTemp, alpha }: TDecayCurveProps) {
  const W = 200, H = 54
  // expected total steps to reach T_MIN
  const totalSteps = Math.ceil(Math.log(T_MIN / initTemp) / Math.log(1 - alpha))
  const xMax = Math.max(totalSteps, tHistory.length, 1)

  // theoretical arc points (sample 60 points)
  const arcPts: string[] = []
  for (let k = 0; k <= 60; k++) {
    const t = (k / 60) * totalSteps
    const temp = initTemp * Math.pow(1 - alpha, t)
    const x = (t / xMax) * W
    const y = H - 4 - ((Math.max(temp - T_MIN, 0)) / (initTemp - T_MIN)) * (H - 8)
    arcPts.push(`${x.toFixed(1)},${y.toFixed(1)}`)
  }

  // actual T history polyline with colour segments (warm→cool gradient approximation)
  const histPts = tHistory.map((temp, idx) => {
    const x = (idx / xMax) * W
    const y = H - 4 - ((Math.max(temp - T_MIN, 0)) / (initTemp - T_MIN)) * (H - 8)
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })

  // current position dot
  const curIdx = tHistory.length - 1
  const curTemp = curIdx >= 0 ? tHistory[curIdx] : initTemp
  const dotX = curIdx >= 0 ? (curIdx / xMax) * W : 0
  const dotY = H - 4 - ((Math.max(curTemp - T_MIN, 0)) / (initTemp - T_MIN)) * (H - 8)
  // colour of dot: red when hot, blue when cool
  const norm = Math.max(curTemp - T_MIN, 0) / (initTemp - T_MIN)
  const r = Math.round(norm * 220)
  const b = Math.round((1 - norm) * 210 + 45)
  const dotColor = `rgb(${r},${Math.round(60 + (1 - norm) * 80)},${b})`

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="sa-tdecay">
      {/* theoretical arc */}
      <polyline points={arcPts.join(" ")} fill="none" stroke="#d0d7de" strokeWidth={1} />
      {/* actual history */}
      {histPts.length > 1 && (
        <polyline points={histPts.join(" ")} fill="none" stroke="#d97706" strokeWidth={1.5} opacity={0.7} />
      )}
      {/* current position dot */}
      {curIdx >= 0 && (
        <circle cx={dotX} cy={dotY} r={4} fill={dotColor} />
      )}
    </svg>
  )
}

// ---------------------------------------------------------------
// AcceptanceSparkline — last 30 steps coloured by outcome
// ---------------------------------------------------------------
type SparkEntry = { prob: number; mode: MoveMode }

function AcceptanceSparkline({ history }: { history: SparkEntry[] }) {
  const W = 180, H = 54
  const visible = history.slice(-30)
  if (!visible.length) return <svg viewBox={`0 0 ${W} ${H}`} className="sa-spark" />
  const slotW = W / 30
  const barW = Math.max(1, slotW - 1)
  const modeColor: Record<string, string> = {
    improvement: "#16a34a",
    accepted: "#d97706",
    rejected: "#dc2626",
    converged: "#0969da",
  }
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="sa-spark">
      {visible.map((h, i) => {
        const bh = Math.max(2, h.prob * (H - 4))
        return (
          <rect key={i}
            x={i * slotW} y={H - bh - 2}
            width={barW} height={bh}
            fill={modeColor[h.mode] ?? "#9ca3af"}
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
export default function SAExplainer() {
  const [initTemp, setInitTemp] = useState(150)
  const [alpha, setAlpha] = useState(0.03)
  const [speed, setSpeed] = useState(5)

  const initTempRef = useRef(150)
  const alphaRef = useRef(0.03)
  const simRef = useRef<SimState>(makeInitState(150))

  const [tour, setTour] = useState<number[]>(() => simRef.current.tour)
  const [best, setBest] = useState<number[]>(() => simRef.current.best)
  const [bestCost, setBestCost] = useState(() => simRef.current.bestCost)
  const [currentCost, setCurrentCost] = useState(() => simRef.current.currentCost)
  const [temperature, setTemperature] = useState(() => simRef.current.temperature)
  const [iter, setIter] = useState(0)

  const [mode, setMode] = useState<MoveMode | null>(null)
  const [lastDelta, setLastDelta] = useState<number | null>(null)
  const [lastProb, setLastProb] = useState<number | null>(null)
  const [diff, setDiff] = useState<EdgeDiff | null>(null)
  const [sparkHistory, setSparkHistory] = useState<SparkEntry[]>([])
  const [tHistory, setTHistory] = useState<number[]>([])

  const [running, setRunning] = useState(false)

  const reinit = useCallback((temp: number) => {
    const s = makeInitState(temp)
    simRef.current = s
    setTour(s.tour.slice())
    setBest(s.best.slice())
    setBestCost(s.bestCost)
    setCurrentCost(s.currentCost)
    setTemperature(s.temperature)
    setIter(0)
    setMode(null)
    setLastDelta(null)
    setLastProb(null)
    setDiff(null)
    setSparkHistory([])
    setTHistory([])
    setRunning(false)
  }, [])

  const stepOnce = useCallback(() => {
    const s = simRef.current
    if (s.temperature < T_MIN) {
      setMode("converged")
      setRunning(false)
      return
    }

    const a = alphaRef.current
    const before = s.tour.slice()
    const { tour: proposed } = twoOptSwap(s.tour)
    const newCost = tourLength(proposed)
    const delta = newCost - s.currentCost

    let accepted = false
    let prob: number
    let stepMode: MoveMode

    if (delta < 0) {
      accepted = true
      prob = 1.0
      stepMode = "improvement"
    } else {
      prob = Math.exp(-delta / s.temperature)
      accepted = Math.random() < prob
      stepMode = accepted ? "accepted" : "rejected"
    }

    const nextTour = accepted ? proposed : s.tour
    const nextCost = accepted ? newCost : s.currentCost
    const isBetter = nextCost < s.bestCost
    const nextBest = isBetter ? nextTour.slice() : s.best
    const nextBestCost = isBetter ? nextCost : s.bestCost

    const nextOutcomes = [...s.recentOutcomes.slice(-99), accepted]
    const nextTemp = s.temperature * (1 - a)

    simRef.current = {
      ...s,
      tour: nextTour,
      best: nextBest,
      bestCost: nextBestCost,
      currentCost: nextCost,
      temperature: nextTemp,
      iter: s.iter + 1,
      accepted: s.accepted + (accepted ? 1 : 0),
      rejected: s.rejected + (accepted ? 0 : 1),
      recentOutcomes: nextOutcomes,
    }

    const edgeDiff = computeEdgeDiff(before, nextTour)

    setTour(nextTour.slice())
    if (isBetter) setBest(nextBest.slice())
    setBestCost(nextBestCost)
    setCurrentCost(nextCost)
    setTemperature(nextTemp)
    setIter(simRef.current.iter)
    setMode(nextTemp < T_MIN ? "converged" : stepMode)
    setLastDelta(delta)
    setLastProb(prob)
    setDiff(edgeDiff)
    setSparkHistory(h => [...h.slice(-29), { prob, mode: stepMode }])
    setTHistory(h => [...h, nextTemp])

    if (nextTemp < T_MIN) {
      setRunning(false)
    }
  }, [])

  // running loop
  useEffect(() => {
    if (!running) return
    const ms = Math.max(50, 660 - speed * 66)
    const id = setInterval(stepOnce, ms)
    return () => clearInterval(id)
  }, [running, speed, stepOnce])

  const acceptanceRate = simRef.current.recentOutcomes.length > 0
    ? (simRef.current.recentOutcomes.filter(Boolean).length / simRef.current.recentOutcomes.length * 100).toFixed(0)
    : "—"

  const converged = mode === "converged"

  // event chip
  let chipText = ""
  let chipClass = "sa-chip"
  if (mode === "improvement") {
    chipText = `✅ improved  ΔE = ${lastDelta !== null ? lastDelta.toFixed(1) : "—"}`
    chipClass = "sa-chip sa-chip-improve"
  } else if (mode === "accepted") {
    chipText = `⚠️ worsening accepted  p = ${lastProb !== null ? lastProb.toFixed(2) : "—"}  ΔE = +${lastDelta !== null ? lastDelta.toFixed(1) : "—"}`
    chipClass = "sa-chip sa-chip-accepted"
  } else if (mode === "rejected") {
    chipText = `❌ rejected  p = ${lastProb !== null ? lastProb.toFixed(2) : "—"}  ΔE = +${lastDelta !== null ? lastDelta.toFixed(1) : "—"}`
    chipClass = "sa-chip sa-chip-rejected"
  } else if (mode === "converged") {
    chipText = "🏁 Converged — best tour highlighted"
    chipClass = "sa-chip sa-chip-converged"
  }

  return (
    <div className="sa-explainer">
      <style>{CSS}</style>

      <div className="sa-header">
        <div className="sa-eyebrow">teeline · algorithms/sa</div>
        <h2 className="sa-title">Simulated Annealing</h2>
        <p className="sa-sub">
          A single tour evolves by random 2-opt swaps. Improvements always accepted;
          worsenings accepted with probability <code>exp(−ΔE / T)</code>.
          As temperature <code>T</code> cools, acceptance of bad moves drops — shifting from exploration to exploitation.
        </p>
      </div>

      <TourSVG tour={tour} best={best} diff={diff} converged={converged} />

      <div className="sa-legend">
        <span className="sa-swatch sa-swatch-tour" /> current tour
        <span className="sa-swatch sa-swatch-best" /> best tour
        <span className="sa-swatch sa-swatch-removed" /> removed edge
        <span className="sa-swatch sa-swatch-added" /> added edge
        <span className="sa-swatch sa-swatch-city" /> city
      </div>

      <div className="sa-curves-row">
        <div className="sa-curve-wrap">
          <div className="sa-curve-label">temperature decay</div>
          <TDecayCurve tHistory={tHistory} initTemp={initTemp} alpha={alpha} />
        </div>
        <div className="sa-spark-wrap">
          <div className="sa-curve-label">acceptance probability (last 30)</div>
          <AcceptanceSparkline history={sparkHistory} />
        </div>
      </div>

      {mode !== null && <div className={chipClass}>{chipText}</div>}

      <div className="sa-stats">
        <div className="sa-stat"><span className="sa-stat-val">{iter}</span><span className="sa-stat-label">iter</span></div>
        <div className="sa-stat"><span className="sa-stat-val">{temperature.toFixed(2)}</span><span className="sa-stat-label">T</span></div>
        <div className="sa-stat"><span className="sa-stat-val">{currentCost.toFixed(0)}</span><span className="sa-stat-label">current</span></div>
        <div className="sa-stat"><span className="sa-stat-val">{bestCost.toFixed(0)}</span><span className="sa-stat-label">best</span></div>
        <div className="sa-stat"><span className="sa-stat-val">{acceptanceRate}%</span><span className="sa-stat-label">acc rate</span></div>
      </div>

      <div className="sa-config">
        <label className="sa-label">
          T₀ = {initTemp}
          <input type="range" min={10} max={500} step={10} value={initTemp}
            className="sa-slider"
            onInput={e => {
              const v = Number((e.target as HTMLInputElement).value)
              setInitTemp(v)
              initTempRef.current = v
              reinit(v)
            }}
          />
        </label>
        <label className="sa-label">
          α = {alpha.toFixed(3)}
          <input type="range" min={0.005} max={0.10} step={0.005} value={alpha}
            className="sa-slider"
            onInput={e => {
              const v = Number((e.target as HTMLInputElement).value)
              setAlpha(v)
              alphaRef.current = v
              reinit(initTempRef.current)
            }}
          />
        </label>
        <label className="sa-label">
          speed = {speed}
          <input type="range" min={1} max={10} step={1} value={speed}
            className="sa-slider"
            onInput={e => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
        </label>
      </div>

      <div className="sa-controls">
        <button className="sa-btn"
          onClick={stepOnce}
          disabled={running || converged}
        >Step</button>
        <button className="sa-btn sa-btn-primary"
          onClick={() => setRunning(r => !r)}
          disabled={converged}
        >{running ? "Pause" : "Run"}</button>
        <button className="sa-btn"
          onClick={() => reinit(initTempRef.current)}
        >Reset</button>
      </div>

      <div className="sa-footer">
        {N_CITIES} cities · T₀ = {initTemp} · α = {alpha.toFixed(3)} · T_min = {T_MIN}
      </div>
    </div>
  )
}

// ---------------------------------------------------------------
// Styles
// ---------------------------------------------------------------
const CSS = `
.sa-explainer {
  --accent: #0969da;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --improve: #16a34a;
  --warm: #d97706;
  --reject: #dc2626;
  --tour: #3b82f6;
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
.sa-header { margin-bottom: 12px; }
.sa-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.sa-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.sa-sub { margin: 0 0 14px; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.sa-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(9,105,218,0.08);
  color: var(--accent); padding: 1px 4px; border-radius: 4px;
}

/* Canvas */
.sa-canvas {
  width: 100%; height: auto; display: block;
  border: 1px solid var(--line); border-radius: 8px; background: var(--bg);
  margin-bottom: 10px;
}
.sa-bg { fill: var(--panel); }
.sa-best { fill: none; stroke: var(--improve); stroke-width: 1.5; stroke-dasharray: 5 4; stroke-linejoin: round; opacity: 0.35; }
.sa-best-converged { fill: none; stroke: var(--improve); stroke-width: 3; stroke-linejoin: round; opacity: 0.85; }
.sa-tour { fill: none; stroke: var(--tour); stroke-width: 2; stroke-linejoin: round; }
.sa-edge-removed { stroke: var(--warm); stroke-width: 2; stroke-dasharray: 4 3; }
.sa-edge-added { stroke: var(--improve); stroke-width: 2.5; }
.sa-city { fill: var(--city); }
.sa-city-changed { stroke: #fff; stroke-width: 2; }
.sa-city-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: var(--text); stroke: #fff; stroke-width: 3;
  paint-order: stroke fill; pointer-events: none; user-select: none;
}

/* Legend */
.sa-legend {
  display: flex; flex-wrap: wrap; gap: 10px; font-size: 0.78rem;
  color: var(--muted); align-items: center; margin-bottom: 12px;
}
.sa-swatch {
  display: inline-block; width: 22px; height: 3px;
  border-radius: 2px; margin-right: 3px; vertical-align: middle;
}
.sa-swatch-tour { background: var(--tour); }
.sa-swatch-best { background: var(--improve); opacity: 0.5; }
.sa-swatch-removed { background: var(--warm); }
.sa-swatch-added { background: var(--improve); }
.sa-swatch-city { background: var(--city); border-radius: 50%; height: 8px; width: 8px; }

/* T-decay + sparkline row */
.sa-curves-row {
  display: flex; gap: 16px; margin-bottom: 10px; align-items: flex-end;
}
.sa-curve-wrap, .sa-spark-wrap { flex: 1; min-width: 0; }
.sa-curve-label {
  font-size: 0.7rem; color: var(--muted); text-transform: uppercase;
  letter-spacing: 0.06em; margin-bottom: 4px;
}
.sa-tdecay, .sa-spark {
  width: 100%; height: 54px; display: block;
  border: 1px solid var(--line); border-radius: 6px; background: var(--panel);
}

/* Event chip */
.sa-chip {
  font-size: 0.82rem; padding: 6px 10px; border-radius: 6px;
  margin-bottom: 10px; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.sa-chip-improve { background: #dcfce7; color: #15803d; }
.sa-chip-accepted { background: #fef3c7; color: #92400e; }
.sa-chip-rejected { background: #fee2e2; color: #991b1b; }
.sa-chip-converged { background: #dbeafe; color: #1e40af; }

/* Stats */
.sa-stats {
  display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 14px;
}
.sa-stat {
  flex: 1; min-width: 60px;
  background: var(--panel); border: 1px solid var(--line); border-radius: 8px;
  padding: 6px 8px; text-align: center;
}
.sa-stat-val { display: block; font-size: 1rem; font-weight: 700; color: var(--text); }
.sa-stat-label { font-size: 0.68rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; }

/* Config sliders */
.sa-config { display: flex; flex-direction: column; gap: 8px; margin-bottom: 14px; }
.sa-label { font-size: 0.82rem; color: var(--muted); display: flex; align-items: center; gap: 8px; }
.sa-slider { flex: 1; accent-color: var(--accent); cursor: pointer; }

/* Controls */
.sa-controls { display: flex; gap: 8px; margin-bottom: 12px; }
.sa-btn {
  padding: 6px 14px; border: 1.5px solid var(--line); border-radius: 6px;
  background: var(--bg); color: var(--text); font-size: 0.85rem; cursor: pointer;
  transition: border-color 0.12s;
}
.sa-btn:hover:not(:disabled) { border-color: var(--accent); }
.sa-btn:disabled { opacity: 0.4; cursor: default; }
.sa-btn-primary { color: var(--accent); border-color: var(--accent); }

/* Footer */
.sa-footer {
  font-size: 0.72rem; color: var(--muted);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  border-top: 1px solid var(--line); padding-top: 8px; margin-top: 4px;
}
`

import { useState, useRef, useEffect, useCallback } from "preact/hooks"

// ---------------------------------------------------------------
// Fixed demo instance: 18 cities on a 300×300 canvas.
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
function randNorm(): number {
  const u1 = Math.max(Math.random(), 1e-12)
  const u2 = Math.random()
  return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2)
}

// Mantegna's algorithm for Lévy step, β = 1.5
// σ precomputed: (Γ(2.5)·sin(3π/4)) / (Γ(1.25)·1.5·2^0.25))^(2/3) ≈ 0.7213
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

// Returns the sequence of adjacent transpositions that transforms a into b.
function swapSequence(a: number[], b: number[]): [number, number][] {
  const arr = a.slice()
  const swaps: [number, number][] = []
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

function applySwaps(tour: number[], swaps: [number, number][], n: number): number[] {
  const t = tour.slice()
  const lim = Math.min(n, swaps.length)
  for (let i = 0; i < lim; i++) {
    const [a, b] = swaps[i]
    ;[t[a], t[b]] = [t[b], t[a]]
  }
  return t
}

function shuffle(n: number): number[] {
  const t = Array.from({ length: n }, (_, i) => i)
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[t[i], t[j]] = [t[j], t[i]]
  }
  return t
}

function centroid(tour: number[]): [number, number] {
  let x = 0, y = 0
  for (const idx of tour) {
    x += CITIES[idx][0]
    y += CITIES[idx][1]
  }
  return [x / tour.length, y / tour.length]
}

// ---------------------------------------------------------------
// Simulation state
// ---------------------------------------------------------------
type SimState = {
  flowers: number[][]
  costs: number[]
  gbest: number[]
  gbestCost: number
  iter: number
  globalCount: number
  localCount: number
}

function makeInitState(nFlowers: number): SimState {
  const flowers = Array.from({ length: nFlowers }, () => shuffle(N_CITIES))
  const costs = flowers.map(tourLength)
  const bestIdx = costs.indexOf(Math.min(...costs))
  return {
    flowers,
    costs,
    gbest: flowers[bestIdx].slice(),
    gbestCost: costs[bestIdx],
    iter: 0,
    globalCount: 0,
    localCount: 0,
  }
}

// ---------------------------------------------------------------
// SVG helper
// ---------------------------------------------------------------
function polyPts(tour: number[]): string {
  const pts = tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ")
  return tour.length > 0 ? pts + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}` : pts
}

// ---------------------------------------------------------------
// TourSVG
// ---------------------------------------------------------------
interface TourSVGProps {
  flowers: number[][]
  gbest: number[]
  activeIdx: number
}

function TourSVG({ flowers, gbest, activeIdx }: TourSVGProps) {
  const ghosts = flowers.filter((_, i) => i !== activeIdx).slice(0, 5)
  const active = flowers[activeIdx]
  return (
    <svg viewBox="0 0 300 300" className="fp-canvas" role="img" aria-label="FPA tour population">
      <rect x={0} y={0} width={300} height={300} className="fp-bg" />
      {ghosts.map((tour, i) => (
        <polyline key={i} className="fp-ghost" points={polyPts(tour)} />
      ))}
      {active && <polyline className="fp-active" points={polyPts(active)} />}
      {gbest.length > 0 && <polyline className="fp-gbest" points={polyPts(gbest)} />}
      {CITIES.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y} r={4} className="fp-city" />
      ))}
      {CITIES.map(([x, y], i) => (
        <text key={i} x={x + 6} y={y - 5} className="fp-city-label">{i}</text>
      ))}
    </svg>
  )
}

// ---------------------------------------------------------------
// Lévy sparkline: bar chart of last 30 step sizes
// ---------------------------------------------------------------
type SparkEntry = { value: number; mode: "global" | "local" }

function LevySparkline({ history }: { history: SparkEntry[] }) {
  const W = 180, H = 54
  const visible = history.slice(-30)
  if (!visible.length) {
    return <svg viewBox={`0 0 ${W} ${H}`} className="fp-spark" />
  }
  const maxVal = Math.min(Math.max(...visible.map(h => h.value), 0.5), 4)
  const slotW = W / 30
  const barW = Math.max(1, slotW - 1)
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="fp-spark">
      {visible.map((h, i) => {
        const bh = Math.max(2, (Math.min(h.value, maxVal) / maxVal) * (H - 4))
        return (
          <rect
            key={i}
            x={i * slotW}
            y={H - bh - 2}
            width={barW}
            height={bh}
            fill={h.mode === "global" ? "#3b82f6" : "#d97706"}
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
export default function FPAExplainer() {
  // Config state (for slider display)
  const [switchProb, setSwitchProb] = useState(0.8)
  const [nFlowers, setNFlowers] = useState(8)
  const [speed, setSpeed] = useState(5)

  // Refs for values read inside the stable step callback
  const switchProbRef = useRef(0.8)
  const simRef = useRef<SimState>(makeInitState(8))

  // Display state — mirrors simRef for rendering
  const [flowers, setFlowers] = useState<number[][]>(() => simRef.current.flowers)
  const [gbest, setGbest] = useState<number[]>(() => simRef.current.gbest)
  const [gbestCost, setGbestCost] = useState(() => simRef.current.gbestCost)
  const [iter, setIter] = useState(0)
  const [globalCount, setGlobalCount] = useState(0)
  const [localCount, setLocalCount] = useState(0)

  // Per-step annotation state
  const [mode, setMode] = useState<"global" | "local" | null>(null)
  const [activeIdx, setActiveIdx] = useState(0)
  const [levyHistory, setLevyHistory] = useState<SparkEntry[]>([])
  const [lastLevy, setLastLevy] = useState<number | null>(null)
  const [iconPos, setIconPos] = useState<{ icon: string; x: number; y: number } | null>(null)

  const [running, setRunning] = useState(false)

  // Reinitialize simulation — stable callback (no deps)
  const reinit = useCallback((n: number) => {
    const s = makeInitState(n)
    simRef.current = s
    setFlowers(s.flowers.slice())
    setGbest(s.gbest.slice())
    setGbestCost(s.gbestCost)
    setIter(0)
    setGlobalCount(0)
    setLocalCount(0)
    setMode(null)
    setActiveIdx(0)
    setLevyHistory([])
    setLastLevy(null)
    setIconPos(null)
    setRunning(false)
  }, [])

  // Advance one pollination event — stable callback, reads via refs
  const stepOnce = useCallback(() => {
    const sim = simRef.current
    const sp = switchProbRef.current
    const n = sim.flowers.length
    if (n === 0) return

    const i = Math.floor(Math.random() * n)
    let newTour: number[]
    let stepMode: "global" | "local"
    let stepLevy: number
    let localJ = -1
    let localK = -1

    if (Math.random() < sp) {
      // Global pollination: Lévy-flight toward gbest
      stepMode = "global"
      const lv = levyStep()
      stepLevy = lv
      const seq = swapSequence(sim.flowers[i], sim.gbest)
      if (seq.length === 0) {
        newTour = sim.flowers[i].slice()
      } else {
        // * 0.5 matches the Rust implementation: never jump more than halfway to gbest
        const nSwaps = Math.min(seq.length, Math.max(1, Math.ceil(lv * seq.length * 0.5)))
        newTour = applySwaps(sim.flowers[i], seq, nSwaps)
      }
    } else {
      // Local pollination: ε-scaled cross-pollination between two random flowers
      stepMode = "local"
      if (n < 3) {
        newTour = sim.flowers[i].slice()
        stepLevy = 0
      } else {
        do { localJ = Math.floor(Math.random() * n) } while (localJ === i)
        do { localK = Math.floor(Math.random() * n) } while (localK === i || localK === localJ)
        const epsilon = Math.random()
        stepLevy = epsilon
        const seq = swapSequence(sim.flowers[localJ], sim.flowers[localK])
        if (seq.length === 0) {
          newTour = sim.flowers[i].slice()
        } else {
          const nSwaps = Math.min(seq.length, Math.max(1, Math.ceil(epsilon * seq.length)))
          newTour = applySwaps(sim.flowers[i], seq, nSwaps)
        }
      }
    }

    const newCost = tourLength(newTour)
    const improved = newCost < sim.costs[i]

    const newFlowers = sim.flowers.slice()
    const newCosts = sim.costs.slice()
    if (improved) {
      newFlowers[i] = newTour
      newCosts[i] = newCost
    }

    let newGbest = sim.gbest
    let newGbestCost = sim.gbestCost
    if (improved && newCost < sim.gbestCost) {
      newGbest = newTour.slice()
      newGbestCost = newCost
    }

    const newGlobalCount = sim.globalCount + (stepMode === "global" ? 1 : 0)
    const newLocalCount = sim.localCount + (stepMode === "local" ? 1 : 0)
    const newIter = sim.iter + 1

    simRef.current = {
      flowers: newFlowers,
      costs: newCosts,
      gbest: newGbest,
      gbestCost: newGbestCost,
      iter: newIter,
      globalCount: newGlobalCount,
      localCount: newLocalCount,
    }

    setFlowers(newFlowers)
    setGbest(newGbest)
    setGbestCost(newGbestCost)
    setIter(newIter)
    setGlobalCount(newGlobalCount)
    setLocalCount(newLocalCount)
    setMode(stepMode)
    setActiveIdx(i)
    setLastLevy(stepLevy)
    setLevyHistory(h => [...h.slice(-99), { value: stepLevy, mode: stepMode }])

    // Position the icon at the midpoint of the relevant move
    if (stepMode === "global") {
      const [ax, ay] = centroid(sim.flowers[i])
      const [gx, gy] = centroid(sim.gbest)
      setIconPos({ icon: "🐝", x: ((ax + gx) / 2 / 300) * 100, y: ((ay + gy) / 2 / 300) * 100 })
    } else if (localJ !== -1 && localK !== -1) {
      const [jx, jy] = centroid(sim.flowers[localJ])
      const [kx, ky] = centroid(sim.flowers[localK])
      setIconPos({ icon: "🌬️", x: ((jx + kx) / 2 / 300) * 100, y: ((jy + ky) / 2 / 300) * 100 })
    }
  }, [])

  // Animation loop
  const delay = Math.max(50, 660 - speed * 66)
  useEffect(() => {
    if (!running) return
    const id = setInterval(stepOnce, delay)
    return () => clearInterval(id)
  }, [running, stepOnce, delay])

  const modeLabel =
    mode === "global"
      ? "🌍 Global pollination — Lévy flight toward gbest"
      : mode === "local"
        ? "🌸 Local pollination — ε cross-pollination"
        : "Press Step or Run to begin"

  return (
    <div className="fp-root">
      <style>{CSS}</style>

      <header className="fp-header">
        <div className="fp-eyebrow">teeline · algorithms/fpa</div>
        <h2 className="fp-title">Flower Pollination Algorithm</h2>
        <p className="fp-sub">
          A population of candidate tours (flowers) evolves each step: with probability{" "}
          <code>p</code> a flower makes a long-range <strong>Lévy flight</strong> toward the
          global best; otherwise it <strong>cross-pollinates</strong> with two random flowers
          nearby. Drag the sliders to see how <code>p</code> and population size shape search.
        </p>
      </header>

      {/* Tour canvas + emoji icon overlay */}
      <div className="fp-canvas-wrap">
        <TourSVG flowers={flowers} gbest={gbest} activeIdx={activeIdx} />
        {iconPos && (
          <span className="fp-icon" style={{ left: `${iconPos.x}%`, top: `${iconPos.y}%` }}>
            {iconPos.icon}
          </span>
        )}
      </div>

      {/* Canvas legend */}
      <div className="fp-legend">
        <span><span className="fp-dot fp-dot-gbest">●</span> gbest</span>
        <span><span className="fp-dot fp-dot-active">●</span> active flower</span>
        <span><span className="fp-dot fp-dot-ghost">●</span> population</span>
        <span><span className="fp-dot fp-dot-city">●</span> city</span>
      </div>

      {/* Mode chip */}
      <div className={`fp-mode fp-mode-${mode ?? "idle"}`}>{modeLabel}</div>

      {/* Lévy sparkline + step size */}
      <div className="fp-spark-row">
        <div className="fp-spark-wrap">
          <LevySparkline history={levyHistory} />
          <div className="fp-spark-caption">
            <span className="fp-swatch fp-swatch-global">■</span> global Lévy step &nbsp;
            <span className="fp-swatch fp-swatch-local">■</span> local ε step
          </div>
        </div>
        <div className="fp-stepval-wrap">
          <div className="fp-statlabel">step size</div>
          <div className="fp-mono fp-stepval">
            {lastLevy !== null ? lastLevy.toFixed(3) : "—"}
          </div>
          <div className="fp-statlabel" style={{ marginTop: "4px" }}>
            {mode === "global"
              ? "Lévy draw"
              : mode === "local"
                ? "ε (uniform)"
                : ""}
          </div>
        </div>
      </div>

      {/* Stats row */}
      <div className="fp-statgrid">
        <div>
          <div className="fp-statlabel">iteration</div>
          <div className="fp-mono">{iter}</div>
        </div>
        <div>
          <div className="fp-statlabel">gbest distance</div>
          <div className="fp-mono">{gbestCost.toFixed(1)}</div>
        </div>
        <div>
          <div className="fp-statlabel">global steps</div>
          <div className="fp-mono fp-global-val">{globalCount}</div>
        </div>
        <div>
          <div className="fp-statlabel">local steps</div>
          <div className="fp-mono fp-local-val">{localCount}</div>
        </div>
      </div>

      {/* Config sliders */}
      <div className="fp-config">
        <div className="fp-config-row">
          <label className="fp-config-label">
            Switch prob <code>p</code> = <strong>{switchProb.toFixed(2)}</strong>
          </label>
          <input
            type="range" min={0} max={1} step={0.05}
            value={switchProb}
            className="fp-slider"
            onInput={(e) => {
              const v = Number((e.target as HTMLInputElement).value)
              setSwitchProb(v)
              switchProbRef.current = v
            }}
          />
          <div className="fp-hint">
            {switchProb >= 0.9
              ? "Mostly global — fast but risks premature convergence"
              : switchProb <= 0.1
                ? "Mostly local — slow diffuse search, global rarely fires"
                : `~${Math.round(switchProb * 100)}% global / ${Math.round((1 - switchProb) * 100)}% local`}
          </div>
        </div>

        <div className="fp-config-row">
          <label className="fp-config-label">
            Flowers (population) = <strong>{nFlowers}</strong>
          </label>
          <input
            type="range" min={3} max={15} step={1}
            value={nFlowers}
            className="fp-slider"
            onInput={(e) => {
              const n = Number((e.target as HTMLInputElement).value)
              setNFlowers(n)
              reinit(n)
            }}
          />
          <div className="fp-hint">
            {nFlowers <= 4
              ? "Small swarm: fast per-step but low diversity"
              : nFlowers >= 12
                ? "Large swarm: more diversity, slower per step"
                : "Balanced population size"}
          </div>
        </div>

        <div className="fp-config-row">
          <label className="fp-config-label">Speed</label>
          <input
            type="range" min={1} max={10} step={1}
            value={speed}
            className="fp-slider"
            onInput={(e) => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
        </div>
      </div>

      {/* Controls */}
      <div className="fp-controls">
        <button className="fp-btn" onClick={stepOnce} disabled={running}>
          ◀ Step
        </button>
        <button
          className={`fp-btn ${!running ? "fp-btn-primary" : ""}`}
          onClick={() => setRunning(r => !r)}
        >
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="fp-btn" onClick={() => reinit(nFlowers)}>
          ↺ Reset
        </button>
      </div>

      <footer className="fp-footer">
        <span className="fp-mono">cities: {N_CITIES}</span>
        <span className="fp-mono">flowers: {nFlowers}</span>
        <span className="fp-mono">p: {switchProb.toFixed(2)}</span>
        <span className="fp-mono">β (Lévy): 1.5</span>
      </footer>
    </div>
  )
}

// ---------------------------------------------------------------
// Styles — self-contained, scoped via fp- prefix
// ---------------------------------------------------------------
const CSS = `
.fp-root {
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --global: #3b82f6;
  --local: #d97706;
  --gbest: #16a34a;
  --active: #3b82f6;
  --ghost: #9ca3af;
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
.fp-root * { box-sizing: border-box; }
.fp-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

/* Header */
.fp-header { margin-bottom: 2px; }
.fp-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--gbest); margin-bottom: 6px;
}
.fp-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.fp-sub {
  margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55;
}
.fp-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(22,163,74,0.08);
  padding: 1px 4px; border-radius: 4px;
}

/* Canvas */
.fp-canvas-wrap { position: relative; display: block; width: 100%; }
.fp-canvas {
  width: 100%; display: block;
  border-radius: 8px; border: 1px solid var(--line);
}
.fp-bg { fill: var(--panel); }
.fp-ghost { fill: none; stroke: var(--ghost); stroke-width: 1; stroke-opacity: 0.22; stroke-linejoin: round; }
.fp-active { fill: none; stroke: var(--active); stroke-width: 1.8; stroke-linejoin: round; }
.fp-gbest { fill: none; stroke: var(--gbest); stroke-width: 2.5; stroke-linejoin: round; }
.fp-city { fill: var(--city); stroke: var(--bg); stroke-width: 1.5; }
.fp-city-label {
  font-size: 8px; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  fill: #374151; stroke: white; stroke-width: 2.5; paint-order: stroke fill;
  dominant-baseline: auto; pointer-events: none; user-select: none;
}

/* Icon overlay */
.fp-icon {
  position: absolute;
  font-size: 18px;
  transform: translate(-50%, -50%);
  pointer-events: none;
  transition: left 0.35s ease, top 0.35s ease;
  filter: drop-shadow(0 1px 2px rgba(0,0,0,0.3));
}

/* Legend */
.fp-legend {
  display: flex; gap: 14px; flex-wrap: wrap;
  font-size: 0.8rem; color: var(--muted);
}
.fp-dot { font-size: 1em; }
.fp-dot-gbest { color: var(--gbest); }
.fp-dot-active { color: var(--active); }
.fp-dot-ghost { color: var(--ghost); }
.fp-dot-city { color: var(--city); }

/* Mode chip */
.fp-mode {
  font-size: 0.95rem; font-weight: 600;
  padding: 8px 14px; border-radius: 8px;
  border: 1px solid transparent;
}
.fp-mode-idle { background: var(--panel); color: var(--muted); border-color: var(--line); }
.fp-mode-global { background: rgba(59,130,246,0.1); color: #1d4ed8; border-color: rgba(59,130,246,0.3); }
.fp-mode-local { background: rgba(217,119,6,0.1); color: #92400e; border-color: rgba(217,119,6,0.3); }

/* Sparkline row */
.fp-spark-row { display: flex; gap: 16px; align-items: flex-start; }
.fp-spark-wrap { display: flex; flex-direction: column; gap: 4px; }
.fp-spark {
  width: 180px; height: 54px;
  border: 1px solid var(--line); border-radius: 6px;
  background: var(--panel); display: block;
}
.fp-spark-caption {
  font-size: 0.72rem; color: var(--muted);
}
.fp-swatch { font-size: 0.9em; }
.fp-swatch-global { color: var(--global); }
.fp-swatch-local { color: var(--local); }
.fp-stepval-wrap { display: flex; flex-direction: column; }
.fp-stepval { font-size: 1.3rem; font-weight: 600; color: var(--text); }

/* Stats grid */
.fp-statgrid {
  display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px;
}
@media (max-width: 480px) {
  .fp-statgrid { grid-template-columns: repeat(2, 1fr); }
}
.fp-statlabel {
  font-size: 0.7rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}
.fp-global-val { color: var(--global); }
.fp-local-val { color: var(--local); }

/* Config sliders */
.fp-config { display: flex; flex-direction: column; gap: 10px; }
.fp-config-row { display: flex; flex-direction: column; gap: 4px; }
.fp-config-label { font-size: 0.88rem; }
.fp-config-label code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.9em;
}
.fp-slider { width: 100%; accent-color: var(--gbest); cursor: pointer; }
.fp-hint { font-size: 0.78rem; color: var(--muted); }

/* Controls */
.fp-controls { display: flex; gap: 8px; }
.fp-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer;
  font-family: inherit;
}
.fp-btn:hover:not(:disabled) { border-color: var(--gbest); }
.fp-btn:disabled { opacity: 0.45; cursor: default; }
.fp-btn-primary { color: var(--gbest); border-color: var(--gbest); }

/* Footer */
.fp-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 10px; border-top: 1px solid var(--line);
  color: var(--muted);
}
`

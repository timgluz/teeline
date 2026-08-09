import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  CITIES, N_CITIES,
  tourLength, maxPheromone, makeInitState, stepOnce,
} from "./aco-algo"
import type { EventMode } from "./aco-algo"

const AXIS_COLORS = [
  "#0d9488", "#2563eb", "#7c3aed", "#db2777", "#ea580c",
  "#16a34a", "#0891b2",
]

const DEFAULTS = { alpha: 1.0, beta: 2.0, evaporationRate: 0.5, numAnts: 10 }

function cityColor(i: number, onBest: boolean, onLast: boolean): string {
  if (onBest) return "#16a34a"
  if (onLast) return "#ea580c"
  return AXIS_COLORS[i % AXIS_COLORS.length]
}

// ---------------------------------------------------------------
// PheromoneCanvas — edge thickness ∝ pheromone, best-tour overlay
// ---------------------------------------------------------------
interface CanvasProps {
  pheromone: number[][]
  maxP: number
  tau0: number
  bestTour: number[]
  lastTour: number[] | null
  phase: string
}
function PheromoneCanvas({ pheromone, maxP, tau0, bestTour, lastTour, phase }: CanvasProps) {
  const bestSet = useMemo(() => new Set(bestTour), [bestTour])
  const lastSet = useMemo(() => lastTour ? new Set(lastTour) : new Set<number>(), [lastTour])

  const bestPts = bestTour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ")
  const bestPtsClosed = bestPts + ` ${CITIES[bestTour[0]][0]},${CITIES[bestTour[0]][1]}`
  const lastPts = lastTour
    ? lastTour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ") + ` ${CITIES[lastTour[0]][0]},${CITIES[lastTour[0]][1]}`
    : ""

  const edges: Array<[number, number, number]> = []
  for (let i = 0; i < N_CITIES; i++) {
    for (let j = i + 1; j < N_CITIES; j++) {
      edges.push([i, j, pheromone[i][j]])
    }
  }

  return (
    <svg viewBox="0 0 300 300" className="aco-canvas" role="img" aria-label="ACO pheromone map">
      <rect x={0} y={0} width={300} height={300} className="aco-bg" />

      {/* pheromone edges — only edges above baseline tau0 are visible;
          untouched edges stay invisible so the colony's emergent trails are
          the only thing on the canvas (no full-graph noise at the start) */}
      {edges.map(([i, j, p], k) => {
        if (p <= tau0 * 1.0001) return null  // untouched, invisible
        const ratio = maxP > 0 ? p / maxP : 0
        const amp = Math.pow(ratio, 0.35)
        return (
          <line key={"p" + k}
            x1={CITIES[i][0]} y1={CITIES[i][1]}
            x2={CITIES[j][0]} y2={CITIES[j][1]}
            stroke="#0d9488"
            strokeWidth={0.3 + amp * 5.0}
            opacity={0.05 + amp * 0.75}
            strokeLinecap="round"
            strokeDasharray={amp > 0.35 ? "5 3" : "3 5"}
          />
        )
      })}

      {/* best tour underlay (faded fill) */}
      <polygon points={bestPts} className="aco-best-area" />

      {/* best tour — thick solid closed line */}
      <polyline points={bestPtsClosed} className="aco-best-tour" />

      {/* last tour (dashed, highlighted) */}
      {lastTour && phase === 'building' && (
        <polyline points={lastPts} className="aco-last-tour" />
      )}

      {/* cities */}
      {CITIES.map(([x, y], i) => {
        const onBest = bestSet.has(i)
        const onLast = lastSet.has(i)
        return (
          <g key={i}>
            <circle cx={x} cy={y} r={onBest ? 7 : onLast ? 6.5 : 5}
              fill={cityColor(i, onBest, onLast)}
              className="aco-city"
            />
            <text x={x + 8} y={y - 5} className="aco-city-label">{i}</text>
          </g>
        )
      })}
    </svg>
  )
}

// ---------------------------------------------------------------
// Side panel — epoch, ant progress, pheromone heatmap grid
// ---------------------------------------------------------------
function PheromoneHeatmap({ pheromone, maxP }: { pheromone: number[][]; maxP: number }) {
  const n = pheromone.length
  const w = 112, pad = 2, cell = Math.floor((w - pad * 2) / n)
  const totalW = cell * n + pad * 2

  return (
    <svg viewBox={`0 0 ${totalW} ${totalW}`} className="aco-heatmap">
      <rect x={0} y={0} width={totalW} height={totalW} className="aco-heatmap-bg" rx={3} />
      {/* separators every 4 cities — visually chunk the 12×12 matrix */}
      {[4, 8].map(x => (
        <line key={"v" + x} x1={pad + x * cell} y1={pad} x2={pad + x * cell} y2={pad + n * cell}
          stroke="var(--line)" strokeWidth={1} />
      ))}
      {[4, 8].map(y => (
        <line key={"h" + y} x1={pad} y1={pad + y * cell} x2={pad + n * cell} y2={pad + y * cell}
          stroke="var(--line)" strokeWidth={1} />
      ))}
      {Array.from({ length: n }, (_, i) =>
        Array.from({ length: n }, (_, j) => {
          if (i >= j) return null  // lower triangle + diagonal
          const ratio = maxP > 0 ? pheromone[i][j] / maxP : 0
          const r = Math.round(240 - ratio * 200)
          const g = Math.round(253 - ratio * 200)
          const b = Math.round(244 - ratio * 220)
          return (
            <rect key={`${i}-${j}`}
              x={pad + j * cell} y={pad + i * cell}
              width={cell - 1.5} height={cell - 1.5}
              fill={`rgb(${r},${g},${b})`}
              rx={1.5}
            />
          )
        })
      )}
    </svg>
  )
}

function SidePanel(props: {
  epoch: number; phase: string; antIdx: number; numAnts: number;
  pheromone: number[][]; bestCost: number; maxP: number;
}) {
  const { epoch, phase, antIdx, numAnts, pheromone, bestCost, maxP } = props

  return (
    <div className="aco-sidebar">
      <div className="aco-side-section">
        <div className="aco-side-label">epoch</div>
        <div className="aco-mono">{epoch}</div>
      </div>
      <div className="aco-side-section">
        <div className="aco-side-label">phase</div>
        <div className="aco-mono">{phase === 'building' ? `🐜 ant ${antIdx + 1}/${numAnts}` : '💨 deposit'}</div>
      </div>
      <div className="aco-side-section">
        <div className="aco-side-label">best cost</div>
        <div className="aco-mono">{bestCost.toFixed(0)}</div>
      </div>
      <div className="aco-side-section">
        <div className="aco-side-label">pheromone</div>
        <PheromoneHeatmap pheromone={pheromone} maxP={maxP} />
      </div>
    </div>
  )
}

// ---------------------------------------------------------------
// Cost sparkline
// ---------------------------------------------------------------
function Sparkline({ values }: { values: number[] }) {
  if (values.length < 2) return null
  const W = 300, H = 54, minC = Math.min(...values), maxC = Math.max(...values)
  const range = maxC - minC || 1
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * W
    const y = H - ((v - minC) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="aco-spark">
      <rect x={0} y={0} width={W} height={H} className="aco-bg" rx={4} />
      <polyline points={pts.join(" ")} fill="none" stroke="#0d9488" strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function AcoExplainer() {
  const [alpha, setAlpha] = useState(DEFAULTS.alpha)
  const [beta, setBeta] = useState(DEFAULTS.beta)
  const [evapRate, setEvapRate] = useState(DEFAULTS.evaporationRate)
  const [numAnts, setNumAnts] = useState(DEFAULTS.numAnts)
  const [speed, setSpeed] = useState(5)

  const simRef = useRef(makeInitState(alpha, beta, evapRate, numAnts))
  const [pheromone, setPheromone] = useState(() => simRef.current.pheromone)
  const [epoch, setEpoch] = useState(0)
  const [phase, setPhase] = useState<"building" | "depositing">(() => simRef.current.phase)
  const [antIdx, setAntIdx] = useState(0)
  const [bestTour, setBestTour] = useState<number[]>(() => simRef.current.bestTour.slice())
  const [bestCost, setBestCost] = useState(() => simRef.current.bestCost)
  const [lastTour, setLastTour] = useState<number[] | null>(null)
  const [lastEvent, setLastEvent] = useState<EventMode | null>(null)
  const [costHistory, setCostHistory] = useState<number[]>([])
  const [tau0, setTau0] = useState(() => simRef.current.tau0)
  const [step, setStep] = useState(0)
  const [running, setRunning] = useState(false)

  const reinit = useCallback((a: number, b: number, e: number, n: number) => {
    simRef.current = makeInitState(a, b, e, n)
    setPheromone(simRef.current.pheromone); setEpoch(0)
    setPhase('building'); setAntIdx(0)
    setBestTour(simRef.current.bestTour.slice()); setBestCost(simRef.current.bestCost)
    setLastTour(null); setLastEvent(null); setCostHistory([])
    setTau0(simRef.current.tau0)
    setStep(0); setRunning(false)
  }, [])

  const step_fn = useCallback(() => {
    const next = stepOnce(simRef.current)
    simRef.current = next
    setPheromone(next.pheromone.map(r => r.slice()))
    setEpoch(next.epoch); setPhase(next.phase); setAntIdx(next.antIdx)
    setBestTour(next.bestTour.slice()); setBestCost(next.bestCost)
    setLastTour(next.lastTour); setLastEvent(next.lastEvent)
    setCostHistory(next.costHistory.slice()); setStep(next.step)
  }, [])

  useEffect(() => {
    if (!running) return
    const ms = Math.max(30, 660 - speed * 66)
    const id = setInterval(step_fn, ms)
    return () => clearInterval(id)
  }, [running, speed, step_fn])

  const maxP = useMemo(() => maxPheromone(simRef.current), [pheromone])

  let chipText = "Press Step or Run to watch the colony evolve"
  let chipClass = "aco-chip aco-chip-idle"
  if (lastEvent === 'ant-built') {
    chipText = `🐜 ant built a tour — cost ${simRef.current.lastTours.length > 0 ? tourLength(simRef.current.lastTours[simRef.current.lastTours.length - 1]).toFixed(0) : "?"}`
    chipClass = "aco-chip aco-chip-build"
  } else if (lastEvent === 'improved') {
    chipText = `⭐ new best! epoch ${epoch - 1}, cost ${bestCost.toFixed(0)}`
    chipClass = "aco-chip aco-chip-improve"
  } else if (lastEvent === 'deposited') {
    chipText = `💨 evaporated + 📥 deposited ${numAnts} tours — advancing to epoch ${epoch}`
    chipClass = "aco-chip aco-chip-deposit"
  }

  return (
    <div className="aco-root">
      <style>{CSS}</style>

      <header className="aco-header">
        <div className="aco-eyebrow">teeline · algorithms/aco</div>
        <h2 className="aco-title">Ant Colony Optimization</h2>
        <p className="aco-sub">
          A colony of ants independently constructs tours, each next city chosen
          probabilistically by a <strong>pheromone trail</strong> (reinforced on short edges
          and decaying over time) weighted by <strong>heuristic desirability</strong>
          (1/distance). Watch the pheromone edges strengthen on good edges and fade on
          bad ones as epochs advance.
        </p>
      </header>

      <div className="aco-viz-row">
        <div className="aco-canvas-wrap">
          <PheromoneCanvas
            pheromone={pheromone} maxP={maxP} tau0={tau0} bestTour={bestTour}
            lastTour={lastTour} phase={phase}
          />
        </div>
        <SidePanel
          epoch={epoch} phase={phase} antIdx={antIdx} numAnts={numAnts}
          pheromone={pheromone} bestCost={bestCost} maxP={maxP}
        />
      </div>

      <div className="aco-legend">
        <span><span className="aco-swatch aco-swatch-best" /> best tour</span>
        <span><span className="aco-swatch aco-swatch-last" /> ant's last tour</span>
        <span><span className="aco-swatch aco-swatch-phero" /> pheromone edge</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="aco-section-label">Cost history (best per epoch)</div>
      <Sparkline values={costHistory} />

      <div className="aco-statgrid">
        <div>
          <div className="aco-statlabel">epoch</div>
          <div className="aco-mono">{epoch}</div>
        </div>
        <div>
          <div className="aco-statlabel">best cost</div>
          <div className="aco-mono">{bestCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="aco-statlabel">ants / epoch</div>
          <div className="aco-mono">{numAnts}</div>
        </div>
        <div>
          <div className="aco-statlabel">step</div>
          <div className="aco-mono">{step}</div>
        </div>
      </div>

      <div className="aco-config">
        <div className="aco-config-row">
          <label className="aco-config-label">α (pheromone influence) = <strong>{alpha.toFixed(1)}</strong></label>
          <input type="range" min={0} max={10} step={0.1} value={alpha}
            className="aco-slider"
            onInput={e => { const v = Number((e.target as HTMLInputElement).value); setAlpha(v); reinit(v, beta, evapRate, numAnts) }}
          />
        </div>
        <div className="aco-config-row">
          <label className="aco-config-label">β (heuristic influence) = <strong>{beta.toFixed(1)}</strong></label>
          <input type="range" min={0} max={6} step={0.1} value={beta}
            className="aco-slider"
            onInput={e => { const v = Number((e.target as HTMLInputElement).value); setBeta(v); reinit(alpha, v, evapRate, numAnts) }}
          />
        </div>
        <div className="aco-config-row">
          <label className="aco-config-label">ρ (evaporation rate) = <strong>{evapRate.toFixed(2)}</strong></label>
          <input type="range" min={0.01} max={0.99} step={0.01} value={evapRate}
            className="aco-slider"
            onInput={e => { const v = Number((e.target as HTMLInputElement).value); setEvapRate(v); reinit(alpha, beta, v, numAnts) }}
          />
        </div>
        <div className="aco-config-row">
          <label className="aco-config-label">Colony size = <strong>{numAnts}</strong></label>
          <input type="range" min={2} max={30} step={1} value={numAnts}
            className="aco-slider"
            onInput={e => { const v = Number((e.target as HTMLInputElement).value); setNumAnts(v); reinit(alpha, beta, evapRate, v) }}
          />
        </div>
        <div className="aco-config-row">
          <label className="aco-config-label">Speed</label>
          <input type="range" min={1} max={10} step={1} value={speed}
            className="aco-slider"
            onInput={e => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
        </div>
      </div>

       <div className="aco-controls">
         <button className="aco-btn" onClick={step_fn} disabled={running}>◀ Step</button>
         <button className={`aco-btn ${!running ? "aco-btn-primary" : ""}`}
           onClick={() => setRunning(r => !r)}>
           {running ? "⏸ Pause" : "▶ Run"}
         </button>
         <button className="aco-btn" onClick={() => reinit(alpha, beta, evapRate, numAnts)}>↺ Reset</button>
       </div>

       <div className="aco-scenarios">
         <div className="aco-section-label">Scenarios</div>
         <div className="aco-scenario-row">
           {([
             { label: 'Default', desc: 'balanced α=1.0 β=2.0 ρ=0.50 ants=10', a: 1.0, b: 2.0, e: 0.50, n: 10 },
             { label: 'Pheromone-heavy', desc: 'high α=3.0 β=1.0 — ants follow existing trails', a: 3.0, b: 1.0, e: 0.30, n: 10 },
             { label: 'Distance-driven', desc: 'high β=5.0 — ants prioritise short edges', a: 0.5, b: 5.0, e: 0.50, n: 10 },
             { label: 'Fast evaporation', desc: 'ρ=0.85 — trails fade quickly, more exploration', a: 1.0, b: 2.0, e: 0.85, n: 10 },
             { label: 'Large colony', desc: 'ants=25 — more deposits per epoch', a: 1.0, b: 2.0, e: 0.50, n: 25 },
           ] as const).map(s => (
             <button key={s.label} className="aco-scenario-btn" title={s.desc}
               onClick={() => { setAlpha(s.a); setBeta(s.b); setEvapRate(s.e); setNumAnts(s.n); reinit(s.a, s.b, s.e, s.n) }}>
               {s.label}
             </button>
           ))}
         </div>
       </div>

      <footer className="aco-footer">
        <span className="aco-mono">cities: {N_CITIES}</span>
        <span className="aco-mono">α={alpha.toFixed(1)} β={beta.toFixed(1)}</span>
        <span className="aco-mono">ρ={evapRate.toFixed(2)}</span>
        <span className="aco-mono">ants: {numAnts}</span>
        <span className="aco-mono">classic AS</span>
      </footer>
    </div>
  )
}

const CSS = `
.aco-root {
  --accent: #0d9488;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --improve: #16a34a;
  --last: #ea580c;
  --phero: #0d9488;
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
  gap: 10px;
}
.aco-root * { box-sizing: border-box; }
.aco-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

.aco-header { margin-bottom: 2px; }
.aco-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.aco-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.aco-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.aco-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(13,148,136,0.1);
  color: var(--accent); padding: 1px 4px; border-radius: 4px;
}

.aco-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.aco-bg { fill: var(--panel); }
.aco-best-area { fill: #bbf7d0; stroke: none; opacity: 0.5; }
.aco-best-tour { fill: none; stroke: #16a34a; stroke-width: 3.2; stroke-linejoin: round; }
.aco-last-tour { fill: none; stroke: var(--last); stroke-width: 1.8; stroke-linejoin: round; stroke-dasharray: 4 3; }
.aco-city { stroke: #fff; stroke-width: 1.2; }
.aco-city-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8px; fill: #374151; stroke: white; stroke-width: 2.5;
  paint-order: stroke fill; dominant-baseline: auto;
  pointer-events: none; user-select: none;
}

.aco-viz-row { display: flex; gap: 10px; align-items: stretch; }
.aco-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

.aco-sidebar {
  width: 130px; flex-shrink: 0; display: flex; flex-direction: column; gap: 6px;
  padding: 8px; background: var(--panel); border: 1px solid var(--line); border-radius: 8px;
}
.aco-side-section { display: flex; flex-direction: column; gap: 1px; }
.aco-side-label {
  font-size: 0.65rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em;
}

.aco-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); align-items: center; }
.aco-swatch {
  display: inline-block; width: 22px; height: 3px;
  border-radius: 2px; margin-right: 3px; vertical-align: middle;
}
.aco-swatch-best  { background: var(--improve); }
.aco-swatch-last  { background: var(--last); }
.aco-swatch-phero { background: var(--phero); }

.aco-chip {
  font-size: 0.85rem; font-weight: 600; padding: 8px 12px;
  border-radius: 8px; border: 1px solid transparent;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.aco-chip-idle    { background: var(--panel); color: var(--muted); border-color: var(--line); font-weight: 400; }
.aco-chip-build   { background: rgba(13,148,136,0.08); color: #0f766e; border-color: rgba(13,148,136,0.25); }
.aco-chip-improve { background: #dcfce7; color: #15803d; border-color: #bbf7d0; }
.aco-chip-deposit { background: #fef3c7; color: #92400e; border-color: #fde68a; }

.aco-section-label {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--muted); margin-bottom: -4px;
}

.aco-spark {
  width: 100%; height: 54px; display: block;
  border: 1px solid var(--line); border-radius: 6px; background: var(--panel);
}

.aco-statgrid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
@media (max-width: 540px) { .aco-statgrid { grid-template-columns: repeat(2, 1fr); } }
.aco-statlabel {
  font-size: 0.65rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}

.aco-config { display: flex; flex-direction: column; gap: 8px; }
.aco-config-row { display: flex; flex-direction: column; gap: 3px; }
.aco-config-label { font-size: 0.85rem; }
.aco-slider { width: 100%; accent-color: var(--accent); cursor: pointer; }

.aco-controls { display: flex; gap: 8px; }
.aco-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer; font-family: inherit;
}
.aco-btn:hover:not(:disabled) { border-color: var(--accent); }
.aco-btn:disabled { opacity: 0.45; cursor: default; }
.aco-btn-primary { color: var(--accent); border-color: var(--accent); }

.aco-heatmap { width: 100%; display: block; margin-top: 4px; }
.aco-heatmap-bg { fill: var(--panel); }

.aco-scenarios { margin-top: 4px; }
.aco-scenario-row { display: flex; gap: 6px; flex-wrap: wrap; }
.aco-scenario-btn {
  background: var(--panel);
  color: var(--muted);
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 4px 10px;
  font-size: 0.78rem;
  cursor: pointer;
  font-family: inherit;
  transition: border-color 0.15s;
}
.aco-scenario-btn:hover { border-color: var(--accent); color: var(--text); }

.aco-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 8px; border-top: 1px solid var(--line); color: var(--muted);
}
`

import { useState, useRef, useEffect, useCallback } from "preact/hooks"
import type { Move, TabuEntry, EventMode, SimState } from "./tabu-algo"
import { CITIES, N_CITIES, makeInitState, stepOnce } from "./tabu-algo"

function polyPts(tour: number[]): string {
  const pts = tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ")
  return pts + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}`
}

function getEdgeDiff(tour: number[], move: Move): {
  removed: [[number, number], [number, number]]
  added: [[number, number], [number, number]]
} {
  const n = tour.length
  const [i, j] = move
  const a = tour[(i - 1 + n) % n], b = tour[i]
  const c = tour[j], d = tour[(j + 1) % n]
  return { removed: [[a, b], [c, d]], added: [[a, c], [b, d]] }
}

// ---------------------------------------------------------------
// CurrentTourSVG — gray tour, edge diff; best tour underlay when paused
// ---------------------------------------------------------------
interface CurrentTourSVGProps {
  tour: number[]
  best: number[]
  lastMove: Move | null
  showBest: boolean
}
function CurrentTourSVG({ tour, best, lastMove, showBest }: CurrentTourSVGProps) {
  const diff = lastMove ? getEdgeDiff(tour, lastMove) : null
  const changedCities = lastMove ? new Set([lastMove[0], lastMove[1]]) : new Set<number>()
  return (
    <svg viewBox="0 0 300 300" className="tabu-canvas" role="img" aria-label="Current tour">
      <rect x={0} y={0} width={300} height={300} className="tabu-bg" />
      {showBest && <polyline points={polyPts(best)} className="tabu-best-tour" />}
      <polyline points={polyPts(tour)} className={showBest ? "tabu-current-tour tabu-current-faded" : "tabu-current-tour"} />
      {diff && diff.removed.map(([a, b], i) => (
        <line key={i}
          x1={CITIES[a][0]} y1={CITIES[a][1]}
          x2={CITIES[b][0]} y2={CITIES[b][1]}
          className="tabu-edge-removed"
        />
      ))}
      {diff && diff.added.map(([a, b], i) => (
        <line key={i}
          x1={CITIES[a][0]} y1={CITIES[a][1]}
          x2={CITIES[b][0]} y2={CITIES[b][1]}
          className="tabu-edge-added"
        />
      ))}
      {CITIES.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y}
          r={changedCities.has(i) ? 7 : 4}
          className={changedCities.has(i) ? "tabu-city tabu-city-changed" : "tabu-city"}
        />
      ))}
      {CITIES.map(([x, y], i) => (
        <text key={i} x={x + 6} y={y - 5} className="tabu-city-label">{i}</text>
      ))}
    </svg>
  )
}

// ---------------------------------------------------------------
// TabuListPanel — (i,j) badges, fading by age
// ---------------------------------------------------------------
interface TabuListPanelProps {
  tabuList: TabuEntry[]
  tenure: number
  step: number
}
function TabuListPanel({ tabuList, tenure, step }: TabuListPanelProps) {
  return (
    <div className="tabu-list-panel">
      <div className="tabu-list-title">Tabu List</div>
      <div className="tabu-list-subtitle">(tenure={tenure})</div>
      {tabuList.length === 0
        ? <div className="tabu-list-empty">no forbidden moves yet</div>
        : tabuList.map((entry, idx) => {
            const age = step - entry.addedAtStep
            const opacity = 1 - (age / tenure) * 0.75
            return (
              <div key={idx} className="tabu-badge" style={{ opacity: Math.max(0.25, opacity) }}>
                ({entry.move[0]},{entry.move[1]})
              </div>
            )
          })
      }
    </div>
  )
}

// ---------------------------------------------------------------
// CostSparkline — cost per step
// ---------------------------------------------------------------
function CostSparkline({ costHistory }: { costHistory: number[] }) {
  const W = 300, H = 54
  if (costHistory.length < 2) {
    return (
      <svg viewBox={`0 0 ${W} ${H}`} className="tabu-spark">
        <rect x={0} y={0} width={W} height={H} className="tabu-bg" rx={4} />
        <text x={W / 2} y={H / 2 + 4} textAnchor="middle" className="tabu-spark-idle">
          Run a few steps to see cost history
        </text>
      </svg>
    )
  }
  const minC = Math.min(...costHistory)
  const maxC = Math.max(...costHistory)
  const range = maxC - minC || 1
  const pad = 4
  const xScale = (W - pad * 2) / (costHistory.length - 1)
  const yScale = (H - pad * 2) / range
  const pts = costHistory
    .map((c, i) => `${(pad + i * xScale).toFixed(1)},${(H - pad - (c - minC) * yScale).toFixed(1)}`)
    .join(" ")
  const lastX = pad + (costHistory.length - 1) * xScale
  const lastY = H - pad - (costHistory[costHistory.length - 1] - minC) * yScale
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="tabu-spark">
      <rect x={0} y={0} width={W} height={H} className="tabu-bg" rx={4} />
      <polyline points={pts} fill="none" stroke="#0d9488" strokeWidth={1.5} strokeLinejoin="round" />
      <circle cx={lastX} cy={lastY} r={3} fill="#0d9488" />
    </svg>
  )
}

// ---------------------------------------------------------------
// TabuExplainer — main component
// ---------------------------------------------------------------
export default function TabuExplainer() {
  const [tenure, setTenure] = useState(7)
  const [sampleSize, setSampleSize] = useState(10)
  const [speed, setSpeed] = useState(5)

  const simRef = useRef<SimState>(makeInitState(7, 10))

  const [tour, setTour] = useState(() => simRef.current.tour)
  const [best, setBest] = useState<number[]>(() => simRef.current.best)
  const [bestCost, setBestCost] = useState(() => simRef.current.bestCost)
  const [currentCost, setCurrentCost] = useState(() => simRef.current.currentCost)
  const [tabuList, setTabuList] = useState<TabuEntry[]>([])
  const [step, setStep] = useState(0)
  const [lastMove, setLastMove] = useState<Move | null>(null)
  const [eventMode, setEventMode] = useState<EventMode | null>(null)
  const [lastDelta, setLastDelta] = useState<number | null>(null)
  const [aspirationHits, setAspirationHits] = useState(0)
  const [improvements, setImprovements] = useState(0)
  const [costHistory, setCostHistory] = useState<number[]>([])
  const [running, setRunning] = useState(false)

  const reinit = useCallback((t: number, ss: number) => {
    const s = makeInitState(t, ss)
    simRef.current = s
    setTour(s.tour.slice()); setBest(s.best.slice()); setBestCost(s.bestCost)
    setCurrentCost(s.currentCost); setTabuList([]); setStep(0)
    setLastMove(null); setEventMode(null); setLastDelta(null)
    setAspirationHits(0); setImprovements(0); setCostHistory([])
    setRunning(false)
  }, [])

  const step_fn = useCallback(() => {
    const next = stepOnce(simRef.current)
    simRef.current = next
    setTour(next.tour.slice()); setBest(next.best.slice()); setBestCost(next.bestCost)
    setCurrentCost(next.currentCost); setTabuList(next.tabuList.slice())
    setStep(next.step); setLastMove(next.lastMove); setEventMode(next.lastEventMode)
    setLastDelta(next.lastDelta); setAspirationHits(next.aspirationHits)
    setImprovements(next.improvements); setCostHistory(next.costHistory.slice())
  }, [])

  useEffect(() => {
    if (!running) return
    const ms = Math.max(50, 660 - speed * 66)
    const id = setInterval(step_fn, ms)
    return () => clearInterval(id)
  }, [running, speed, step_fn])

  let chipText = "Press Step or Run to begin"
  let chipClass = "tabu-chip tabu-chip-idle"
  if (eventMode === 'improvement' && lastMove !== null) {
    chipText = `✅ improved  Δ = ${lastDelta !== null ? lastDelta.toFixed(1) : "—"}  (${lastMove[0]},${lastMove[1]})`
    chipClass = "tabu-chip tabu-chip-improve"
  } else if (eventMode === 'admissible' && lastMove !== null) {
    chipText = `➡️ best admissible  Δ = +${lastDelta !== null ? Math.abs(lastDelta).toFixed(1) : "—"}  (tabu avoided)`
    chipClass = "tabu-chip tabu-chip-admissible"
  } else if (eventMode === 'aspiration' && lastMove !== null) {
    chipText = `⭐ aspiration — new global best!  (${lastMove[0]},${lastMove[1]})`
    chipClass = "tabu-chip tabu-chip-aspiration"
  }

  return (
    <div className="tabu-root">
      <style>{CSS}</style>

      <header className="tabu-header">
        <div className="tabu-eyebrow">teeline · algorithms/tabu</div>
        <h2 className="tabu-title">Tabu Search</h2>
        <p className="tabu-sub">
          A single tour improves via <strong>best-admissible 2-opt</strong> moves. Recent moves are
          added to the <strong>tabu list</strong> and forbidden for <code>tenure</code> steps —
          preventing cycles. If a forbidden move beats the global best, the{" "}
          <strong>aspiration criterion</strong> overrides the ban.
        </p>
      </header>

      <div className="tabu-viz-row">
        <div className="tabu-canvas-wrap">
          <CurrentTourSVG tour={tour} best={best} lastMove={lastMove} showBest={!running && step > 0} />
        </div>
        <TabuListPanel tabuList={tabuList} tenure={tenure} step={step} />
      </div>

      <div className="tabu-legend">
        <span><span className="tabu-swatch tabu-swatch-current" /> current tour</span>
        <span><span className="tabu-swatch tabu-swatch-removed" /> removed edge</span>
        <span><span className="tabu-swatch tabu-swatch-added" /> added edge</span>
        {!running && step > 0 && <span><span className="tabu-swatch tabu-swatch-best" /> best tour</span>}
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="tabu-section-label">Cost history</div>
      <CostSparkline costHistory={costHistory} />

      <div className="tabu-statgrid">
        <div>
          <div className="tabu-statlabel">step</div>
          <div className="tabu-mono">{step}</div>
        </div>
        <div>
          <div className="tabu-statlabel">tabu size</div>
          <div className="tabu-mono">{tabuList.length}/{tenure}</div>
        </div>
        <div>
          <div className="tabu-statlabel">best cost</div>
          <div className="tabu-mono">{bestCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="tabu-statlabel">current cost</div>
          <div className="tabu-mono">{currentCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="tabu-statlabel">improvements</div>
          <div className="tabu-mono">{improvements}</div>
        </div>
        <div>
          <div className="tabu-statlabel">aspiration hits</div>
          <div className="tabu-mono">{aspirationHits}</div>
        </div>
      </div>

      <div className="tabu-config">
        <div className="tabu-config-row">
          <label className="tabu-config-label">Tenure = <strong>{tenure}</strong></label>
          <input type="range" min={3} max={15} step={1} value={tenure}
            className="tabu-slider"
            onInput={e => {
              const v = Number((e.target as HTMLInputElement).value)
              setTenure(v); reinit(v, sampleSize)
            }}
          />
          <div className="tabu-hint">Steps a move stays forbidden</div>
        </div>
        <div className="tabu-config-row">
          <label className="tabu-config-label">Sample size = <strong>{sampleSize}</strong></label>
          <input type="range" min={5} max={30} step={5} value={sampleSize}
            className="tabu-slider"
            onInput={e => {
              const v = Number((e.target as HTMLInputElement).value)
              setSampleSize(v); reinit(tenure, v)
            }}
          />
          <div className="tabu-hint">Neighbours evaluated per step</div>
        </div>
        <div className="tabu-config-row">
          <label className="tabu-config-label">Speed</label>
          <input type="range" min={1} max={10} step={1} value={speed}
            className="tabu-slider"
            onInput={e => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
        </div>
      </div>

      <div className="tabu-controls">
        <button className="tabu-btn" onClick={step_fn} disabled={running}>◀ Step</button>
        <button className={`tabu-btn ${!running ? "tabu-btn-primary" : ""}`}
          onClick={() => setRunning(r => !r)}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="tabu-btn" onClick={() => reinit(tenure, sampleSize)}>↺ Reset</button>
      </div>

      <footer className="tabu-footer">
        <span className="tabu-mono">cities: {N_CITIES}</span>
        <span className="tabu-mono">tenure: {tenure}</span>
        <span className="tabu-mono">sample: {sampleSize}</span>
        <span className="tabu-mono">neighbourhood: 2-opt</span>
      </footer>
    </div>
  )
}

const CSS = `
.tabu-root {
  --accent: #0d9488;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --improve: #16a34a;
  --removed: #f97316;
  --city: #f2a154;
  --tour-current: #6b7280;
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
.tabu-root * { box-sizing: border-box; }
.tabu-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

/* Header */
.tabu-header { margin-bottom: 2px; }
.tabu-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.tabu-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.tabu-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.tabu-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(13,148,136,0.1);
  color: var(--accent); padding: 1px 4px; border-radius: 4px;
}

/* Section labels */
.tabu-section-label {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--muted); margin-bottom: -4px;
}

/* Canvas */
.tabu-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.tabu-bg { fill: var(--panel); }
.tabu-best-tour { fill: none; stroke: var(--improve); stroke-width: 2.5; stroke-linejoin: round; }
.tabu-current-tour { fill: none; stroke: var(--tour-current); stroke-width: 1.8; stroke-linejoin: round; }
.tabu-current-faded { opacity: 0.3; }
.tabu-edge-removed { stroke: var(--removed); stroke-width: 2; stroke-dasharray: 4 3; }
.tabu-edge-added { stroke: var(--accent); stroke-width: 2.5; }
.tabu-city { fill: var(--city); }
.tabu-city-changed { stroke: #fff; stroke-width: 2; }
.tabu-city-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8px; fill: #374151; stroke: white; stroke-width: 2.5;
  paint-order: stroke fill; dominant-baseline: auto;
  pointer-events: none; user-select: none;
}

/* Viz row */
.tabu-viz-row { display: flex; gap: 10px; align-items: stretch; }
.tabu-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

/* Tabu list panel */
.tabu-list-panel {
  width: 130px; flex-shrink: 0; display: flex; flex-direction: column; gap: 4px;
  padding: 8px; background: var(--panel); border: 1px solid var(--line); border-radius: 8px;
}
.tabu-list-title {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;
  color: var(--muted); margin-bottom: 0;
}
.tabu-list-subtitle {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;
  color: var(--muted); margin-bottom: 4px;
}
.tabu-list-empty { font-size: 0.75rem; color: var(--muted); font-style: italic; }
.tabu-badge {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; font-weight: 600;
  background: var(--accent); color: #fff;
  border-radius: 5px; padding: 3px 8px; text-align: center;
  transition: opacity 0.2s;
}

/* Legend */
.tabu-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); align-items: center; }
.tabu-swatch {
  display: inline-block; width: 22px; height: 3px;
  border-radius: 2px; margin-right: 3px; vertical-align: middle;
}
.tabu-swatch-best    { background: var(--improve); }
.tabu-swatch-current { background: var(--tour-current); }
.tabu-swatch-removed { background: var(--removed); }
.tabu-swatch-added   { background: var(--accent); }

/* Event chip */
.tabu-chip {
  font-size: 0.85rem; font-weight: 600; padding: 8px 12px;
  border-radius: 8px; border: 1px solid transparent;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.tabu-chip-idle       { background: var(--panel); color: var(--muted); border-color: var(--line); font-weight: 400; }
.tabu-chip-improve    { background: #dcfce7; color: #15803d; border-color: #bbf7d0; }
.tabu-chip-admissible { background: rgba(13,148,136,0.08); color: #0f766e; border-color: rgba(13,148,136,0.25); }
.tabu-chip-aspiration { background: #fef3c7; color: #92400e; border-color: #fde68a; }

/* Sparkline */
.tabu-spark {
  width: 100%; height: 54px; display: block;
  border: 1px solid var(--line); border-radius: 6px; background: var(--panel);
}
.tabu-spark-idle { font-size: 9px; fill: var(--muted); }

/* Stats */
.tabu-statgrid { display: grid; grid-template-columns: repeat(6, 1fr); gap: 8px; }
@media (max-width: 540px) { .tabu-statgrid { grid-template-columns: repeat(3, 1fr); } }
.tabu-statlabel {
  font-size: 0.65rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}

/* Config */
.tabu-config { display: flex; flex-direction: column; gap: 8px; }
.tabu-config-row { display: flex; flex-direction: column; gap: 3px; }
.tabu-config-label { font-size: 0.88rem; }
.tabu-slider { width: 100%; accent-color: var(--accent); cursor: pointer; }
.tabu-hint { font-size: 0.75rem; color: var(--muted); }

/* Controls */
.tabu-controls { display: flex; gap: 8px; }
.tabu-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer; font-family: inherit;
}
.tabu-btn:hover:not(:disabled) { border-color: var(--accent); }
.tabu-btn:disabled { opacity: 0.45; cursor: default; }
.tabu-btn-primary { color: var(--accent); border-color: var(--accent); }

/* Footer */
.tabu-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 8px; border-top: 1px solid var(--line); color: var(--muted);
}
`

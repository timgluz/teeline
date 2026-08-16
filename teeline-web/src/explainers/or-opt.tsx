import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  CITIES, N_CITIES, SCENARIOS,
  tourLength, makeInitState, stepOnce,
} from "./or-opt-algo"
import type { Phase, MoveEvent, Scenario } from "./or-opt-algo"

const DEFAULT_SCENARIO = SCENARIOS.single_segment
const SPEEDS = [600, 420, 280, 180, 100, 50, 25]
const SPEED_LABELS = ["1x", "2x", "3x", "4x", "5x", "6x", "7x"]

function edgeKey(a: number, b: number): string {
  return `${Math.min(a, b)}-${Math.max(a, b)}`
}

function midTick(x1: number, y1: number, x2: number, y2: number): [number, number, number, number] {
  const dx = x2 - x1
  const dy = y2 - y1
  const len = Math.hypot(dx, dy) || 1
  const mx = (x1 + x2) / 2
  const my = (y1 + y2) / 2
  const px = (-dy / len) * 6
  const py = (dx / len) * 6
  return [mx - px, my - py, mx + px, my + py]
}

// ---------------------------------------------------------------
// TourCanvas — current tour with segment/cut/paste highlights,
// faint scan ticks at improving insertion gaps, and a CSS morph
// that makes the tour flow to its new shape when a move applies.
// ---------------------------------------------------------------
function TourCanvas({ tour, pending, lastMove, phase, morph }: {
  tour: number[]
  pending: MoveEvent | null
  lastMove: MoveEvent | null
  phase: Phase
  morph: boolean
}) {
  const showCandidate = phase === 'candidate' && pending
  const showApplied = phase === 'move_applied' && lastMove

  // The move currently being displayed: the proposed candidate, or the one
  // just applied.
  const activeMove = useMemo<MoveEvent | null>(() => {
    if (showCandidate) return pending
    if (showApplied) return lastMove
    return null
  }, [pending, lastMove, showCandidate, showApplied])

  const cutSet = useMemo(() => {
    if (!activeMove) return new Set<string>()
    return new Set(activeMove.cutEdges.map(([a, b]) => edgeKey(a, b)))
  }, [activeMove])
  const pasteSet = useMemo(() => {
    if (!activeMove) return new Set<string>()
    return new Set(activeMove.pasteEdges.map(([a, b]) => edgeKey(a, b)))
  }, [activeMove])

  const segSet = useMemo(() => {
    if (!activeMove) return new Set<number>()
    return new Set(activeMove.segCities)
  }, [activeMove])

  const edges: Array<{ from: number; to: number; key: string }> = []
  for (let k = 0; k < tour.length; k++) {
    const a = tour[k]
    const b = tour[(k + 1) % tour.length]
    edges.push({ from: a, to: b, key: edgeKey(a, b) })
  }

  const ticks: Array<{ key: string; x1: number; y1: number; x2: number; y2: number }> = []
  if (showCandidate && pending) {
    for (const j of pending.improvingGaps) {
      if (j === pending.j) continue // the best gap gets the strong paste highlight
      const a = tour[j]
      const b = tour[(j + 1) % tour.length]
      const [x1, y1, x2, y2] = midTick(CITIES[a][0], CITIES[a][1], CITIES[b][0], CITIES[b][1])
      ticks.push({ key: `t${j}`, x1, y1, x2, y2 })
    }
  }

  return (
    <svg viewBox="0 0 300 300"
      className={`or-canvas ${morph && showApplied ? 'or-morph' : ''}`}
      role="img" aria-label="Or-opt tour">
      <rect x={0} y={0} width={300} height={300} className="or-bg" />

      {/* Faint ticks at every improving insertion gap (the scan) */}
      {ticks.map((t) => (
        <line key={t.key} className="or-scan-tick" x1={t.x1} y1={t.y1} x2={t.x2} y2={t.y2} />
      ))}

      {/* Best insertion gap marker */}
      {showCandidate && pending && (
        <line className="or-gap-marker"
          x1={CITIES[pending.insertAfter][0]} y1={CITIES[pending.insertAfter][1]}
          x2={CITIES[pending.insertBefore][0]} y2={CITIES[pending.insertBefore][1]} />
      )}

      <g className="or-tour">
        {edges.map(({ from, to, key }) => {
          let cls = 'or-edge'
          if (cutSet.has(key)) cls += showCandidate ? ' or-cand-cut' : ' or-cut'
          else if (pasteSet.has(key)) cls += showCandidate ? ' or-cand-paste' : ' or-paste'
          return <line key={key} className={cls}
            x1={CITIES[from][0]} y1={CITIES[from][1]}
            x2={CITIES[to][0]} y2={CITIES[to][1]} />
        })}
        {tour.map((id) => (
          <g key={id}>
            <circle className={`or-city ${segSet.has(id) ? 'or-city-seg' : ''}`}
              cx={CITIES[id][0]} cy={CITIES[id][1]} r={segSet.has(id) ? 8 : 6} />
            <text className="or-label"
              x={CITIES[id][0]} y={CITIES[id][1] + 16}>{id}</text>
          </g>
        ))}
      </g>
    </svg>
  )
}

// ---------------------------------------------------------------
// Cost sparkline (same pattern as 2-opt / ACO)
// ---------------------------------------------------------------
function Sparkline({ values }: { values: number[] }) {
  if (values.length < 2) return null
  const W = 300, H = 46
  const minV = Math.min(...values)
  const maxV = Math.max(...values)
  const range = maxV - minV || 1
  const pts = values.map((v, i) => {
    const x = (i / (values.length - 1)) * W
    const y = H - ((v - minV) / range) * (H - 4) - 2
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="or-spark" aria-label="cost over passes">
      <rect x={0} y={0} width={W} height={H} className="or-bg" rx={4} />
      <polyline points={pts.join(" ")} fill="none" stroke="#0d9488"
        strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function OrOptExplainer() {
  const [tour, setTour] = useState<number[]>(() => [...DEFAULT_SCENARIO.tour])
  const [phase, setPhase] = useState<Phase>('idle')
  const [pass, setPass] = useState(0)
  const [moves, setMoves] = useState(0)
  const [bestCost, setBestCost] = useState(() => tourLength(DEFAULT_SCENARIO.tour))
  const [pending, setPending] = useState<MoveEvent | null>(null)
  const [lastMove, setLastMove] = useState<MoveEvent | null>(null)
  const [costHistory, setCostHistory] = useState<number[]>(() => [tourLength(DEFAULT_SCENARIO.tour)])
  const [step, setStep] = useState(0)
  const [running, setRunning] = useState(false)
  const [speedIdx, setSpeedIdx] = useState(2) // 3x default
  const [morphOn, setMorphOn] = useState(false) // animate the tour flow on apply, not on Back

  const simRef = useRef(makeInitState(DEFAULT_SCENARIO.tour))
  const historyRef = useRef<Array<typeof simRef.current>>([])
  const scenarioRef = useRef<Scenario>(DEFAULT_SCENARIO)

  const reinit = useCallback((scenario: Scenario) => {
    scenarioRef.current = scenario
    simRef.current = makeInitState(scenario.tour)
    historyRef.current = []
    setTour([...simRef.current.tour])
    setPhase('idle')
    setPass(0)
    setMoves(0)
    setBestCost(simRef.current.bestCost)
    setPending(null)
    setLastMove(null)
    setCostHistory([simRef.current.bestCost])
    setStep(0)
    setRunning(false)
    setMorphOn(false)
  }, [])

  const stepForward = useCallback(() => {
    // Guard against an in-flight interval tick after the run already finished:
    // stepping a local_optimum state would push a phantom undo entry.
    if (simRef.current.phase === 'local_optimum') return
    historyRef.current.push(structuredClone(simRef.current))
    const next = stepOnce(simRef.current)
    simRef.current = next
    setTour([...next.tour])
    setPhase(next.phase)
    setPass(next.pass)
    setMoves(next.moves)
    setBestCost(next.bestCost)
    setPending(next.pending)
    setLastMove(next.lastMove)
    setCostHistory([...next.costHistory])
    setStep(next.step)
    setMorphOn(next.phase === 'move_applied')
    if (next.phase === 'local_optimum') setRunning(false)
  }, [])

  const stepBack = useCallback(() => {
    const h = historyRef.current
    if (h.length === 0 || running) return
    const prev = h.pop()!
    simRef.current = prev
    setTour([...prev.tour])
    setPhase(prev.phase)
    setPass(prev.pass)
    setMoves(prev.moves)
    setBestCost(prev.bestCost)
    setPending(prev.pending)
    setLastMove(prev.lastMove)
    setCostHistory([...prev.costHistory])
    setStep(prev.step)
    setMorphOn(false) // Back restores instantly
  }, [running])

  useEffect(() => {
    if (!running) return
    const ms = SPEEDS[speedIdx]
    const id = setInterval(stepForward, ms)
    return () => clearInterval(id)
  }, [running, speedIdx, stepForward])

  const distance = tourLength(tour)

  // Status chip
  let chipText = "Click Step to scan for the best Or-1/2/3 relocation"
  let chipClass = "or-chip or-chip-idle"
  if (phase === 'candidate' && pending) {
    const rev = pending.reversed ? ' ↺ reversed' : ''
    chipText = `Candidate — move ${pending.segCities.join('→')} (k=${pending.segLen}) after ${pending.insertAfter}${rev}  (Δ=${pending.delta.toFixed(0)})`
    chipClass = "or-chip or-chip-candidate"
  } else if (phase === 'move_applied' && lastMove) {
    const rev = lastMove.reversed ? ' ↺ reversed' : ''
    chipText = `Moved — ${lastMove.segCities.join('→')} relocated after ${lastMove.insertAfter}${rev}  (Δ=${lastMove.delta.toFixed(0)})`
    chipClass = "or-chip or-chip-applied"
  } else if (phase === 'local_optimum') {
    chipText = `Local optimum — no improving Or-1/2/3 relocation exists (${moves} moves, ${pass} passes)`
    chipClass = "or-chip or-chip-done"
  }

  return (
    <div className="or-root">
      <style>{CSS}</style>

      <header className="or-header">
        <div className="or-eyebrow">teeline · algorithms/or_opt</div>
        <h2 className="or-title">Or-opt Local Search</h2>
        <p className="or-sub">
          Or-opt relocates <strong>segments of 1–3 consecutive cities</strong> to a better position
          elsewhere in the tour — a <em>cut-and-paste</em> move, unlike 2-opt's edge reversal.
          Each pass scans every segment size (Or-1, Or-2, Or-3) and every insertion point, applying
          the single <strong>best-improving</strong> relocation (reversed insertions are also tried
          for Or-2/Or-3). Repeats until no relocation improves the tour.
        </p>
      </header>

      <div className="or-viz-row">
        <div className="or-canvas-wrap">
          <TourCanvas tour={tour} pending={pending} lastMove={lastMove} phase={phase} morph={morphOn} />
        </div>
      </div>

      <div className="or-legend">
        <span><span className="or-swatch or-swatch-normal" /> tour edge</span>
        <span><span className="or-swatch or-swatch-seg" /> segment (relocating)</span>
        <span><span className="or-swatch or-swatch-cut" /> cut edge</span>
        <span><span className="or-swatch or-swatch-paste" /> paste edge</span>
        <span><span className="or-swatch or-swatch-gap" /> insertion gap</span>
        <span><span className="or-swatch or-swatch-tick" /> improving gap (scan)</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="or-section-label">Cost over passes</div>
      <Sparkline values={costHistory} />

      <div className="or-statgrid">
        <div>
          <div className="or-statlabel">pass</div>
          <div className="or-mono">{pass}</div>
        </div>
        <div>
          <div className="or-statlabel">moves</div>
          <div className="or-mono">{moves}</div>
        </div>
        <div>
          <div className="or-statlabel">best cost</div>
          <div className="or-mono">{bestCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="or-statlabel">distance</div>
          <div className="or-mono">{distance.toFixed(0)}</div>
        </div>
        <div>
          <div className="or-statlabel">last Δ</div>
          <div className="or-mono" style={{ color: lastMove && lastMove.delta < 0 ? '#16a34a' : 'inherit' }}>
            {lastMove ? lastMove.delta.toFixed(0) : '—'}
          </div>
        </div>
        <div>
          <div className="or-statlabel">step</div>
          <div className="or-mono">{step}</div>
        </div>
      </div>

      <div className="or-config">
        <div className="or-config-row">
          <label className="or-label">Speed</label>
          <div className="or-speed-btns">
            {SPEED_LABELS.map((l, i) => (
              <button key={l}
                className={`or-speed-btn ${i === speedIdx ? 'or-speed-btn-sel' : ''}`}
                onClick={() => setSpeedIdx(i)} disabled={running}>
                {l}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="or-controls">
        <button className="or-btn" onClick={stepBack} disabled={running || historyRef.current.length === 0}>
          ⏴ Back
        </button>
        <button className="or-btn" onClick={stepForward} disabled={running || phase === 'local_optimum'}>
          ⏵ Step
        </button>
        <button className="or-btn" onClick={() => setRunning(!running)} disabled={phase === 'local_optimum'}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="or-btn" onClick={() => reinit(scenarioRef.current)} disabled={running}>↺ Reset</button>
      </div>

      <div className="or-scenarios">
        <div className="or-section-label">Scenarios</div>
        <div className="or-scenario-row">
          {Object.entries(SCENARIOS).map(([key, s]) => (
            <button key={key} className="or-scenario-btn" title={s.desc}
              onClick={() => reinit(s)} disabled={running}>
              {s.label}
            </button>
          ))}
        </div>
      </div>

      <footer className="or-footer">
        <span className="or-mono">cities: {N_CITIES}</span>
        <span className="or-mono">Or-1/2/3 relocation · best-improvement · runs to local optimum</span>
      </footer>
    </div>
  )
}

const CSS = `
.or-root {
  --accent: #0d9488;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --improve: #16a34a;
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
.or-root * { box-sizing: border-box; }
.or-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

.or-header { margin-bottom: 2px; }
.or-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.or-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.or-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }

.or-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.or-bg { fill: var(--panel); }
.or-edge { stroke: #94a3b8; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.or-cand-cut { stroke: #ea580c; stroke-width: 3.5; stroke-dasharray: 6 4; }
.or-cand-paste { stroke: #16a34a; stroke-width: 3.5; stroke-dasharray: 4 6; }
.or-cut { stroke: #ef4444; stroke-width: 3.5; stroke-dasharray: 6 3; }
.or-paste { stroke: #16a34a; stroke-width: 3.5; }
.or-scan-tick { stroke: #f59e0b; stroke-width: 2; stroke-dasharray: 2 2; opacity: 0.5; }
.or-gap-marker { stroke: #16a34a; stroke-width: 2.5; stroke-dasharray: 5 4; opacity: 0.55; }
.or-city { fill: #1f2937; stroke: #fff; stroke-width: 1.5; transition: cx 0.35s ease, cy 0.35s ease; }
.or-city-seg { fill: #ea580c; stroke: #fff7ed; }
.or-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: #374151; text-anchor: middle;
  paint-order: stroke fill; stroke: white; stroke-width: 3;
  pointer-events: none; user-select: none;
}

/* Morph: when a move is applied, the whole tour flows to its new shape. */
.or-morph .or-edge,
.or-morph .or-cut,
.or-morph .or-paste,
.or-morph .or-cand-cut,
.or-morph .or-cand-paste {
  transition: x1 0.35s ease, y1 0.35s ease, x2 0.35s ease, y2 0.35s ease;
}

.or-viz-row { display: flex; gap: 10px; align-items: stretch; }
.or-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

.or-legend {
  display: flex; flex-wrap: wrap; gap: 6px 14px; font-size: 0.78rem; color: var(--muted);
}
.or-swatch {
  display: inline-block; width: 14px; height: 3px; border-radius: 2px;
  margin-right: 4px; vertical-align: middle;
}
.or-swatch-normal { background: #94a3b8; }
.or-swatch-seg { background: #ea580c; }
.or-swatch-cut { background: #ef4444; }
.or-swatch-paste { background: #16a34a; }
.or-swatch-gap { background: #16a34a; }
.or-swatch-tick { background: #f59e0b; }

.or-chip {
  font-size: 0.82rem; padding: 6px 12px; border-radius: 8px;
  font-weight: 500; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.or-chip-idle { background: #f1f5f9; color: var(--muted); }
.or-chip-candidate { background: #ffedd5; color: #9a3412; }
.or-chip-applied { background: #dcfce7; color: #166534; }
.or-chip-done { background: #f1f5f9; color: var(--muted); }

.or-section-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }
.or-spark { width: 100%; display: block; border-radius: 6px; }

.or-statgrid {
  display: flex; flex-wrap: wrap; gap: 14px 22px;
  font-size: 0.82rem;
}
.or-statlabel { color: var(--muted); font-size: 0.72em; text-transform: uppercase; letter-spacing: 0.06em; }

.or-config {
  display: flex; flex-direction: column; gap: 8px;
  padding: 12px; background: var(--panel); border-radius: 8px;
}
.or-config-row {
  display: flex; align-items: center; gap: 10px; flex-wrap: wrap;
}
.or-label {
  font-size: 0.82rem; font-weight: 500; color: var(--text);
  min-width: 50px;
}
.or-speed-btns { display: flex; gap: 4px; }
.or-speed-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.or-speed-btn:hover:not(:disabled) { background: #f0fdf4; }
.or-speed-btn:disabled { opacity: 0.4; cursor: default; }
.or-speed-btn-sel { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.or-controls { display: flex; gap: 8px; }
.or-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem; padding: 6px 14px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.or-btn:hover:not(:disabled) { background: #f0fdf4; }
.or-btn:disabled { opacity: 0.4; cursor: default; }

.or-scenarios { display: flex; flex-direction: column; gap: 6px; }
.or-scenario-row { display: flex; flex-wrap: wrap; gap: 6px; }
.or-scenario-btn {
  font-size: 0.8rem; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.or-scenario-btn:hover:not(:disabled) { background: #f0fdf4; border-color: var(--accent); }
.or-scenario-btn:disabled { opacity: 0.4; cursor: default; }

.or-footer {
  display: flex; gap: 12px; justify-content: center;
  padding-top: 8px; border-top: 1px solid var(--line);
  color: var(--muted); font-size: 0.78rem;
}
`

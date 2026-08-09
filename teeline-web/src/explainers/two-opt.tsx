import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  CITIES, N_CITIES, SCENARIOS,
  tourLength, makeInitState, stepOnce,
} from "./two-opt-algo"
import type { Phase } from "./two-opt-algo"

const DEFAULT_TOUR = SCENARIOS.bad_shuffle.tour
const SPEEDS = [600, 420, 280, 180, 100, 50, 25]
const SPEED_LABELS = ["1x", "2x", "3x", "4x", "5x", "6x", "7x"]

// ---------------------------------------------------------------
// TourCanvas — current tour with highlighted swap edges
// ---------------------------------------------------------------
function TourCanvas({ tour, lastSwap, phase }: {
  tour: number[]
  lastSwap: { removed: [number, number][]; added: [number, number][] } | null
  phase: Phase
}) {
  const removeSet = useMemo(() => {
    if (!lastSwap) return new Set<string>()
    return new Set(lastSwap.removed.map(([a, b]) => `${Math.min(a, b)}-${Math.max(a, b)}`))
  }, [lastSwap])
  const addSet = useMemo(() => {
    if (!lastSwap) return new Set<string>()
    return new Set(lastSwap.added.map(([a, b]) => `${Math.min(a, b)}-${Math.max(a, b)}`))
  }, [lastSwap])

  const edges: Array<{ from: number; to: number; key: string }> = []
  for (let k = 0; k < tour.length; k++) {
    const a = tour[k]
    const b = tour[(k + 1) % tour.length]
    edges.push({ from: a, to: b, key: `${Math.min(a, b)}-${Math.max(a, b)}` })
  }

  const showCandidate = phase === 'candidate' && lastSwap
  const showApplied = phase === 'swap_found' && lastSwap

  return (
    <svg viewBox="0 0 300 300" className="topt-canvas" role="img" aria-label="2-opt tour">
      <rect x={0} y={0} width={300} height={300} className="topt-bg" />
      {edges.map(({ from, to, key }) => {
        let cls = "topt-edge"
        if (showCandidate) {
          if (removeSet.has(key)) cls += " topt-cand-removed"
          else if (addSet.has(key)) cls += " topt-cand-added"
        } else if (showApplied) {
          if (removeSet.has(key)) cls += " topt-removed"
          else if (addSet.has(key)) cls += " topt-added"
        }
        return <line key={key} className={cls}
          x1={CITIES[from][0]} y1={CITIES[from][1]}
          x2={CITIES[to][0]} y2={CITIES[to][1]} />
      })}
      {tour.map((id) => (
        <g key={id}>
          <circle className="topt-city"
            cx={CITIES[id][0]} cy={CITIES[id][1]} r={6} />
          <text className="topt-label"
            x={CITIES[id][0]} y={CITIES[id][1] + 16}>{id}</text>
        </g>
      ))}
    </svg>
  )
}

// ---------------------------------------------------------------
// Cost sparkline (same pattern as ACO)
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
    <svg viewBox={`0 0 ${W} ${H}`} className="topt-spark">
      <rect x={0} y={0} width={W} height={H} className="topt-bg" rx={4} />
      <polyline points={pts.join(" ")} fill="none" stroke="#0d9488"
        strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function TwoOptExplainer() {
  const [tour, setTour] = useState<number[]>(() => [...DEFAULT_TOUR])
  const [phase, setPhase] = useState<Phase>('idle')
  const [bestCost, setBestCost] = useState(() => tourLength(DEFAULT_TOUR))
  const [pass, setPass] = useState(0)
  const [totalSwaps, setTotalSwaps] = useState(0)
  const [lastSwap, setLastSwap] = useState<{
    i: number; j: number; removed: [number, number][]; added: [number, number][]; delta: number
  } | null>(null)
  const [costHistory, setCostHistory] = useState<number[]>(() => [tourLength(DEFAULT_TOUR)])
  const [step, setStep] = useState(0)
  const [running, setRunning] = useState(false)
  const [speedIdx, setSpeedIdx] = useState(0) // 1x = 600ms — slow enough to observe swaps

  const simRef = useRef(makeInitState(DEFAULT_TOUR))

  const reinit = useCallback((t: number[]) => {
    simRef.current = makeInitState(t)
    setTour([...t])
    setPhase('idle')
    setBestCost(simRef.current.bestCost)
    setPass(0)
    setTotalSwaps(0)
    setLastSwap(null)
    setCostHistory([simRef.current.bestCost])
    setStep(0)
    setRunning(false)
  }, [])

  const step_fn = useCallback(() => {
    const next = stepOnce(simRef.current)
    simRef.current = next
    setTour([...next.tour])
    setPhase(next.phase)
    setBestCost(next.bestCost)
    setPass(next.pass)
    setTotalSwaps(next.totalSwaps)
    setLastSwap(next.lastSwap)
    setCostHistory([...next.costHistory])
    setStep(next.step)
    if (next.phase === 'local_optimum') setRunning(false)
  }, [])

  useEffect(() => {
    if (!running) return
    const ms = SPEEDS[speedIdx]
    const id = setInterval(step_fn, ms)
    return () => clearInterval(id)
  }, [running, speedIdx, step_fn])

  let chipText = "Click Step to scan for improving swaps"
  let chipClass = "topt-chip topt-chip-idle"
  if (phase === 'candidate' && lastSwap) {
    chipText = `Candidate swap (${lastSwap.i + 1},${lastSwap.j + 1}) — Δ=${lastSwap.delta.toFixed(0)} — click Step to apply`
    chipClass = "topt-chip topt-chip-candidate"
  } else if (phase === 'swap_found' && lastSwap) {
    chipText = `Swap (${lastSwap.i + 1},${lastSwap.j + 1}) applied — Δ=${lastSwap.delta.toFixed(0)}`
    chipClass = "topt-chip topt-chip-swap"
  } else if (phase === 'local_optimum') {
    chipText = `Local optimum reached — no improving swap exists (${totalSwaps} swaps, ${pass} passes)`
    chipClass = "topt-chip topt-chip-done"
  }

  return (
    <div className="topt-root">
      <style>{CSS}</style>

      <header className="topt-header">
        <div className="topt-eyebrow">teeline · algorithms/2opt</div>
        <h2 className="topt-title">2-opt Local Search</h2>
        <p className="topt-sub">
          2-opt iteratively improves a tour by removing two edges and
          reconnecting the resulting segments in the only other valid way
          — reversing the segment between the two removed edges. Each pass
          scans all edge pairs and applies the <strong>best-improving</strong> swap;
          the algorithm stops when no improving swap exists (local optimum).
        </p>
      </header>

      <div className="topt-viz-row">
        <div className="topt-canvas-wrap">
          <TourCanvas tour={tour} lastSwap={lastSwap} phase={phase} />
        </div>
      </div>

      <div className="topt-legend">
        <span><span className="topt-swatch topt-swatch-normal" /> tour edge</span>
        <span><span className="topt-swatch topt-swatch-cand-rm" /> candidate (remove)</span>
        <span><span className="topt-swatch topt-swatch-cand-ad" /> candidate (add)</span>
        <span><span className="topt-swatch topt-swatch-removed" /> removed</span>
        <span><span className="topt-swatch topt-swatch-added" /> new edge</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="topt-section-label">Cost history</div>
      <Sparkline values={costHistory} />

      <div className="topt-statgrid">
        <div>
          <div className="topt-statlabel">pass</div>
          <div className="topt-mono">{pass}</div>
        </div>
        <div>
          <div className="topt-statlabel">swaps</div>
          <div className="topt-mono">{totalSwaps}</div>
        </div>
        <div>
          <div className="topt-statlabel">best cost</div>
          <div className="topt-mono">{bestCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="topt-statlabel">step</div>
          <div className="topt-mono">{step}</div>
        </div>
        {lastSwap && (
          <div>
            <div className="topt-statlabel">last Δ</div>
            <div className="topt-mono" style={{ color: lastSwap.delta < 0 ? "#16a34a" : "inherit" }}>
              {lastSwap.delta.toFixed(0)}
            </div>
          </div>
        )}
      </div>

      <div className="topt-config">
        <div className="topt-config-row">
          <label className="topt-label">Speed</label>
          <div className="topt-speed-btns">
            {SPEED_LABELS.map((l, i) => (
              <button key={l}
                className={`topt-speed-btn ${i === speedIdx ? 'topt-speed-btn-sel' : ''}`}
                onClick={() => setSpeedIdx(i)} disabled={running}>
                {l}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="topt-controls">
        <button className="topt-btn" onClick={step_fn} disabled={running || phase === 'local_optimum'}>
          ⏵ Step
        </button>
        <button className="topt-btn" onClick={() => setRunning(!running)} disabled={phase === 'local_optimum'}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="topt-btn" onClick={() => reinit([...tour])}>↺ Reset</button>
      </div>

      <div className="topt-scenarios">
        <div className="topt-section-label">Scenarios</div>
        <div className="topt-scenario-row">
          {Object.entries(SCENARIOS).map(([key, s]) => (
            <button key={key} className="topt-scenario-btn" title={s.desc}
              onClick={() => reinit([...s.tour])}>
              {s.label}
            </button>
          ))}
        </div>
      </div>

      <footer className="topt-footer">
        <span className="topt-mono">cities: {N_CITIES}</span>
        <span className="topt-mono">best-improvement 2-opt</span>
      </footer>
    </div>
  )
}

const CSS = `
.topt-root {
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
.topt-root * { box-sizing: border-box; }
.topt-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

.topt-header { margin-bottom: 2px; }
.topt-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.topt-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.topt-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }

.topt-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.topt-bg { fill: var(--panel); }
.topt-edge { stroke: #94a3b8; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.topt-cand-removed { stroke: #ea580c; stroke-width: 3.5; stroke-dasharray: 6 4; }
.topt-cand-added { stroke: #16a34a; stroke-width: 3.5; stroke-dasharray: 4 6; }
.topt-removed { stroke: #ef4444; stroke-width: 3.5; stroke-dasharray: 6 3; }
.topt-added { stroke: #16a34a; stroke-width: 3.5; }
.topt-city { fill: #1f2937; stroke: #fff; stroke-width: 1.5; }
.topt-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: #374151; text-anchor: middle;
  paint-order: stroke fill; stroke: white; stroke-width: 3;
  pointer-events: none; user-select: none;
}

.topt-viz-row { display: flex; gap: 10px; align-items: stretch; }
.topt-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

.topt-legend {
  display: flex; gap: 14px; font-size: 0.78rem; color: var(--muted);
}
.topt-swatch {
  display: inline-block; width: 14px; height: 3px; border-radius: 2px;
  margin-right: 4px; vertical-align: middle;
}
.topt-swatch-normal { background: #94a3b8; }
.topt-swatch-cand-rm { background: #ea580c; }
.topt-swatch-cand-ad { background: #16a34a; }
.topt-swatch-removed { background: #ef4444; }
.topt-swatch-added { background: #16a34a; }

.topt-chip {
  font-size: 0.82rem; padding: 6px 12px; border-radius: 8px;
  font-weight: 500; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.topt-chip-idle { background: #f1f5f9; color: var(--muted); }
.topt-chip-candidate { background: #ffedd5; color: #9a3412; }
.topt-chip-swap { background: #fef3c7; color: #92400e; }
.topt-chip-done { background: #dcfce7; color: #166534; }

.topt-section-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }
.topt-spark { width: 100%; display: block; border-radius: 6px; }

.topt-statgrid {
  display: flex; flex-wrap: wrap; gap: 14px 22px;
  font-size: 0.82rem;
}
.topt-statlabel { color: var(--muted); font-size: 0.72em; text-transform: uppercase; letter-spacing: 0.06em; }

.topt-config {
  display: flex; flex-direction: column; gap: 8px;
  padding: 12px; background: var(--panel); border-radius: 8px;
}
.topt-config-row {
  display: flex; align-items: center; gap: 10px; flex-wrap: wrap;
}
.topt-label {
  font-size: 0.82rem; font-weight: 500; color: var(--text);
  min-width: 50px;
}
.topt-speed-btns { display: flex; gap: 4px; }
.topt-speed-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.topt-speed-btn:hover:not(:disabled) { background: #f0fdf4; }
.topt-speed-btn:disabled { opacity: 0.4; cursor: default; }
.topt-speed-btn-sel { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.topt-controls { display: flex; gap: 8px; }
.topt-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem; padding: 6px 14px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.topt-btn:hover:not(:disabled) { background: #f0fdf4; }
.topt-btn:disabled { opacity: 0.4; cursor: default; }

.topt-scenarios { display: flex; flex-direction: column; gap: 6px; }
.topt-scenario-row { display: flex; flex-wrap: wrap; gap: 6px; }
.topt-scenario-btn {
  font-size: 0.8rem; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.topt-scenario-btn:hover { background: #f0fdf4; border-color: var(--accent); }

.topt-footer {
  display: flex; gap: 12px; justify-content: center;
  padding-top: 8px; border-top: 1px solid var(--line);
  color: var(--muted); font-size: 0.78rem;
}
`

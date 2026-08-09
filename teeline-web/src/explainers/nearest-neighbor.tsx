import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  CITIES, N_CITIES, SCENARIOS,
  tourLength, makeInitState, stepOnce,
} from "./nearest-neighbor-algo"
import type { EventMode, Candidate } from "./nearest-neighbor-algo"

const SPEEDS = [600, 420, 280, 180, 100, 50, 25]
const SPEED_LABELS = ["1x", "2x", "3x", "4x", "5x", "6x", "7x"]

const DEFAULT_START = SCENARIOS.balanced.startCity

// ---------------------------------------------------------------
// NNCanvas — tour edges, visited/unvisited cities, candidate edges
// ---------------------------------------------------------------
function NNCanvas({ tour, candidateDists, done, lastEvent }: {
  tour: number[]
  candidateDists: Candidate[] | null
  done: boolean
  lastEvent: EventMode | null
}) {
  const visitedSet = useMemo(() => new Set(tour), [tour])
  const current = tour.length > 0 ? tour[tour.length - 1] : -1
  const orderMap = useMemo(() => {
    const m = new Map<number, number>()
    // Don't index the closing duplicate of start
    for (let i = 0; i < Math.min(tour.length, N_CITIES); i++) {
      m.set(tour[i], i + 1)
    }
    return m
  }, [tour])

  // Tour edges (from tour array, excluding the last if done since it closes)
  const edges: Array<[number, number]> = []
  for (let k = 0; k < tour.length - 1; k++) {
    edges.push([tour[k], tour[k + 1]])
  }

  // Candidate edges from current city to unvisited
  const showCandidates = !done && candidateDists && candidateDists.length > 0

  return (
    <svg viewBox="0 0 300 300" className="nn-canvas" role="img" aria-label="Nearest Neighbor tour">
      <rect x={0} y={0} width={300} height={300} className="nn-bg" />

      {/* candidate edges — faint dashed from current to all unvisited */}
      {showCandidates && candidateDists!.map((c) => (
        <line key={c.city}
          className={c === candidateDists![0] ? "nn-cand-best" : "nn-cand-edge"}
          x1={CITIES[current][0]} y1={CITIES[current][1]}
          x2={CITIES[c.city][0]} y2={CITIES[c.city][1]} />
      ))}

      {/* tour edges — solid green for visited path */}
      {edges.map(([a, b], i) => {
        const isLast = !done && i === edges.length - 1 && lastEvent === 'visited'
        return (
          <line key={`${a}-${b}`}
            className={isLast ? "nn-edge-last" : "nn-edge"}
            x1={CITIES[a][0]} y1={CITIES[a][1]}
            x2={CITIES[b][0]} y2={CITIES[b][1]} />
        )
      })}

      {/* closing edge */}
      {done && tour.length >= 2 && (() => {
        const last = tour[tour.length - 1]
        const prev = tour[tour.length - 2]
        return (
          <line key="closing"
            className="nn-edge-closing"
            x1={CITIES[prev][0]} y1={CITIES[prev][1]}
            x2={CITIES[last][0]} y2={CITIES[last][1]} />
        )
      })()}

      {/* cities */}
      {Array.from({ length: N_CITIES }, (_, i) => {
        const isVisited = visitedSet.has(i) && orderMap.has(i)
        const isCurrent = i === current && !done
        const order = orderMap.get(i)

        let cls = "nn-city"
        if (isCurrent) cls += " nn-city-current"
        else if (isVisited) cls += " nn-city-visited"
        else cls += " nn-city-unvisited"

        return (
          <g key={i}>
            <circle className={cls} cx={CITIES[i][0]} cy={CITIES[i][1]} r={6} />
            <text className="nn-label"
              x={CITIES[i][0]} y={CITIES[i][1] + 16}>{i}</text>
            {order && (
              <text className="nn-order"
                x={CITIES[i][0]} y={CITIES[i][1] + 3}>{order}</text>
            )}
          </g>
        )
      })}
    </svg>
  )
}

// ---------------------------------------------------------------
// Sparkline
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
    <svg viewBox={`0 0 ${W} ${H}`} className="nn-spark">
      <rect x={0} y={0} width={W} height={H} className="nn-bg" rx={4} />
      <polyline points={pts.join(" ")} fill="none" stroke="#0d9488"
        strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function NNExplainer() {
  const [tour, setTour] = useState<number[]>(() => [DEFAULT_START])
  const [unvisited, setUnvisited] = useState<number[]>(() => {
    const arr: number[] = []
    for (let i = 0; i < N_CITIES; i++) if (i !== DEFAULT_START) arr.push(i)
    return arr
  })
  const [done, setDone] = useState(false)
  const [lastEvent, setLastEvent] = useState<EventMode | null>(null)
  const [lastCity, setLastCity] = useState<number | null>(null)
  const [lastDist, setLastDist] = useState(0)
  const [candidateDists, setCandidateDists] = useState<Candidate[] | null>(null)
  const [step, setStep] = useState(0)
  const [running, setRunning] = useState(false)
  const [speedIdx, setSpeedIdx] = useState(0)
  const [costHistory, setCostHistory] = useState<number[]>([])

  const simRef = useRef(makeInitState(DEFAULT_START))
  const historyRef = useRef<Array<typeof simRef.current>>([])

  const reinit = useCallback((startCity: number) => {
    simRef.current = makeInitState(startCity)
    historyRef.current = []
    setTour([startCity])
    setUnvisited([...simRef.current.unvisited])
    setDone(false)
    setLastEvent(null)
    setLastCity(null)
    setLastDist(0)
    setCandidateDists(null)
    setStep(0)
    setRunning(false)
    setCostHistory([])
  }, [])

  const step_fn = useCallback(() => {
    historyRef.current.push(structuredClone(simRef.current))
    const next = stepOnce(simRef.current)
    simRef.current = next
    setTour([...next.tour])
    setUnvisited([...next.unvisited])
    setDone(next.done)
    setLastEvent(next.lastEvent)
    setLastCity(next.lastCity)
    setLastDist(next.lastDist)
    setCandidateDists(next.candidateDists ? [...next.candidateDists] : null)
    setStep(next.step)
    if (next.lastEvent === 'visited' || next.lastEvent === 'closing') {
      setCostHistory(h => [...h, tourLength(next.tour)])
    }
    if (next.done) setRunning(false)
  }, [])

  const stepBack = useCallback(() => {
    const h = historyRef.current
    if (h.length === 0 || running) return
    const prev = h.pop()!
    simRef.current = prev
    setTour([...prev.tour])
    setUnvisited([...prev.unvisited])
    setDone(prev.done)
    setLastEvent(prev.lastEvent)
    setLastCity(prev.lastCity)
    setLastDist(prev.lastDist)
    setCandidateDists(prev.candidateDists ? [...prev.candidateDists] : null)
    setStep(prev.step)
  }, [running])

  useEffect(() => {
    if (!running) return
    const id = setInterval(step_fn, SPEEDS[speedIdx])
    return () => clearInterval(id)
  }, [running, speedIdx, step_fn])

  const currentDist = tour.length >= 2 ? tourLength(tour) : 0

  let chipText = "Click Step to start — NN picks the closest unvisited city"
  let chipClass = "nn-chip nn-chip-idle"
  if (lastEvent === 'visited' && lastCity !== null && candidateDists && candidateDists.length > 0) {
    const next = candidateDists[0]
    chipText = `NN picks city ${lastCity} (dist=${lastDist.toFixed(0)}) — next closest: ${next.city} at ${next.dist.toFixed(0)}`
    chipClass = "nn-chip nn-chip-visit"
  } else if (lastEvent === 'visited' && lastCity !== null && (!candidateDists || candidateDists.length === 0)) {
    chipText = `Last city ${lastCity} visited (dist=${lastDist.toFixed(0)}) — closing to start`
    chipClass = "nn-chip nn-chip-visit"
  } else if (lastEvent === 'closing') {
    chipText = `Tour closed — final edge  ${lastDist.toFixed(0)}  — total distance  ${currentDist.toFixed(0)}`
    chipClass = "nn-chip nn-chip-done"
  }

  return (
    <div className="nn-root">
      <style>{CSS}</style>

      <header className="nn-header">
        <div className="nn-eyebrow">teeline · algorithms/nn</div>
        <h2 className="nn-title">Nearest Neighbor Construction</h2>
        <p className="nn-sub">
          The simplest constructive TSP heuristic: start from a chosen city, repeatedly
          move to the <strong>closest unvisited city</strong>, then close the tour back to
          the start. Each step is a single greedy decision — easy to follow, but the
          final tour can be far from optimal.
        </p>
      </header>

      <div className="nn-viz-row">
        <div className="nn-canvas-wrap">
          <NNCanvas
            tour={tour}
            candidateDists={candidateDists} done={done} lastEvent={lastEvent}
          />
        </div>
      </div>

      <div className="nn-legend">
        <span><span className="nn-swatch nn-swatch-path" /> visited path</span>
        <span><span className="nn-swatch nn-swatch-last" /> last edge</span>
        <span><span className="nn-swatch nn-swatch-cand" /> candidate edge</span>
        <span><span className="nn-swatch nn-swatch-best" /> nearest candidate</span>
        <span><span className="nn-swatch nn-swatch-curr" /> current city</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="nn-statgrid">
        <div>
          <div className="nn-statlabel">visited</div>
          <div className="nn-mono">{new Set(tour).size - (done ? 1 : 0)}/{N_CITIES}</div>
        </div>
        <div>
          <div className="nn-statlabel">remaining</div>
          <div className="nn-mono">{unvisited.length}</div>
        </div>
        <div>
          <div className="nn-statlabel">distance</div>
          <div className="nn-mono">{currentDist.toFixed(0)}</div>
        </div>
        <div>
          <div className="nn-statlabel">step</div>
          <div className="nn-mono">{step}</div>
        </div>
      </div>

      {costHistory.length > 0 ? (
        <>
          <div className="nn-section-label">Distance per step</div>
          <Sparkline values={costHistory} />
        </>
      ) : null}

      <div className="nn-config">
        <div className="nn-config-row">
          <label className="nn-label">Speed</label>
          <div className="nn-speed-btns">
            {SPEED_LABELS.map((l, i) => (
              <button key={l}
                className={`nn-speed-btn ${i === speedIdx ? 'nn-speed-btn-sel' : ''}`}
                onClick={() => setSpeedIdx(i)} disabled={running}>
                {l}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="nn-controls">
        <button className="nn-btn" onClick={stepBack} disabled={running || historyRef.current.length === 0}>
          ⏴ Back
        </button>
        <button className="nn-btn" onClick={step_fn} disabled={running || done}>
          ⏵ Step
        </button>
        <button className="nn-btn" onClick={() => setRunning(!running)} disabled={done}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="nn-btn" onClick={() => reinit(tour[0])}>↺ Reset</button>
      </div>

      <div className="nn-scenarios">
        <div className="nn-section-label">Scenarios</div>
        <div className="nn-scenario-row">
          {Object.entries(SCENARIOS).map(([key, s]) => (
            <button key={key} className="nn-scenario-btn" title={s.desc}
              onClick={() => reinit(s.startCity)}>
              {s.label}
            </button>
          ))}
        </div>
      </div>

      <footer className="nn-footer">
        <span className="nn-mono">cities: {N_CITIES}</span>
        <span className="nn-mono">{done ? `total: ${currentDist.toFixed(0)}` : `partial: ${currentDist.toFixed(0)}`}</span>
        <span className="nn-mono">greedy nearest neighbor</span>
      </footer>
    </div>
  )
}

const CSS = `
.nn-root {
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
.nn-root * { box-sizing: border-box; }
.nn-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

.nn-header { margin-bottom: 2px; }
.nn-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.nn-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.nn-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }

.nn-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.nn-bg { fill: var(--panel); }

.nn-cand-edge { stroke: #cbd5e1; stroke-width: 1.2; stroke-dasharray: 4 5; }
.nn-cand-best { stroke: #f59e0b; stroke-width: 2.2; stroke-dasharray: 3 3; }
.nn-edge { stroke: #94a3b8; stroke-width: 2; stroke-linecap: round; }
.nn-edge-last { stroke: #16a34a; stroke-width: 3; stroke-linecap: round; }
.nn-edge-closing { stroke: #2563eb; stroke-width: 3.2; stroke-linecap: round; stroke-dasharray: 5 3; }

.nn-city { stroke: #fff; stroke-width: 1.5; }
.nn-city-visited { fill: #16a34a; }
.nn-city-current { fill: #0d9488; stroke: #f59e0b; stroke-width: 2.5; }
.nn-city-unvisited { fill: #94a3b8; opacity: 0.5; }

.nn-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: #374151; text-anchor: middle;
  paint-order: stroke fill; stroke: white; stroke-width: 3;
  pointer-events: none; user-select: none;
}
.nn-order {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 7px; fill: #fff; text-anchor: middle; font-weight: 700;
  pointer-events: none; user-select: none;
}

.nn-viz-row { display: flex; gap: 10px; align-items: stretch; }
.nn-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

.nn-legend {
  display: flex; gap: 12px; font-size: 0.78rem; color: var(--muted); flex-wrap: wrap;
}
.nn-swatch {
  display: inline-block; width: 14px; height: 3px; border-radius: 2px;
  margin-right: 4px; vertical-align: middle;
}
.nn-swatch-path { background: #94a3b8; }
.nn-swatch-last { background: #16a34a; }
.nn-swatch-cand { background: #cbd5e1; }
.nn-swatch-best { background: #f59e0b; }
.nn-swatch-curr { display: inline-block; width: 10px; height: 10px; border-radius: 50%;
  background: #0d9488; border: 1.5px solid #f59e0b; }

.nn-chip {
  font-size: 0.82rem; padding: 6px 12px; border-radius: 8px;
  font-weight: 500; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.nn-chip-idle { background: #f1f5f9; color: var(--muted); }
.nn-chip-visit { background: #dcfce7; color: #166534; }
.nn-chip-done { background: #dbeafe; color: #1e40af; }

.nn-section-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }
.nn-spark { width: 100%; display: block; border-radius: 6px; }

.nn-statgrid {
  display: flex; flex-wrap: wrap; gap: 14px 22px; font-size: 0.82rem;
}
.nn-statlabel { color: var(--muted); font-size: 0.72em; text-transform: uppercase; letter-spacing: 0.06em; }

.nn-config {
  display: flex; flex-direction: column; gap: 8px;
  padding: 12px; background: var(--panel); border-radius: 8px;
}
.nn-config-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.nn-label { font-size: 0.82rem; font-weight: 500; color: var(--text); min-width: 50px; }
.nn-speed-btns { display: flex; gap: 4px; }
.nn-speed-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.nn-speed-btn:hover:not(:disabled) { background: #f0fdf4; }
.nn-speed-btn:disabled { opacity: 0.4; cursor: default; }
.nn-speed-btn-sel { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.nn-controls { display: flex; gap: 8px; }
.nn-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem; padding: 6px 14px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.nn-btn:hover:not(:disabled) { background: #f0fdf4; }
.nn-btn:disabled { opacity: 0.4; cursor: default; }

.nn-scenarios { display: flex; flex-direction: column; gap: 6px; }
.nn-scenario-row { display: flex; flex-wrap: wrap; gap: 6px; }
.nn-scenario-btn {
  font-size: 0.8rem; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.nn-scenario-btn:hover { background: #f0fdf4; border-color: var(--accent); }

.nn-footer {
  display: flex; gap: 12px; justify-content: center;
  padding-top: 8px; border-top: 1px solid var(--line);
  color: var(--muted); font-size: 0.78rem;
}
`

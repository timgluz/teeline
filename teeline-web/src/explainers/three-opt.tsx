import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  CITIES, N_CITIES, SCENARIOS, CASE_LABELS,
  tourLength, makeInitState, stepOnce,
} from "./three-opt-algo"
import type { Phase, MoveEvent, Scenario, CaseNo } from "./three-opt-algo"

const DEFAULT_SCENARIO = SCENARIOS.single_3opt
const SPEEDS = [600, 420, 280, 180, 100, 50, 25]
const SPEED_LABELS = ["1x", "2x", "3x", "4x", "5x", "6x", "7x"]
const CASES: CaseNo[] = [1, 2, 3, 4, 5, 6, 7]

function edgeKey(a: number, b: number): string {
  return `${Math.min(a, b)}-${Math.max(a, b)}`
}

// ---------------------------------------------------------------
// Reconnection pattern diagram — the 7 ways to reconnect the three
// segments (a "truth table" of which pairs get joined). Nodes A–F sit
// on a hexagon; removed edges (AB, CD, EF) are faint, the case's three
// new edges are orange. The active case is highlighted.
// ---------------------------------------------------------------
const NODE_POS: Record<string, [number, number]> = {
  A: [24, 4],
  B: [41, 13],
  C: [41, 27],
  D: [24, 36],
  E: [7, 27],
  F: [7, 13],
}
const REMOVED_LABEL_PAIRS: [string, string][] = [['A', 'B'], ['C', 'D'], ['E', 'F']]

function caseLabelPairs(caseNo: CaseNo): [string, string][] {
  switch (caseNo) {
    case 1: return [['A', 'C'], ['B', 'D'], ['E', 'F']]
    case 2: return [['A', 'B'], ['C', 'E'], ['D', 'F']]
    case 3: return [['A', 'C'], ['B', 'E'], ['D', 'F']]
    case 4: return [['A', 'D'], ['E', 'B'], ['C', 'F']]
    case 5: return [['A', 'D'], ['E', 'C'], ['B', 'F']]
    case 6: return [['A', 'E'], ['D', 'B'], ['C', 'F']]
    case 7: return [['A', 'E'], ['D', 'C'], ['B', 'F']]
  }
}

function PatternCell({ caseNo, active }: { caseNo: CaseNo; active: boolean }) {
  const pairs = caseLabelPairs(caseNo)
  return (
    <div className={`t3-pattern-cell ${active ? 't3-pattern-active' : ''}`} title={CASE_LABELS[caseNo]}>
      <svg viewBox="0 0 48 40" className="t3-pattern-svg" aria-label={`case ${caseNo}: ${CASE_LABELS[caseNo]}`}>
        {REMOVED_LABEL_PAIRS.map(([a, b]) => (
          <line key={`r${a}${b}`} className="t3-pattern-removed"
            x1={NODE_POS[a][0]} y1={NODE_POS[a][1]}
            x2={NODE_POS[b][0]} y2={NODE_POS[b][1]} />
        ))}
        {pairs.map(([a, b]) => (
          <line key={`n${a}${b}`} className="t3-pattern-new"
            x1={NODE_POS[a][0]} y1={NODE_POS[a][1]}
            x2={NODE_POS[b][0]} y2={NODE_POS[b][1]} />
        ))}
        {Object.entries(NODE_POS).map(([label, [x, y]]) => (
          <text key={label} className="t3-pattern-node" x={x} y={y}>{label}</text>
        ))}
      </svg>
      <div className="t3-pattern-case">{caseNo}</div>
    </div>
  )
}

// ---------------------------------------------------------------
// TourCanvas — the current tour with 3 removed / 3 added edges and a
// CSS morph that makes the tour flow to its new shape on apply.
// ---------------------------------------------------------------
function TourCanvas({ tour, pending, lastMove, phase, morph }: {
  tour: number[]
  pending: MoveEvent | null
  lastMove: MoveEvent | null
  phase: Phase
  morph: boolean
}) {
  const showCandidate = phase === 'candidate' && pending
  const showApplied = phase === 'swap_applied' && lastMove

  const activeMove = useMemo<MoveEvent | null>(() => {
    if (showCandidate) return pending
    if (showApplied) return lastMove
    return null
  }, [pending, lastMove, showCandidate, showApplied])

  const removedSet = useMemo(() => {
    if (!activeMove) return new Set<string>()
    return new Set(activeMove.removedEdges.map(([a, b]) => edgeKey(a, b)))
  }, [activeMove])
  const addedSet = useMemo(() => {
    if (!activeMove) return new Set<string>()
    return new Set(activeMove.addedEdges.map(([a, b]) => edgeKey(a, b)))
  }, [activeMove])

  const edges: Array<{ from: number; to: number; key: string }> = []
  for (let k = 0; k < tour.length; k++) {
    const a = tour[k]
    const b = tour[(k + 1) % tour.length]
    edges.push({ from: a, to: b, key: edgeKey(a, b) })
  }

  return (
    <svg viewBox="0 0 300 300"
      className={`t3-canvas ${morph && showApplied ? 't3-morph' : ''}`}
      role="img" aria-label="3-opt tour">
      <rect x={0} y={0} width={300} height={300} className="t3-bg" />
      <g className="t3-tour">
        {edges.map(({ from, to, key }) => {
          let cls = 't3-edge'
          if (addedSet.has(key)) cls += showCandidate ? ' t3-cand-new' : ' t3-new'
          else if (removedSet.has(key)) cls += showCandidate ? ' t3-cand-removed' : ' t3-removed'
          return <line key={key} className={cls}
            x1={CITIES[from][0]} y1={CITIES[from][1]}
            x2={CITIES[to][0]} y2={CITIES[to][1]} />
        })}
        {tour.map((id) => (
          <g key={id}>
            <circle className="t3-city"
              cx={CITIES[id][0]} cy={CITIES[id][1]} r={6} />
            <text className="t3-label"
              x={CITIES[id][0]} y={CITIES[id][1] + 16}>{id}</text>
          </g>
        ))}
      </g>
    </svg>
  )
}

// ---------------------------------------------------------------
// Cost sparkline (same pattern as 2-opt / or-opt)
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
    <svg viewBox={`0 0 ${W} ${H}`} className="t3-spark" aria-label="cost over passes">
      <rect x={0} y={0} width={W} height={H} className="t3-bg" rx={4} />
      <polyline points={pts.join(" ")} fill="none" stroke="#0d9488"
        strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function ThreeOptExplainer() {
  const [tour, setTour] = useState<number[]>(() => [...DEFAULT_SCENARIO.tour])
  const [phase, setPhase] = useState<Phase>('idle')
  const [pass, setPass] = useState(0)
  const [swaps, setSwaps] = useState(0)
  const [bestCost, setBestCost] = useState(() => tourLength(DEFAULT_SCENARIO.tour))
  const [pending, setPending] = useState<MoveEvent | null>(null)
  const [lastMove, setLastMove] = useState<MoveEvent | null>(null)
  const [costHistory, setCostHistory] = useState<number[]>(() => [tourLength(DEFAULT_SCENARIO.tour)])
  const [step, setStep] = useState(0)
  const [running, setRunning] = useState(false)
  const [speedIdx, setSpeedIdx] = useState(2) // 3x default
  const [morphOn, setMorphOn] = useState(false) // animate on apply, not on Back

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
    setSwaps(0)
    setBestCost(simRef.current.bestCost)
    setPending(null)
    setLastMove(null)
    setCostHistory([simRef.current.bestCost])
    setStep(0)
    setRunning(false)
    setMorphOn(false)
  }, [])

  const stepForward = useCallback(() => {
    // Guard against an in-flight interval tick after the run already finished.
    if (simRef.current.phase === 'local_optimum') return
    historyRef.current.push(structuredClone(simRef.current))
    const next = stepOnce(simRef.current)
    simRef.current = next
    setTour([...next.tour])
    setPhase(next.phase)
    setPass(next.pass)
    setSwaps(next.swaps)
    setBestCost(next.bestCost)
    setPending(next.pending)
    setLastMove(next.lastMove)
    setCostHistory([...next.costHistory])
    setStep(next.step)
    setMorphOn(next.phase === 'swap_applied')
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
    setSwaps(prev.swaps)
    setBestCost(prev.bestCost)
    setPending(prev.pending)
    setLastMove(prev.lastMove)
    setCostHistory([...prev.costHistory])
    setStep(prev.step)
    setMorphOn(false)
  }, [running])

  useEffect(() => {
    if (!running) return
    const ms = SPEEDS[speedIdx]
    const id = setInterval(stepForward, ms)
    return () => clearInterval(id)
  }, [running, speedIdx, stepForward])

  const distance = tourLength(tour)
  let activeCase: CaseNo | null = null
  if (phase === 'candidate' && pending) {
    activeCase = pending.caseNo
  } else if (phase === 'swap_applied' && lastMove) {
    activeCase = lastMove.caseNo
  }

  // Status chip
  let chipText = "Click Step to scan triples for the best 3-opt reconnection"
  let chipClass = "t3-chip t3-chip-idle"
  if (phase === 'candidate' && pending) {
    const [[a, b], [c, d], [e, f]] = pending.removedEdges
    chipText = `Candidate — remove ${a}→${b} · ${c}→${d} · ${e}→${f}, case ${pending.caseNo} (${CASE_LABELS[pending.caseNo]})  (Δ=${pending.delta.toFixed(0)})`
    chipClass = "t3-chip t3-chip-candidate"
  } else if (phase === 'swap_applied' && lastMove) {
    chipText = `Applied — case ${lastMove.caseNo} (${CASE_LABELS[lastMove.caseNo]})  (Δ=${lastMove.delta.toFixed(0)})`
    chipClass = "t3-chip t3-chip-applied"
  } else if (phase === 'local_optimum') {
    chipText = `Local optimum — no improving 3-opt move (${swaps} swaps, ${pass} passes)`
    chipClass = "t3-chip t3-chip-done"
  }

  return (
    <div className="t3-root">
      <style>{CSS}</style>

      <header className="t3-header">
        <div className="t3-eyebrow">teeline · algorithms/3opt</div>
        <h2 className="t3-title">3-opt Local Search</h2>
        <p className="t3-sub">
          3-opt removes <strong>three edges</strong> and reconnects the three resulting segments in
          one of <strong>seven ways</strong> (2-opt only has one). Cases 1–3 reverse segments; cases
          4–7 <em>swap</em> segments — the moves 2-opt cannot express. Each pass scans all triples
          and applies the single <strong>best-improving</strong> reconnection, repeating until no
          triple yields an improvement.
        </p>
      </header>

      <div className="t3-viz-row">
        <div className="t3-canvas-wrap">
          <TourCanvas tour={tour} pending={pending} lastMove={lastMove} phase={phase} morph={morphOn} />
        </div>
        <div className="t3-pattern-wrap">
          <div className="t3-section-label">Reconnection patterns</div>
          <div className="t3-pattern-grid">
            {CASES.map((c) => (
              <PatternCell key={c} caseNo={c} active={activeCase === c} />
            ))}
          </div>
        </div>
      </div>

      <div className="t3-legend">
        <span><span className="t3-swatch t3-swatch-normal" /> tour edge</span>
        <span><span className="t3-swatch t3-swatch-removed" /> removed</span>
        <span><span className="t3-swatch t3-swatch-new" /> new edge</span>
        <span><span className="t3-swatch t3-swatch-pattern" /> chosen pattern</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="t3-section-label">Cost over passes</div>
      <Sparkline values={costHistory} />

      <div className="t3-statgrid">
        <div>
          <div className="t3-statlabel">pass</div>
          <div className="t3-mono">{pass}</div>
        </div>
        <div>
          <div className="t3-statlabel">swaps</div>
          <div className="t3-mono">{swaps}</div>
        </div>
        <div>
          <div className="t3-statlabel">best cost</div>
          <div className="t3-mono">{bestCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="t3-statlabel">distance</div>
          <div className="t3-mono">{distance.toFixed(0)}</div>
        </div>
        <div>
          <div className="t3-statlabel">last Δ</div>
          <div className="t3-mono" style={{ color: lastMove && lastMove.delta < 0 ? '#16a34a' : 'inherit' }}>
            {lastMove ? lastMove.delta.toFixed(0) : '—'}
          </div>
        </div>
        <div>
          <div className="t3-statlabel">step</div>
          <div className="t3-mono">{step}</div>
        </div>
      </div>

      <div className="t3-config">
        <div className="t3-config-row">
          <label className="t3-label">Speed</label>
          <div className="t3-speed-btns">
            {SPEED_LABELS.map((l, i) => (
              <button key={l}
                className={`t3-speed-btn ${i === speedIdx ? 't3-speed-btn-sel' : ''}`}
                onClick={() => setSpeedIdx(i)} disabled={running}>
                {l}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="t3-controls">
        <button className="t3-btn" onClick={stepBack} disabled={running || historyRef.current.length === 0}>
          ⏴ Back
        </button>
        <button className="t3-btn" onClick={stepForward} disabled={running || phase === 'local_optimum'}>
          ⏵ Step
        </button>
        <button className="t3-btn" onClick={() => setRunning(!running)} disabled={phase === 'local_optimum'}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="t3-btn" onClick={() => reinit(scenarioRef.current)} disabled={running}>↺ Reset</button>
      </div>

      <div className="t3-scenarios">
        <div className="t3-section-label">Scenarios</div>
        <div className="t3-scenario-row">
          {Object.entries(SCENARIOS).map(([key, s]) => (
            <button key={key} className="t3-scenario-btn" title={s.desc}
              onClick={() => reinit(s)} disabled={running}>
              {s.label}
            </button>
          ))}
        </div>
      </div>

      <footer className="t3-footer">
        <span className="t3-mono">cities: {N_CITIES}</span>
        <span className="t3-mono">O(n³) triple scan · 7 reconnections · best-improvement</span>
      </footer>
    </div>
  )
}

const CSS = `
.t3-root {
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
  max-width: 820px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.t3-root * { box-sizing: border-box; }
.t3-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

.t3-header { margin-bottom: 2px; }
.t3-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.t3-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.t3-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }

.t3-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.t3-bg { fill: var(--panel); }
.t3-edge { stroke: #94a3b8; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.t3-cand-removed { stroke: #ea580c; stroke-width: 3.5; stroke-dasharray: 6 4; }
.t3-cand-new { stroke: #16a34a; stroke-width: 3.5; stroke-dasharray: 4 6; }
.t3-removed { stroke: #ef4444; stroke-width: 3.5; stroke-dasharray: 6 3; }
.t3-new { stroke: #16a34a; stroke-width: 3.5; }
.t3-city { fill: #1f2937; stroke: #fff; stroke-width: 1.5; transition: cx 0.35s ease, cy 0.35s ease; }
.t3-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: #374151; text-anchor: middle;
  paint-order: stroke fill; stroke: white; stroke-width: 3;
  pointer-events: none; user-select: none;
}

/* Morph: when a swap is applied, the whole tour flows to its new shape. */
.t3-morph .t3-edge,
.t3-morph .t3-removed,
.t3-morph .t3-new,
.t3-morph .t3-cand-removed,
.t3-morph .t3-cand-new {
  transition: x1 0.35s ease, y1 0.35s ease, x2 0.35s ease, y2 0.35s ease;
}

.t3-viz-row { display: flex; gap: 10px; align-items: stretch; }
.t3-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

.t3-pattern-wrap { width: 168px; flex-shrink: 0; display: flex; flex-direction: column; gap: 6px; }
.t3-pattern-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 4px; }
.t3-pattern-cell {
  border: 1px solid var(--line); border-radius: 6px; padding: 2px;
  background: var(--bg); display: flex; flex-direction: column; align-items: center;
}
.t3-pattern-active {
  border-color: #ea580c; background: #fff7ed;
  box-shadow: 0 0 0 1px #ea580c;
}
.t3-pattern-svg { width: 100%; display: block; }
.t3-pattern-removed { stroke: #94a3b8; stroke-width: 1; stroke-dasharray: 2 2; opacity: 0.6; }
.t3-pattern-new { stroke: #ea580c; stroke-width: 1.6; }
.t3-pattern-node {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 6px; fill: #374151; text-anchor: middle; dominant-baseline: central;
}
.t3-pattern-case {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.7rem; color: var(--muted);
}

.t3-legend {
  display: flex; flex-wrap: wrap; gap: 6px 14px; font-size: 0.78rem; color: var(--muted);
}
.t3-swatch {
  display: inline-block; width: 14px; height: 3px; border-radius: 2px;
  margin-right: 4px; vertical-align: middle;
}
.t3-swatch-normal { background: #94a3b8; }
.t3-swatch-removed { background: #ef4444; }
.t3-swatch-new { background: #16a34a; }
.t3-swatch-pattern { background: #ea580c; }

.t3-chip {
  font-size: 0.82rem; padding: 6px 12px; border-radius: 8px;
  font-weight: 500; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.t3-chip-idle { background: #f1f5f9; color: var(--muted); }
.t3-chip-candidate { background: #ffedd5; color: #9a3412; }
.t3-chip-applied { background: #dcfce7; color: #166534; }
.t3-chip-done { background: #f1f5f9; color: var(--muted); }

.t3-section-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }
.t3-spark { width: 100%; display: block; border-radius: 6px; }

.t3-statgrid {
  display: flex; flex-wrap: wrap; gap: 14px 22px;
  font-size: 0.82rem;
}
.t3-statlabel { color: var(--muted); font-size: 0.72em; text-transform: uppercase; letter-spacing: 0.06em; }

.t3-config {
  display: flex; flex-direction: column; gap: 8px;
  padding: 12px; background: var(--panel); border-radius: 8px;
}
.t3-config-row {
  display: flex; align-items: center; gap: 10px; flex-wrap: wrap;
}
.t3-label {
  font-size: 0.82rem; font-weight: 500; color: var(--text);
  min-width: 50px;
}
.t3-speed-btns { display: flex; gap: 4px; }
.t3-speed-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.t3-speed-btn:hover:not(:disabled) { background: #f0fdf4; }
.t3-speed-btn:disabled { opacity: 0.4; cursor: default; }
.t3-speed-btn-sel { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.t3-controls { display: flex; gap: 8px; }
.t3-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem; padding: 6px 14px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.t3-btn:hover:not(:disabled) { background: #f0fdf4; }
.t3-btn:disabled { opacity: 0.4; cursor: default; }

.t3-scenarios { display: flex; flex-direction: column; gap: 6px; }
.t3-scenario-row { display: flex; flex-wrap: wrap; gap: 6px; }
.t3-scenario-btn {
  font-size: 0.8rem; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.t3-scenario-btn:hover:not(:disabled) { background: #f0fdf4; border-color: var(--accent); }
.t3-scenario-btn:disabled { opacity: 0.4; cursor: default; }

.t3-footer {
  display: flex; gap: 12px; justify-content: center;
  padding-top: 8px; border-top: 1px solid var(--line);
  color: var(--muted); font-size: 0.78rem;
}
`

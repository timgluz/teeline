import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  CITIES, N_CITIES, SCENARIOS,
  tourLength, makeInitState, stepOnce,
} from "./stochastic-hill-algo"
import type { Phase, CandidateEvent, Scenario } from "./stochastic-hill-algo"

const DEFAULT_SCENARIO = SCENARIOS.quick_convergence
const SPEEDS = [600, 420, 280, 180, 100, 50, 25]
const SPEED_LABELS = ["1x", "2x", "3x", "4x", "5x", "6x", "7x"]

function sameTour(a: number[], b: number[]): boolean {
  if (a.length !== b.length) return false
  return a.every((v, i) => v === b[i])
}

function edgeKey(a: number, b: number): string {
  return `${Math.min(a, b)}-${Math.max(a, b)}`
}

// ---------------------------------------------------------------
// TourCanvas — ghost best tour behind the current tour, with
// candidate/verdict edge highlighting and a restart fade.
// ---------------------------------------------------------------
function TourCanvas({ tour, bestTour, pending, phase }: {
  tour: number[]
  bestTour: number[]
  pending: CandidateEvent | null
  phase: Phase
}) {
  const showGhost = !sameTour(bestTour, tour)

  const removedSet = useMemo(() => {
    if (!pending) return new Set<string>()
    return new Set(pending.removed.map(([a, b]) => edgeKey(a, b)))
  }, [pending])
  const addedSet = useMemo(() => {
    if (!pending) return new Set<string>()
    return new Set(pending.added.map(([a, b]) => edgeKey(a, b)))
  }, [pending])

  const edges: Array<{ from: number; to: number; key: string }> = []
  for (let k = 0; k < tour.length; k++) {
    const a = tour[k]
    const b = tour[(k + 1) % tour.length]
    edges.push({ from: a, to: b, key: edgeKey(a, b) })
  }

  const isPropose = phase === 'propose' && pending
  const isAccepted = phase === 'accepted' && pending
  const isRejected = phase === 'rejected' && pending
  const isRestarting = phase === 'restart'

  return (
    <svg viewBox="0 0 300 300" className={`shc-canvas ${isAccepted ? 'shc-glow' : ''}`}
      role="img" aria-label="Stochastic hill climbing tour">
      <rect x={0} y={0} width={300} height={300} className="shc-bg" />

      {/* Ghost of the best tour found so far (visible when it differs — i.e. after a restart) */}
      {showGhost && (
        <g className="shc-ghost">
          {bestTour.map((_id, k) => {
            const a = bestTour[k]
            const b = bestTour[(k + 1) % bestTour.length]
            return <line key={`g${edgeKey(a, b)}`} className="shc-edge-ghost"
              x1={CITIES[a][0]} y1={CITIES[a][1]}
              x2={CITIES[b][0]} y2={CITIES[b][1]} />
          })}
        </g>
      )}

      <g className={isRestarting ? 'shc-restarting' : ''}>
        {edges.map(({ from, to, key }) => {
          let cls = 'shc-edge'
          if (isPropose) {
            if (removedSet.has(key)) cls += ' shc-cand-removed'
            else if (addedSet.has(key)) cls += ' shc-cand-added'
          } else if (isAccepted) {
            if (removedSet.has(key)) cls += ' shc-removed'
            else if (addedSet.has(key)) cls += ' shc-added'
          } else if (isRejected) {
            if (removedSet.has(key)) cls += ' shc-rejected'
          }
          return <line key={key} className={cls}
            x1={CITIES[from][0]} y1={CITIES[from][1]}
            x2={CITIES[to][0]} y2={CITIES[to][1]} />
        })}
        {tour.map((id) => (
          <g key={id}>
            <circle className="shc-city"
              cx={CITIES[id][0]} cy={CITIES[id][1]} r={6} />
            <text className="shc-label"
              x={CITIES[id][0]} y={CITIES[id][1] + 16}>{id}</text>
          </g>
        ))}
      </g>
    </svg>
  )
}

// ---------------------------------------------------------------
// Best-cost sparkline (same pattern as 2-opt / ACO)
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
    <svg viewBox={`0 0 ${W} ${H}`} className="shc-spark" aria-label="best cost over epochs">
      <rect x={0} y={0} width={W} height={H} className="shc-bg" rx={4} />
      <polyline points={pts.join(" ")} fill="none" stroke="#0d9488"
        strokeWidth={1.5} strokeLinejoin="round" />
    </svg>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function StochasticHillExplainer() {
  const [tour, setTour] = useState<number[]>(() => [...DEFAULT_SCENARIO.initTour!])
  const [bestTour, setBestTour] = useState<number[]>(() => [...DEFAULT_SCENARIO.initTour!])
  const [phase, setPhase] = useState<Phase>('idle')
  const [epoch, setEpoch] = useState(0)
  const [nStale, setNStale] = useState(0)
  const [restarts, setRestarts] = useState(0)
  const [bestCost, setBestCost] = useState(() => tourLength(DEFAULT_SCENARIO.initTour!))
  const [currentCost, setCurrentCost] = useState(() => tourLength(DEFAULT_SCENARIO.initTour!))
  const [acceptedCount, setAcceptedCount] = useState(0)
  const [rejectedCount, setRejectedCount] = useState(0)
  const [pending, setPending] = useState<CandidateEvent | null>(null)
  const [costHistory, setCostHistory] = useState<number[]>(() => [tourLength(DEFAULT_SCENARIO.initTour!)])
  const [step, setStep] = useState(0)
  const [running, setRunning] = useState(false)
  const [speedIdx, setSpeedIdx] = useState(3) // 4x default — 8-city runs are short; keep them snappy
  const [epochsInput, setEpochsInput] = useState(DEFAULT_SCENARIO.epochs)
  const [patienceInput, setPatienceInput] = useState(DEFAULT_SCENARIO.patience)

  const simRef = useRef(makeInitState(DEFAULT_SCENARIO))
  const historyRef = useRef<Array<typeof simRef.current>>([])
  const scenarioRef = useRef<Scenario>(DEFAULT_SCENARIO)

  const reinit = useCallback((scenario: Scenario) => {
    scenarioRef.current = scenario
    simRef.current = makeInitState(scenario)
    historyRef.current = []
    setTour([...simRef.current.tour])
    setBestTour([...simRef.current.bestTour])
    setPhase('idle')
    setEpoch(0)
    setNStale(0)
    setRestarts(0)
    setBestCost(simRef.current.bestCost)
    setCurrentCost(simRef.current.currentCost)
    setAcceptedCount(0)
    setRejectedCount(0)
    setPending(null)
    setCostHistory([simRef.current.bestCost])
    setStep(0)
    setRunning(false)
    setEpochsInput(scenario.epochs)
    setPatienceInput(scenario.patience)
  }, [])

  const stepForward = useCallback(() => {
    // Guard against an in-flight interval tick after the run already finished:
    // stepping a 'done' state would push a phantom undo entry.
    if (simRef.current.phase === 'done') return
    historyRef.current.push(structuredClone(simRef.current))
    const next = stepOnce(simRef.current)
    simRef.current = next
    setTour([...next.tour])
    setBestTour([...next.bestTour])
    setPhase(next.phase)
    setEpoch(next.epoch)
    setNStale(next.nStale)
    setRestarts(next.restarts)
    setBestCost(next.bestCost)
    setCurrentCost(next.currentCost)
    setAcceptedCount(next.acceptedCount)
    setRejectedCount(next.rejectedCount)
    setPending(next.pending)
    setCostHistory([...next.costHistory])
    setStep(next.step)
    if (next.phase === 'done') setRunning(false)
  }, [])

  const stepBack = useCallback(() => {
    const h = historyRef.current
    if (h.length === 0 || running) return
    const prev = h.pop()!
    simRef.current = prev
    setTour([...prev.tour])
    setBestTour([...prev.bestTour])
    setPhase(prev.phase)
    setEpoch(prev.epoch)
    setNStale(prev.nStale)
    setRestarts(prev.restarts)
    setBestCost(prev.bestCost)
    setCurrentCost(prev.currentCost)
    setAcceptedCount(prev.acceptedCount)
    setRejectedCount(prev.rejectedCount)
    setPending(prev.pending)
    setCostHistory([...prev.costHistory])
    setStep(prev.step)
  }, [running])

  useEffect(() => {
    if (!running) return
    const ms = SPEEDS[speedIdx]
    const id = setInterval(stepForward, ms)
    return () => clearInterval(id)
  }, [running, speedIdx, stepForward])

  // Restart-pace params are per-scenario defaults, user-adjustable; changing
  // them restarts the run with the new values (keeps the current scenario).
  const applyParams = useCallback((epochs: number, patience: number) => {
    const sc = scenarioRef.current
    // Math.floor(x) || 1 guards against NaN (a partially-typed number input
    // like "-" or ".") and 0 — either would otherwise break the epoch cap /
    // restart threshold (e.g. maxEpochs: NaN never triggers 'done').
    const clampedEpochs = Math.max(1, Math.min(500, Math.floor(epochs) || 1))
    const clampedPatience = Math.max(1, Math.min(100, Math.floor(patience) || 1))
    setEpochsInput(clampedEpochs)
    setPatienceInput(clampedPatience)
    reinit({ ...sc, epochs: clampedEpochs, patience: clampedPatience })
  }, [reinit])

  const acceptRate = acceptedCount + rejectedCount > 0
    ? Math.round((acceptedCount / (acceptedCount + rejectedCount)) * 100)
    : null

  // Status chip
  let chipText = "Click Step to draw a random 2-opt candidate"
  let chipClass = "shc-chip shc-chip-idle"
  if (phase === 'propose' && pending) {
    chipText = `Candidate — reverse ${pending.i}…${pending.j}  (Δ=${pending.delta.toFixed(0)}, ${pending.accepted ? 'beats best' : 'won’t beat best'})`
    chipClass = "shc-chip shc-chip-candidate"
  } else if (phase === 'accepted' && pending) {
    chipText = `Accepted — new best ${bestCost.toFixed(0)}  (Δ=${pending.delta.toFixed(0)})`
    chipClass = "shc-chip shc-chip-accepted"
  } else if (phase === 'rejected' && pending) {
    chipText = `Rejected — Δ=${pending.delta.toFixed(0)}, candidate ${pending.candidateCost.toFixed(0)} ≥ best ${bestCost.toFixed(0)}  (${nStale} stale)`
    chipClass = "shc-chip shc-chip-rejected"
  } else if (phase === 'restart') {
    chipText = "Restarting… search went stale — fresh random tour appears (best survives)"
    chipClass = "shc-chip shc-chip-restart"
  } else if (phase === 'done') {
    chipText = `Done — ${epoch} epochs · ${restarts} restarts · best ${bestCost.toFixed(0)}`
    chipClass = "shc-chip shc-chip-done"
  }

  return (
    <div className="shc-root">
      <style>{CSS}</style>

      <header className="shc-header">
        <div className="shc-eyebrow">teeline · algorithms/stochastic_hill</div>
        <h2 className="shc-title">Stochastic Hill Climbing</h2>
        <p className="shc-sub">
          Each step draws a <strong>random 2-opt candidate</strong> — a random segment reversal.
          The candidate is accepted only if it <strong>beats the best tour found so far</strong>;
          otherwise it is rejected and the tour stays. When the search goes stale (too many
          rejections in a row), it <strong>restarts from a fresh random tour</strong> — the best
          tour survives. Random restarts are what rescue the search from mediocre local optima.
        </p>
      </header>

      <div className="shc-viz-row">
        <div className="shc-canvas-wrap">
          <TourCanvas tour={tour} bestTour={bestTour} pending={pending} phase={phase} />
        </div>
      </div>

      <div className="shc-legend">
        <span><span className="shc-swatch shc-swatch-normal" /> tour edge</span>
        <span><span className="shc-swatch shc-swatch-ghost" /> best tour (ghost)</span>
        <span><span className="shc-swatch shc-swatch-cand-rm" /> candidate (remove)</span>
        <span><span className="shc-swatch shc-swatch-cand-ad" /> candidate (add)</span>
        <span><span className="shc-swatch shc-swatch-removed" /> removed</span>
        <span><span className="shc-swatch shc-swatch-added" /> new edge</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="shc-section-label">Best cost over epochs</div>
      <Sparkline values={costHistory} />

      <div className="shc-statgrid">
        <div>
          <div className="shc-statlabel">epoch</div>
          <div className="shc-mono">{epoch}</div>
        </div>
        <div>
          <div className="shc-statlabel">restarts</div>
          <div className="shc-mono">{restarts}</div>
        </div>
        <div>
          <div className="shc-statlabel">best cost</div>
          <div className="shc-mono">{bestCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="shc-statlabel">current cost</div>
          <div className="shc-mono">{currentCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="shc-statlabel">accept rate</div>
          <div className="shc-mono">{acceptRate === null ? '—' : `${acceptRate}%`}</div>
        </div>
        <div>
          <div className="shc-statlabel">step</div>
          <div className="shc-mono">{step}</div>
        </div>
      </div>

      <div className="shc-config">
        <div className="shc-config-row">
          <label className="shc-label" htmlFor="shc-epochs">Epochs</label>
          <input id="shc-epochs" className="shc-input" type="number" min={1} max={500}
            value={epochsInput}
            onInput={(e) => setEpochsInput(Number((e.target as HTMLInputElement).value))}
            onBlur={() => applyParams(epochsInput, patienceInput)} />
          <label className="shc-label" htmlFor="shc-patience">Restart patience</label>
          <input id="shc-patience" className="shc-input" type="number" min={1} max={100}
            value={patienceInput}
            onInput={(e) => setPatienceInput(Number((e.target as HTMLInputElement).value))}
            onBlur={() => applyParams(epochsInput, patienceInput)} />
        </div>
        <div className="shc-config-row">
          <label className="shc-label">Speed</label>
          <div className="shc-speed-btns">
            {SPEED_LABELS.map((l, i) => (
              <button key={l}
                className={`shc-speed-btn ${i === speedIdx ? 'shc-speed-btn-sel' : ''}`}
                onClick={() => setSpeedIdx(i)} disabled={running}>
                {l}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="shc-controls">
        <button className="shc-btn" onClick={stepBack} disabled={running || historyRef.current.length === 0}>
          ⏴ Back
        </button>
        <button className="shc-btn" onClick={stepForward} disabled={running || phase === 'done'}>
          ⏵ Step
        </button>
        <button className="shc-btn" onClick={() => setRunning(!running)} disabled={phase === 'done'}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="shc-btn" onClick={() => reinit(scenarioRef.current)}>↺ Reset</button>
      </div>

      <div className="shc-scenarios">
        <div className="shc-section-label">Scenarios</div>
        <div className="shc-scenario-row">
          {Object.entries(SCENARIOS).map(([key, s]) => (
            <button key={key} className="shc-scenario-btn" title={s.desc}
              onClick={() => reinit(s)} disabled={running}>
              {s.label}
            </button>
          ))}
        </div>
      </div>

      <footer className="shc-footer">
        <span className="shc-mono">cities: {N_CITIES}</span>
        <span className="shc-mono">random 2-opt · accept only if &lt; best · random restart</span>
      </footer>
    </div>
  )
}

const CSS = `
.shc-root {
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
.shc-root * { box-sizing: border-box; }
.shc-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

.shc-header { margin-bottom: 2px; }
.shc-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.shc-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.shc-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }

.shc-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
  transition: box-shadow 0.3s ease;
}
.shc-glow { box-shadow: 0 0 0 3px rgba(22, 163, 74, 0.35); }
.shc-bg { fill: var(--panel); }
.shc-edge { stroke: #94a3b8; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.shc-edge-ghost { stroke: #64748b; stroke-width: 1.5; stroke-dasharray: 2 4; opacity: 0.35; }
.shc-cand-removed { stroke: #ea580c; stroke-width: 3.5; stroke-dasharray: 6 4; }
.shc-cand-added { stroke: #16a34a; stroke-width: 3.5; stroke-dasharray: 4 6; }
.shc-removed { stroke: #ef4444; stroke-width: 3.5; stroke-dasharray: 6 3; }
.shc-added { stroke: #16a34a; stroke-width: 3.5; }
.shc-rejected { stroke: #ef4444; stroke-width: 3.5; stroke-dasharray: 3 3; animation: shc-flash-red 0.6s ease; }
.shc-city { fill: #1f2937; stroke: #fff; stroke-width: 1.5; }
.shc-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: #374151; text-anchor: middle;
  paint-order: stroke fill; stroke: white; stroke-width: 3;
  pointer-events: none; user-select: none;
}

@keyframes shc-flash-red {
  0% { opacity: 0.3; }
  50% { opacity: 1; }
  100% { opacity: 1; }
}
@keyframes shc-fade {
  0% { opacity: 1; }
  45% { opacity: 0.12; }
  55% { opacity: 0.12; }
  100% { opacity: 1; }
}
.shc-restarting { animation: shc-fade 1.1s ease-in-out; }

.shc-viz-row { display: flex; gap: 10px; align-items: stretch; }
.shc-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

.shc-legend {
  display: flex; flex-wrap: wrap; gap: 6px 14px; font-size: 0.78rem; color: var(--muted);
}
.shc-swatch {
  display: inline-block; width: 14px; height: 3px; border-radius: 2px;
  margin-right: 4px; vertical-align: middle;
}
.shc-swatch-normal { background: #94a3b8; }
.shc-swatch-ghost { background: #64748b; }
.shc-swatch-cand-rm { background: #ea580c; }
.shc-swatch-cand-ad { background: #16a34a; }
.shc-swatch-removed { background: #ef4444; }
.shc-swatch-added { background: #16a34a; }

.shc-chip {
  font-size: 0.82rem; padding: 6px 12px; border-radius: 8px;
  font-weight: 500; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.shc-chip-idle { background: #f1f5f9; color: var(--muted); }
.shc-chip-candidate { background: #ffedd5; color: #9a3412; }
.shc-chip-accepted { background: #dcfce7; color: #166534; }
.shc-chip-rejected { background: #fee2e2; color: #991b1b; }
.shc-chip-restart { background: #ede9fe; color: #5b21b6; }
.shc-chip-done { background: #f1f5f9; color: var(--muted); }

.shc-section-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }
.shc-spark { width: 100%; display: block; border-radius: 6px; }

.shc-statgrid {
  display: flex; flex-wrap: wrap; gap: 14px 22px;
  font-size: 0.82rem;
}
.shc-statlabel { color: var(--muted); font-size: 0.72em; text-transform: uppercase; letter-spacing: 0.06em; }

.shc-config {
  display: flex; flex-direction: column; gap: 8px;
  padding: 12px; background: var(--panel); border-radius: 8px;
}
.shc-config-row {
  display: flex; align-items: center; gap: 10px; flex-wrap: wrap;
}
.shc-label {
  font-size: 0.82rem; font-weight: 500; color: var(--text);
}
.shc-input {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.82rem; width: 72px; padding: 4px 6px;
  border: 1px solid var(--line); border-radius: 5px; background: var(--bg); color: var(--text);
}
.shc-speed-btns { display: flex; gap: 4px; }
.shc-speed-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.shc-speed-btn:hover:not(:disabled) { background: #f0fdf4; }
.shc-speed-btn:disabled { opacity: 0.4; cursor: default; }
.shc-speed-btn-sel { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.shc-controls { display: flex; gap: 8px; }
.shc-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem; padding: 6px 14px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.shc-btn:hover:not(:disabled) { background: #f0fdf4; }
.shc-btn:disabled { opacity: 0.4; cursor: default; }

.shc-scenarios { display: flex; flex-direction: column; gap: 6px; }
.shc-scenario-row { display: flex; flex-wrap: wrap; gap: 6px; }
.shc-scenario-btn {
  font-size: 0.8rem; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer; transition: background 0.15s;
}
.shc-scenario-btn:hover:not(:disabled) { background: #f0fdf4; border-color: var(--accent); }
.shc-scenario-btn:disabled { opacity: 0.4; cursor: default; }

.shc-footer {
  display: flex; gap: 12px; justify-content: center;
  padding-top: 8px; border-top: 1px solid var(--line);
  color: var(--muted); font-size: 0.78rem;
}
`

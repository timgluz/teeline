import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  PHASES, SCENARIOS,
  makeInitState, stepOnce,
} from "./christofides-algo"
import type { Phase, Instance, SimState } from "./christofides-algo"

const DEFAULT_SCENARIO = SCENARIOS.balanced
const SPEEDS = [600, 420, 280, 180, 100, 50, 25]
const SPEED_LABELS = ["1x", "2x", "3x", "4x", "5x", "6x", "7x"]

const PHASE_LABELS: Record<Phase, string> = {
  mst: 'MST',
  odd: 'Odd vertices',
  matching: 'Matching',
  euler: 'Euler walk',
  shortcut: 'Shortcut',
  done: 'Done',
}

function edgeKey(a: number, b: number): string {
  return `${Math.min(a, b)}-${Math.max(a, b)}`
}

// ---------------------------------------------------------------
// TourCanvas — the city map with progressive layers:
// green MST edges, purple dashed matching, the multigraph with a
// walking dot (euler), and the final tour being carved out (shortcut).
// ---------------------------------------------------------------
function TourCanvas({ sim, showMst, showMatch, showTour }: {
  sim: SimState
  showMst: boolean
  showMatch: boolean
  showTour: boolean
}) {
  const { cities, phase } = sim
  const pos = (i: number) => cities[i]

  const multigraphEdges = useMemo(() => {
    const list: Array<{ id: number; a: number; b: number; kind: 'mst' | 'match' }> = []
    for (const [a, b] of sim.mstEdges) list.push({ id: list.length, a, b, kind: 'mst' })
    for (const [a, b] of sim.matchingEdges) list.push({ id: list.length, a, b, kind: 'match' })
    return list
  }, [sim.mstEdges, sim.matchingEdges])

  // multigraph edges the walker has already used, per unordered pair
  const usedPairs = useMemo(() => {
    const map = new Map<string, number>()
    let upto: number
    if (phase === 'euler') {
      upto = sim.walkerPos
    } else if (phase === 'shortcut' || phase === 'done') {
      upto = sim.eulerCircuit.length - 1
    } else {
      upto = 0
    }
    for (let t = 0; t < upto; t++) {
      const k = edgeKey(sim.eulerCircuit[t], sim.eulerCircuit[t + 1])
      map.set(k, (map.get(k) ?? 0) + 1)
    }
    return map
  }, [phase, sim.walkerPos, sim.eulerCircuit])

  const pairCount = useMemo(() => {
    const map = new Map<string, number>()
    for (const e of multigraphEdges) {
      const k = edgeKey(e.a, e.b)
      map.set(k, (map.get(k) ?? 0) + 1)
    }
    return map
  }, [multigraphEdges])

  const isEdgeUsed = (a: number, b: number) => {
    const k = edgeKey(a, b)
    const used = usedPairs.get(k) ?? 0
    const total = pairCount.get(k) ?? 1
    return used >= total
  }

  // final tour edges (built during shortcut; complete at done)
  const tourEdges: Array<[number, number]> = []
  const kept = phase === 'done' ? sim.shortcutTour : sim.kept
  for (let t = 0; t < kept.length - 1; t++) tourEdges.push([kept[t], kept[t + 1]])
  if (phase === 'done' && kept.length > 1) tourEdges.push([kept[kept.length - 1], kept[0]])

  const skippedSet = new Set(sim.skipped)

  // walker position: euler phase follows walkerPos; shortcut follows shortcutStep
  let walkerCity: number | null = null
  if (phase === 'euler') {
    walkerCity = sim.eulerCircuit[Math.min(sim.walkerPos, sim.eulerCircuit.length - 1)]
  } else if (phase === 'shortcut') {
    walkerCity = sim.eulerCircuit[Math.min(sim.shortcutStep, sim.eulerCircuit.length - 1)]
  }

  const constructing = phase === 'mst' || phase === 'odd' || phase === 'matching'
  let mstShown: [number, number][] = []
  if (showMst && constructing) {
    const limit = phase === 'mst' ? sim.mstRevealed : sim.mstEdges.length
    mstShown = sim.mstEdges.slice(0, limit)
  }
  let matchShown: [number, number][] = []
  if (showMatch && phase === 'matching') {
    matchShown = sim.matchingEdges.slice(0, sim.matchingRevealed)
  } else if (showMatch && (phase === 'euler' || phase === 'shortcut' || phase === 'done')) {
    matchShown = sim.matchingEdges
  }
  const showMultigraph = phase === 'euler' || phase === 'shortcut'
  const showTourLayer = showTour && (phase === 'shortcut' || phase === 'done')

  return (
    <svg viewBox="0 0 300 300" className="chr-canvas" role="img" aria-label="Christofides construction">
      <rect x={0} y={0} width={300} height={300} className="chr-bg" />

      {/* Plain construction layers */}
      {mstShown.map(([a, b]) => (
        <line key={`m${edgeKey(a, b)}`} className="chr-mst-edge"
          x1={pos(a)[0]} y1={pos(a)[1]} x2={pos(b)[0]} y2={pos(b)[1]} />
      ))}
      {matchShown.map(([a, b]) => (
        <line key={`p${edgeKey(a, b)}`} className="chr-match-edge"
          x1={pos(a)[0]} y1={pos(a)[1]} x2={pos(b)[0]} y2={pos(b)[1]} />
      ))}

      {/* Multigraph (euler/shortcut): used edges fade */}
      {showMultigraph && multigraphEdges.map((e) => {
        const used = isEdgeUsed(e.a, e.b)
        const cls = `chr-multi-edge ${e.kind === 'mst' ? 'chr-multi-mst' : 'chr-multi-match'} ${used ? 'chr-multi-used' : ''}`
        return <line key={e.id} className={cls}
          x1={pos(e.a)[0]} y1={pos(e.a)[1]} x2={pos(e.b)[0]} y2={pos(e.b)[1]} />
      })}

      {/* Final tour (shortcut phase + done) */}
      {showTourLayer && tourEdges.map(([a, b]) => (
        <line key={`t${edgeKey(a, b)}`} className="chr-tour-edge"
          x1={pos(a)[0]} y1={pos(a)[1]} x2={pos(b)[0]} y2={pos(b)[1]} />
      ))}

      {/* Cities */}
      {cities.map(([x, y], id) => {
        const oddHighlight = phase === 'odd' || phase === 'matching'
        return (
          <g key={id}>
            <circle className={`chr-city ${sim.odd.includes(id) && oddHighlight ? 'chr-city-odd' : ''}`}
              cx={x} cy={y} r={7} />
            <text className="chr-label" x={x} y={y + 16}>{id}</text>
          </g>
        )
      })}

      {/* Walker dot */}
      {walkerCity !== null && (
        <circle className="chr-walker" cx={pos(walkerCity)[0]} cy={pos(walkerCity)[1]} r={5} />
      )}

      {/* Skipped repeat markers (shortcut) */}
      {[...skippedSet].map((idx) => {
        const city = sim.eulerCircuit[idx]
        const [x, y] = pos(city)
        return <text key={`sk${idx}`} className="chr-skip" x={x} y={y - 8}>✕</text>
      })}
    </svg>
  )
}

// ---------------------------------------------------------------
// Ratio meter — observed ratio vs the 1.5× bound
// ---------------------------------------------------------------
function RatioMeter({ ratio }: { ratio: number }) {
  const ratioClamped = Math.min(1.6, Math.max(1.0, ratio))
  const pct = ((ratioClamped - 1) / 0.6) * 100
  return (
    <div className="chr-meter">
      <div className="chr-meter-track">
        <div className="chr-meter-fill" style={{ width: `${Math.min(100, pct)}%` }} />
        <div className="chr-meter-mark chr-meter-mark-opt" />
        <div className="chr-meter-mark chr-meter-mark-bound" />
        <div className="chr-meter-dot" style={{ left: `${Math.min(100, pct)}%` }} />
      </div>
      <div className="chr-meter-labels">
        <span>1.0× OPT</span>
        <span>1.5× bound</span>
      </div>
      <div className="chr-meter-value">tour = {ratio.toFixed(2)}× optimal</div>
    </div>
  )
}

// ---------------------------------------------------------------
// Comparison rows — NN / doubled-MST / Christofides
// ---------------------------------------------------------------
function CompareRow({ label, cost, opt, best }: { label: string; cost: number; opt: number; best: boolean }) {
  const ratio = cost / opt
  const pct = Math.min(100, ((ratio - 1) / 0.6) * 100)
  return (
    <div className={`chr-compare-row ${best ? 'chr-compare-best' : ''}`}>
      <span className="chr-compare-label">{label}</span>
      <div className="chr-compare-track">
        <div className="chr-compare-fill" style={{ width: `${Math.max(0, pct)}%` }} />
      </div>
      <span className="chr-compare-value">{ratio.toFixed(2)}×</span>
    </div>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function ChristofidesExplainer() {
  const [sim, setSim] = useState<SimState>(() => makeInitState(DEFAULT_SCENARIO))
  const [running, setRunning] = useState(false)
  const [speedIdx, setSpeedIdx] = useState(2) // 3x default
  const [showMstLayer, setShowMstLayer] = useState(true)
  const [showMatchLayer, setShowMatchLayer] = useState(true)
  const [showTourLayer, setShowTourLayer] = useState(true)

  const simRef = useRef(sim)
  const historyRef = useRef<Array<typeof sim>>([])
  const scenarioRef = useRef<Instance>(DEFAULT_SCENARIO)

  const commit = useCallback((next: SimState) => {
    simRef.current = next
    setSim(next)
  }, [])

  const reinit = useCallback((instance: Instance) => {
    scenarioRef.current = instance
    historyRef.current = []
    commit(makeInitState(instance))
    setRunning(false)
  }, [commit])

  const stepForward = useCallback(() => {
    const cur = simRef.current
    if (cur.phase === 'done') return
    historyRef.current.push(structuredClone(cur))
    const next = stepOnce(cur)
    simRef.current = next
    setSim(next)
    if (next.phase === 'done') setRunning(false)
  }, [])

  const stepBack = useCallback(() => {
    const h = historyRef.current
    if (h.length === 0 || running) return
    commit(h.pop()!)
  }, [running, commit])

  useEffect(() => {
    if (!running) return
    const id = setInterval(stepForward, SPEEDS[speedIdx])
    return () => clearInterval(id)
  }, [running, speedIdx, stepForward])

  // Phase selector — jump straight to a phase's start (replays deterministically)
  const jumpTo = useCallback((target: Phase) => {
    let s = makeInitState(scenarioRef.current)
    let guard = 600
    while (s.phase !== target && s.phase !== 'done' && guard-- > 0) s = stepOnce(s)
    historyRef.current = []
    commit(s)
    setRunning(false)
  }, [commit])

  const s = sim
  const phaseIdx = PHASES.indexOf(s.phase)
  const matchShare = s.matchingCost / s.tourCost

  // Status chip
  let chipText = "Step 1 — growing the Minimum Spanning Tree (Prim's)"
  let chipClass = "chr-chip chr-chip-idle"
  if (s.phase === 'mst') {
    if (s.mstRevealed === 0) {
      chipText = `MST — click Step to grow the tree (Prim's, cost ${s.mstCost.toFixed(0)})`
    } else if (s.mstRevealed < s.mstEdges.length) {
      chipText = `MST — added edge ${s.mstEdges[s.mstRevealed - 1][0]}–${s.mstEdges[s.mstRevealed - 1][1]} (${s.mstRevealed}/${s.mstEdges.length})`
    } else {
      chipText = `MST complete — cost ${s.mstCost.toFixed(0)} ≤ OPT ${s.opt.toFixed(0)}. Step to find odd-degree vertices`
    }
  } else if (s.phase === 'odd') {
    chipText = `Odd-degree vertices: ${s.odd.join(', ')} — always an even count (handshaking lemma)`
    chipClass = "chr-chip chr-chip-odd"
  } else if (s.phase === 'matching') {
    chipText = s.matchingRevealed < s.matchingEdges.length
      ? `Matching — paired ${s.matchingEdges[s.matchingRevealed - 1][0]}–${s.matchingEdges[s.matchingRevealed - 1][1]} (${s.matchingRevealed}/${s.matchingEdges.length} pairs)`
      : `Matching complete — cost ${s.matchingCost.toFixed(0)}`
    chipClass = "chr-chip chr-chip-match"
  } else if (s.phase === 'euler') {
    chipText = `Eulerian circuit — walking ${s.eulerCircuit[s.walkerPos - 1] ?? '—'}→${s.eulerCircuit[s.walkerPos]} (${s.walkerPos}/${s.eulerCircuit.length - 1})`
    chipClass = "chr-chip chr-chip-euler"
  } else if (s.phase === 'shortcut') {
    const city = s.eulerCircuit[Math.min(s.shortcutStep, s.eulerCircuit.length - 1)]
    const fresh = s.kept.includes(city)
    chipText = fresh
      ? `Shortcut — ${city} added to the tour`
      : `Shortcut — ${city} already visited, skipped (✕)`
    chipClass = "chr-chip chr-chip-shortcut"
  } else if (s.phase === 'done') {
    chipText = `Done — tour cost ${s.tourCost.toFixed(0)} = ${s.ratio.toFixed(2)}× OPT (bound 1.5×)`
    chipClass = "chr-chip chr-chip-done"
  }

  return (
    <div className="chr-root">
      <style>{CSS}</style>

      <header className="chr-header">
        <div className="chr-eyebrow">teeline · algorithms/christofides</div>
        <h2 className="chr-title">Christofides — a ≤1.5× Approximation</h2>
        <p className="chr-sub">
          The only simple TSP heuristic with a <strong>provable worst-case bound</strong>: the tour is
          always within 1.5× of optimal. Three ingredients — <strong>MST</strong> (skeleton, ≤ OPT),
          <strong>matching</strong> on the odd-degree vertices (patch, ≤ OPT/2), and an
          <strong>Eulerian walk + shortcut</strong> (≤ MST + matching) — add up to the guarantee.
        </p>
      </header>

      <div className="chr-viz-row">
        <div className="chr-canvas-wrap">
          <TourCanvas sim={s} showMst={showMstLayer} showMatch={showMatchLayer} showTour={showTourLayer} />
        </div>
        <div className="chr-side">
          <div className="chr-section-label">Pipeline</div>
          <div className="chr-phase-list">
            {PHASES.map((p, i) => {
              const done = i < phaseIdx || s.phase === 'done'
              return (
                <button key={p} className={`chr-phase-btn ${i === phaseIdx ? 'chr-phase-cur' : ''} ${done ? 'chr-phase-done' : ''}`}
                  onClick={() => jumpTo(p)} disabled={running}>
                  <span className="chr-phase-num">{done ? '✓' : i + 1}</span>
                  {PHASE_LABELS[p]}
                </button>
              )
            })}
          </div>

          <div className="chr-section-label">Approximation ratio</div>
          <RatioMeter ratio={s.ratio} />

          <div className="chr-section-label">Why ≤ 1.5×?</div>
          <div className="chr-proof">
            <div className={`chr-proof-line ${phaseIdx >= 1 ? 'chr-proof-on' : ''}`}>
              <span className="chr-proof-tag">MST</span> {s.mstCost.toFixed(0)} ≤ OPT {s.opt.toFixed(0)}
            </div>
            <div className={`chr-proof-line ${phaseIdx >= 3 ? 'chr-proof-on' : ''}`}>
              <span className="chr-proof-tag">Match</span> {s.matchingCost.toFixed(0)} {s.matchingCost > s.opt / 2 ? '>' : '≤'} OPT/2 {(s.opt / 2).toFixed(0)}
              {s.matchingCost > s.opt / 2 && (
                <span className="chr-proof-note"> greedy &gt; theory — the 1.5× bound assumes the true minimum matching</span>
              )}
            </div>
            <div className={`chr-proof-line ${s.phase === 'done' ? 'chr-proof-on' : ''}`}>
              <span className="chr-proof-tag">Tour</span> {s.tourCost.toFixed(0)} ≤ MST+matching {s.eulerCost.toFixed(0)} ≤ 1.5·OPT {(1.5 * s.opt).toFixed(0)}
            </div>
          </div>

          <div className="chr-section-label">Compare on this instance</div>
          <div className="chr-compare">
            <CompareRow label="Nearest neighbour" cost={s.nnCost} opt={s.opt} best={s.nnCost <= s.approx2Cost && s.nnCost <= s.tourCost} />
            <CompareRow label="Doubled MST (2×)" cost={s.approx2Cost} opt={s.opt} best={s.approx2Cost < s.nnCost && s.approx2Cost <= s.tourCost} />
            <CompareRow label="Christofides" cost={s.tourCost} opt={s.opt} best={s.tourCost < s.nnCost && s.tourCost < s.approx2Cost} />
          </div>
        </div>
      </div>

      <div className="chr-legend">
        <span><span className="chr-swatch chr-swatch-mst" /> MST edge</span>
        <span><span className="chr-swatch chr-swatch-match" /> matching edge</span>
        <span><span className="chr-swatch chr-swatch-tour" /> final tour</span>
        <span><span className="chr-swatch chr-swatch-odd" /> odd-degree vertex</span>
        <span><span className="chr-swatch chr-swatch-walker" /> walker</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="chr-statgrid">
        <div><div className="chr-statlabel">phase</div><div className="chr-mono">{PHASE_LABELS[s.phase]}</div></div>
        <div><div className="chr-statlabel">MST cost</div><div className="chr-mono">{s.mstCost.toFixed(0)}</div></div>
        <div><div className="chr-statlabel">matching cost</div><div className="chr-mono">{s.matchingCost.toFixed(0)} ({Math.round(matchShare * 100)}%)</div></div>
        <div><div className="chr-statlabel">tour cost</div><div className="chr-mono">{s.tourCost.toFixed(0)}</div></div>
        <div><div className="chr-statlabel">ratio</div><div className="chr-mono">{s.ratio.toFixed(2)}×</div></div>
        <div><div className="chr-statlabel">step</div><div className="chr-mono">{s.step}</div></div>
      </div>

      <div className="chr-config">
        <div className="chr-config-row">
          <span className="chr-label">Show</span>
          <label className="chr-toggle"><input type="checkbox" checked={showMstLayer} onChange={(e) => setShowMstLayer((e.target as HTMLInputElement).checked)} /> MST</label>
          <label className="chr-toggle"><input type="checkbox" checked={showMatchLayer} onChange={(e) => setShowMatchLayer((e.target as HTMLInputElement).checked)} /> Matching</label>
          <label className="chr-toggle"><input type="checkbox" checked={showTourLayer} onChange={(e) => setShowTourLayer((e.target as HTMLInputElement).checked)} /> Tour</label>
        </div>
        <div className="chr-config-row">
          <span className="chr-label">Speed</span>
          <div className="chr-speed-btns">
            {SPEED_LABELS.map((l, i) => (
              <button key={l} className={`chr-speed-btn ${i === speedIdx ? 'chr-speed-btn-sel' : ''}`}
                onClick={() => setSpeedIdx(i)} disabled={running}>{l}</button>
            ))}
          </div>
        </div>
      </div>

      <div className="chr-controls">
        <button className="chr-btn" onClick={stepBack} disabled={running || s.step === 0}>⏴ Back</button>
        <button className="chr-btn" onClick={stepForward} disabled={running || s.phase === 'done'}>⏵ Step</button>
        <button className="chr-btn" onClick={() => setRunning(!running)} disabled={s.phase === 'done'}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="chr-btn" onClick={() => reinit(scenarioRef.current)} disabled={running}>↺ Reset</button>
      </div>

      <div className="chr-scenarios">
        <div className="chr-section-label">Scenarios</div>
        <div className="chr-scenario-row">
          {Object.entries(SCENARIOS).map(([key, inst]) => (
            <button key={key} className="chr-scenario-btn" title={inst.desc}
              onClick={() => reinit(inst)} disabled={running}>
              {inst.label}
            </button>
          ))}
        </div>
      </div>

      <footer className="chr-footer">
        <span className="chr-mono">cities: {s.n}</span>
        <span className="chr-mono">MST + matching + Euler + shortcut · ≤ 1.5× optimal</span>
      </footer>
    </div>
  )
}

const CSS = `
.chr-root {
  --accent: #0d9488;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 20px;
  max-width: 880px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.chr-root * { box-sizing: border-box; }
.chr-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

.chr-header { margin-bottom: 2px; }
.chr-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.chr-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.chr-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }

.chr-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.chr-bg { fill: var(--panel); }
.chr-mst-edge { stroke: #16a34a; stroke-width: 2.5; stroke-linecap: round; }
.chr-match-edge { stroke: #7c3aed; stroke-width: 2.5; stroke-dasharray: 6 4; }
.chr-multi-edge { stroke-width: 2; stroke-linecap: round; transition: opacity 0.2s; }
.chr-multi-mst { stroke: #16a34a; }
.chr-multi-match { stroke: #7c3aed; stroke-dasharray: 6 4; }
.chr-multi-used { opacity: 0.18; }
.chr-tour-edge { stroke: #1f2328; stroke-width: 3; stroke-linecap: round; }
.chr-city { fill: #1f2937; stroke: #fff; stroke-width: 1.5; }
.chr-city-odd { fill: #ef4444; animation: chr-pulse 1s ease-in-out infinite; }
.chr-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: #374151; text-anchor: middle;
  paint-order: stroke fill; stroke: white; stroke-width: 3;
  pointer-events: none; user-select: none;
}
.chr-walker { fill: #f59e0b; stroke: #fff; stroke-width: 2; }
.chr-skip {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px; fill: #ef4444; font-weight: 700; text-anchor: middle;
}
@keyframes chr-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.35; }
}

.chr-viz-row { display: flex; gap: 12px; align-items: stretch; }
.chr-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
.chr-side { width: 260px; flex-shrink: 0; display: flex; flex-direction: column; gap: 8px; }

.chr-section-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }

.chr-phase-list { display: flex; flex-direction: column; gap: 3px; }
.chr-phase-btn {
  display: flex; align-items: center; gap: 8px; text-align: left;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 4px 8px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--muted);
  cursor: pointer;
}
.chr-phase-btn:hover:not(:disabled) { border-color: var(--accent); }
.chr-phase-btn:disabled { opacity: 0.5; cursor: default; }
.chr-phase-cur { border-color: var(--accent); color: #115e59; font-weight: 600; background: #f0fdf4; }
.chr-phase-done { color: #166534; }
.chr-phase-num {
  display: inline-flex; align-items: center; justify-content: center;
  width: 16px; height: 16px; border-radius: 50%;
  background: var(--panel); border: 1px solid var(--line);
  font-size: 0.68rem;
}
.chr-phase-done .chr-phase-num { background: #dcfce7; border-color: #86efac; color: #166534; }
.chr-phase-cur .chr-phase-num { background: #ccfbf1; border-color: var(--accent); }

.chr-meter-track {
  position: relative; height: 8px; border-radius: 4px;
  background: linear-gradient(90deg, #bbf7d0, #fef08a 60%, #fecaca);
  border: 1px solid var(--line);
}
.chr-meter-fill { position: absolute; top: 0; left: 0; bottom: 0; border-radius: 4px; background: #0d9488; opacity: 0.25; }
.chr-meter-dot {
  position: absolute; top: -3px; width: 12px; height: 12px; margin-left: -6px;
  border-radius: 50%; background: #0d9488; border: 2px solid #fff;
}
.chr-meter-mark {
  position: absolute; top: -2px; bottom: -2px; width: 2px; background: #64748b;
}
.chr-meter-mark-opt { left: 0; }
.chr-meter-mark-bound { left: 83.3%; background: #dc2626; }
.chr-meter-labels {
  display: flex; justify-content: space-between; font-size: 0.68rem; color: var(--muted);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.chr-meter-value {
  font-size: 0.78rem; font-weight: 600; color: var(--text);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

.chr-proof { display: flex; flex-direction: column; gap: 4px; }
.chr-proof-line {
  font-size: 0.76rem; color: var(--muted); opacity: 0.45;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  transition: opacity 0.3s;
}
.chr-proof-on { opacity: 1; color: var(--text); }
.chr-proof-tag {
  display: inline-block; min-width: 44px; font-weight: 700;
  color: var(--accent);
}
.chr-proof-note { color: #b45309; }

.chr-compare { display: flex; flex-direction: column; gap: 4px; }
.chr-compare-row {
  display: flex; align-items: center; gap: 8px; font-size: 0.74rem;
  padding: 3px 6px; border-radius: 6px;
}
.chr-compare-best { background: #f0fdf4; }
.chr-compare-label { width: 110px; color: var(--muted); flex-shrink: 0; }
.chr-compare-track {
  flex: 1; height: 6px; border-radius: 3px; background: var(--panel);
  border: 1px solid var(--line);
}
.chr-compare-fill { height: 100%; border-radius: 3px; background: #94a3b8; }
.chr-compare-best .chr-compare-fill { background: #0d9488; }
.chr-compare-value {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.72rem; width: 48px; text-align: right;
}

.chr-legend {
  display: flex; flex-wrap: wrap; gap: 6px 14px; font-size: 0.78rem; color: var(--muted);
}
.chr-swatch {
  display: inline-block; width: 14px; height: 3px; border-radius: 2px;
  margin-right: 4px; vertical-align: middle;
}
.chr-swatch-mst { background: #16a34a; }
.chr-swatch-match { background: #7c3aed; }
.chr-swatch-tour { background: #1f2328; }
.chr-swatch-odd { background: #ef4444; }
.chr-swatch-walker { background: #f59e0b; }

.chr-chip {
  font-size: 0.82rem; padding: 6px 12px; border-radius: 8px;
  font-weight: 500; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.chr-chip-idle { background: #f1f5f9; color: var(--muted); }
.chr-chip-odd { background: #fee2e2; color: #991b1b; }
.chr-chip-match { background: #f3e8ff; color: #6b21a8; }
.chr-chip-euler { background: #ecfdf5; color: #065f46; }
.chr-chip-shortcut { background: #ffedd5; color: #9a3412; }
.chr-chip-done { background: #dcfce7; color: #166534; }

.chr-statgrid {
  display: flex; flex-wrap: wrap; gap: 14px 22px;
  font-size: 0.82rem;
}
.chr-statlabel { color: var(--muted); font-size: 0.72em; text-transform: uppercase; letter-spacing: 0.06em; }

.chr-config {
  display: flex; flex-direction: column; gap: 8px;
  padding: 12px; background: var(--panel); border-radius: 8px;
}
.chr-config-row {
  display: flex; align-items: center; gap: 12px; flex-wrap: wrap;
}
.chr-label { font-size: 0.82rem; font-weight: 500; color: var(--text); }
.chr-toggle { font-size: 0.78rem; color: var(--text); display: flex; align-items: center; gap: 4px; cursor: pointer; }
.chr-speed-btns { display: flex; gap: 4px; }
.chr-speed-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.chr-speed-btn:hover:not(:disabled) { background: #f0fdf4; }
.chr-speed-btn:disabled { opacity: 0.4; cursor: default; }
.chr-speed-btn-sel { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.chr-controls { display: flex; gap: 8px; }
.chr-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem; padding: 6px 14px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.chr-btn:hover:not(:disabled) { background: #f0fdf4; }
.chr-btn:disabled { opacity: 0.4; cursor: default; }

.chr-scenarios { display: flex; flex-direction: column; gap: 6px; }
.chr-scenario-row { display: flex; flex-wrap: wrap; gap: 6px; }
.chr-scenario-btn {
  font-size: 0.8rem; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.chr-scenario-btn:hover:not(:disabled) { background: #f0fdf4; border-color: var(--accent); }
.chr-scenario-btn:disabled { opacity: 0.4; cursor: default; }

.chr-footer {
  display: flex; gap: 12px; justify-content: center;
  padding-top: 8px; border-top: 1px solid var(--line);
  color: var(--muted); font-size: 0.78rem;
}
`

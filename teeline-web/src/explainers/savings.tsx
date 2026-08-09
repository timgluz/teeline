import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  CITIES, N_CITIES,
  components, makeInitState, stepOnce,
} from "./savings-algo"
import type { Edge, EventMode, RejectReason } from "./savings-algo"

const COMP_PALETTE = [
  "#0d9488", "#2563eb", "#7c3aed", "#db2777", "#ea580c",
  "#16a34a", "#0891b2", "#ca8a04", "#dc2626", "#4f46e5",
  "#0f766e", "#9333ea",
]
function compColor(rootIndex: number): string {
  return COMP_PALETTE[rootIndex % COMP_PALETTE.length]
}

function edgeKey(e: Edge): string {
  return `${e.u}-${e.v}`
}

interface ScanCanvasProps {
  accepted: Edge[]
  lastEdge: Edge | null
  lastEvent: EventMode | null
  rejectedTrail: Array<{ edge: Edge; reason: RejectReason }>
  degree: number[]
  parent: number[]
  hubIdx: number
  tour: number[] | null
}
function ScanCanvas({ accepted, lastEdge, lastEvent, rejectedTrail, degree, parent, hubIdx, tour }: ScanCanvasProps) {
  const comps = useMemo(() => components(parent), [parent])
  const rootOf = useMemo(() => {
    const m = new Map<number, number>()
    comps.forEach((group, gi) => group.forEach(c => m.set(c, gi)))
    return m
  }, [comps])

  const acceptedSet = useMemo(() => new Set(accepted.map(edgeKey)), [accepted])

  const tourPts = tour
    ? tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ") + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}`
    : ""

  return (
    <svg viewBox="0 0 300 300" className="sav-canvas" role="img" aria-label="Savings edge scan">
      <rect x={0} y={0} width={300} height={300} className="sav-bg" />

      {tour && <polygon points={tourPts} className="sav-tour" />}

      {accepted.map((e, i) => (
        <line key={"a" + i}
          x1={CITIES[e.u][0]} y1={CITIES[e.u][1]}
          x2={CITIES[e.v][0]} y2={CITIES[e.v][1]}
          className="sav-accepted"
        />
      ))}

      {rejectedTrail.map((r, i) => (
        <line key={"r" + i}
          x1={CITIES[r.edge.u][0]} y1={CITIES[r.edge.u][1]}
          x2={CITIES[r.edge.v][0]} y2={CITIES[r.edge.v][1]}
          className={r.reason === "degree" ? "sav-rejected sav-rejected-degree" : "sav-rejected sav-rejected-cycle"}
        />
      ))}

      {lastEdge && !acceptedSet.has(edgeKey(lastEdge)) && !tour && (
        <line
          x1={CITIES[lastEdge.u][0]} y1={CITIES[lastEdge.u][1]}
          x2={CITIES[lastEdge.v][0]} y2={CITIES[lastEdge.v][1]}
          className={
            lastEvent === "rejected-degree" ? "sav-cand sav-cand-degree"
            : lastEvent === "rejected-cycle" ? "sav-cand sav-cand-cycle"
            : lastEvent === "closing" ? "sav-cand sav-cand-closing"
            : "sav-cand sav-cand-accept"
          }
        />
      )}

      {CITIES.map(([x, y], i) => {
        const gi = rootOf.get(i) ?? 0
        const full = degree[i] >= 2
        return (
          <g key={i}>
            {i === hubIdx && (
              <circle cx={x} cy={y} r={11} className="sav-hub-ring" />
            )}
            <circle cx={x} cy={y} r={5.5}
              fill={compColor(gi)}
              className={full ? "sav-city sav-city-full" : "sav-city"}
            />
            {i === hubIdx && (
              <text x={x} y={y + 3} className="sav-hub-star">★</text>
            )}
            <text x={x + 7} y={y - 6} className="sav-city-label">{i}</text>
            <text x={x} y={y + 18} className="sav-degree-badge"
              style={{ opacity: degree[i] > 0 ? 1 : 0.25 }}>{degree[i]}</text>
          </g>
        )
      })}
    </svg>
  )
}

function ComponentPanel({ parent }: { parent: number[] }) {
  const comps = useMemo(() => components(parent), [parent])
  return (
    <div className="sav-list-panel">
      <div className="sav-list-title">Union-Find</div>
      <div className="sav-list-subtitle">{comps.length} component{comps.length === 1 ? "" : "s"}</div>
      {comps.map((g, i) => (
        <div key={i} className="sav-comp-badge" style={{ borderLeftColor: compColor(i) }}>
          {"{" + g.join(",") + "}"}
        </div>
      ))}
    </div>
  )
}

export default function SavingsExplainer() {
  const [speed, setSpeed] = useState(5)

  const simRef = useRef(makeInitState())
  const [accepted, setAccepted] = useState<Edge[]>(() => [])
  const [rejected, setRejected] = useState<Array<{ edge: Edge; reason: RejectReason }>>(() => [])
  const [degree, setDegree] = useState<number[]>(() => new Array(N_CITIES).fill(0))
  const [parent, setParent] = useState<number[]>(() => simRef.current.parent.slice())
  const [hubIdx, setHubIdx] = useState<number>(() => simRef.current.hubIndex)
  const [lastEdge, setLastEdge] = useState<Edge | null>(null)
  const [lastEvent, setLastEvent] = useState<EventMode | null>(null)
  const [step, setStep] = useState(0)
  const [done, setDone] = useState(false)
  const [tour, setTour] = useState<number[] | null>(null)
  const [running, setRunning] = useState(false)

  const reinit = useCallback(() => {
    simRef.current = makeInitState()
    setAccepted([]); setRejected([]); setDegree(new Array(N_CITIES).fill(0))
    setParent(simRef.current.parent.slice()); setLastEdge(null); setLastEvent(null)
    setHubIdx(simRef.current.hubIndex)
    setStep(0); setDone(false); setTour(null); setRunning(false)
  }, [])

  const step_fn = useCallback(() => {
    const next = stepOnce(simRef.current)
    simRef.current = next
    setAccepted(next.accepted.slice())
    setRejected(next.rejected.slice(-6))
    setDegree(next.degree.slice())
    setParent(next.parent.slice())
    setLastEdge(next.lastEdge); setLastEvent(next.lastEvent)
    setStep(next.step); setDone(next.done); setTour(next.tour)
    if (next.done) setRunning(false)
  }, [])

  useEffect(() => {
    if (!running) return
    const ms = Math.max(40, 660 - speed * 66)
    const id = setInterval(step_fn, ms)
    return () => clearInterval(id)
  }, [running, speed, step_fn])

  const partialCost = useMemo(
    () => accepted.reduce((s, e) => s + e.dist, 0),
    [accepted],
  )

  let chipText = "Press Step or Run to scan the highest-savings edges first"
  let chipClass = "sav-chip sav-chip-idle"
  if (lastEvent === "accepted" && lastEdge) {
    chipText = `✅ accepted  (${lastEdge.u}, ${lastEdge.v})  — saving = +${lastEdge.val.toFixed(1)}`
    chipClass = "sav-chip sav-chip-accept"
  } else if (lastEvent === "rejected-degree" && lastEdge) {
    chipText = `✗ rejected  (${lastEdge.u}, ${lastEdge.v})  — would give a city degree 3`
    chipClass = "sav-chip sav-chip-degree"
  } else if (lastEvent === "rejected-cycle" && lastEdge) {
    chipText = `✗ rejected  (${lastEdge.u}, ${lastEdge.v})  — premature sub-cycle`
    chipClass = "sav-chip sav-chip-cycle"
  } else if (lastEvent === "closing" && lastEdge) {
    chipText = `🔒 closing edge  (${lastEdge.u}, ${lastEdge.v})  — the path becomes a cycle`
    chipClass = "sav-chip sav-chip-closing"
  }

  const totalEdges = (N_CITIES * (N_CITIES - 1)) / 2
  const comps = useMemo(() => components(parent), [parent])
  const hubCityId = hubIdx

  return (
    <div className="sav-root">
      <style>{CSS}</style>

      <header className="sav-header">
        <div className="sav-eyebrow">teeline · algorithms/savings</div>
        <h2 className="sav-title">Savings Construction</h2>
        <p className="sav-sub">
          Every pairwise edge is ranked by <strong>Clarke-Wright savings</strong> s(i,j) = d(hub,i) + d(hub,j) − d(i,j)
          and scanned highest-first. The <strong>hub city</strong> (★, nearest the centroid) is visited like any other
          city — it only biases the <em>ordering</em> of candidate merges. Acceptance rules reuse the same
          Kruskal-style scan as Greedy Edge (degree ≤ 2, no premature sub-cycle).
        </p>
      </header>

      <div className="sav-viz-row">
        <div className="sav-canvas-wrap">
          <ScanCanvas
            accepted={accepted} lastEdge={lastEdge} lastEvent={lastEvent}
            rejectedTrail={rejected} degree={degree} parent={parent}
            hubIdx={hubIdx} tour={tour}
          />
        </div>
        <ComponentPanel parent={parent} />
      </div>

      <div className="sav-legend">
        <span><span className="sav-swatch sav-swatch-accepted" /> accepted edge</span>
        <span><span className="sav-swatch sav-swatch-degree" /> rejected — degree</span>
        <span><span className="sav-swatch sav-swatch-cycle" /> rejected — cycle</span>
        <span><span className="sav-swatch sav-swatch-closing" /> closing edge</span>
        <span><span className="sav-swatch sav-swatch-hub" /> hub (city {hubCityId})</span>
        {done && <span><span className="sav-swatch sav-swatch-tour" /> final tour</span>}
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="sav-statgrid">
        <div>
          <div className="sav-statlabel">edges scanned</div>
          <div className="sav-mono">{accepted.length + rejected.length}/{totalEdges}</div>
        </div>
        <div>
          <div className="sav-statlabel">accepted</div>
          <div className="sav-mono">{accepted.length}/{N_CITIES}</div>
        </div>
        <div>
          <div className="sav-statlabel">rejected</div>
          <div className="sav-mono">{step > 0 ? simRef.current.rejected.length : 0}</div>
        </div>
        <div>
          <div className="sav-statlabel">components</div>
          <div className="sav-mono">{comps.length}</div>
        </div>
        <div>
          <div className="sav-statlabel">partial cost</div>
          <div className="sav-mono">{partialCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="sav-statlabel">step</div>
          <div className="sav-mono">{step}</div>
        </div>
      </div>

      <div className="sav-config">
        <div className="sav-config-row">
          <label className="sav-config-label">Speed</label>
          <input type="range" min={1} max={10} step={1} value={speed}
            className="sav-slider"
            onInput={e => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
          <div className="sav-hint">Savings is parameter-free — no other knobs to tune.</div>
        </div>
      </div>

      <div className="sav-controls">
        <button className="sav-btn" onClick={step_fn} disabled={running || done}>◀ Step</button>
        <button className={`sav-btn ${!running ? "sav-btn-primary" : ""}`}
          onClick={() => setRunning(r => !r)} disabled={done}>
          {running ? "⏸ Pause" : done ? "✓ Done" : "▶ Run"}
        </button>
        <button className="sav-btn" onClick={reinit}>↺ Reset</button>
      </div>

      <footer className="sav-footer">
        <span className="sav-mono">cities: {N_CITIES}</span>
        <span className="sav-mono">hub: city {hubIdx}</span>
        <span className="sav-mono">candidate edges: {totalEdges}</span>
        <span className="sav-mono">sort: savings, descending</span>
      </footer>
    </div>
  )
}

const CSS = `
.sav-root {
  --accent: #0d9488;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --accept: #16a34a;
  --closing: #ca8a04;
  --reject-degree: #db2777;
  --reject-cycle: #ea580c;
  --tour: #2563eb;
  --hub: #dc2626;
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
.sav-root * { box-sizing: border-box; }
.sav-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

.sav-header { margin-bottom: 2px; }
.sav-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.sav-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.sav-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.sav-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(13,148,136,0.1);
  color: var(--accent); padding: 1px 4px; border-radius: 4px;
}

.sav-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.sav-bg { fill: var(--panel); }
.sav-tour { fill: none; stroke: var(--tour); stroke-width: 3; stroke-linejoin: round; opacity: 0.35; }
.sav-accepted { stroke: var(--accept); stroke-width: 2.5; stroke-linecap: round; }
.sav-rejected { stroke-width: 1.4; stroke-dasharray: 4 3; opacity: 0.4; }
.sav-rejected-degree { stroke: var(--reject-degree); }
.sav-rejected-cycle  { stroke: var(--reject-cycle); }
.sav-cand { stroke-width: 3.2; stroke-linecap: round; }
.sav-cand-accept  { stroke: var(--accept); }
.sav-cand-closing { stroke: var(--closing); }
.sav-cand-degree  { stroke: var(--reject-degree); }
.sav-cand-cycle   { stroke: var(--reject-cycle); }
.sav-city { stroke: #fff; stroke-width: 1.2; }
.sav-city-full { stroke: #fff; stroke-width: 2; }
.sav-hub-ring { fill: none; stroke: var(--hub); stroke-width: 1.8; }
.sav-hub-star {
  font-size: 11px; fill: var(--hub); text-anchor: middle; dominant-baseline: middle;
  pointer-events: none; user-select: none;
}
.sav-city-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8px; fill: #374151; stroke: white; stroke-width: 2.5;
  paint-order: stroke fill; dominant-baseline: auto;
  pointer-events: none; user-select: none;
}
.sav-degree-badge {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8.5px; font-weight: 700; fill: #1f2328;
  text-anchor: middle; pointer-events: none; user-select: none;
}

.sav-viz-row { display: flex; gap: 10px; align-items: stretch; }
.sav-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

.sav-list-panel {
  width: 130px; flex-shrink: 0; display: flex; flex-direction: column; gap: 4px;
  padding: 8px; background: var(--panel); border: 1px solid var(--line); border-radius: 8px;
  overflow-y: auto; max-height: 300px;
}
.sav-list-title {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;
  color: var(--muted); margin-bottom: 0;
}
.sav-list-subtitle {
  font-size: 0.7rem; color: var(--muted); margin-bottom: 4px;
}
.sav-comp-badge {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.72rem; font-weight: 600;
  background: #fff; color: var(--text);
  border-left: 3px solid var(--accent);
  border-radius: 3px; padding: 2px 6px;
}

.sav-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); align-items: center; }
.sav-swatch {
  display: inline-block; width: 22px; height: 3px;
  border-radius: 2px; margin-right: 3px; vertical-align: middle;
}
.sav-swatch-accept  { background: var(--accept); }
.sav-swatch-degree  { background: var(--reject-degree); }
.sav-swatch-cycle   { background: var(--reject-cycle); }
.sav-swatch-closing { background: var(--closing); }
.sav-swatch-hub     { background: var(--hub); }
.sav-swatch-tour    { background: var(--tour); }
.sav-swatch-accepted{ background: var(--accept); }

.sav-chip {
  font-size: 0.85rem; font-weight: 600; padding: 8px 12px;
  border-radius: 8px; border: 1px solid transparent;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.sav-chip-idle   { background: var(--panel); color: var(--muted); border-color: var(--line); font-weight: 400; }
.sav-chip-accept { background: #dcfce7; color: #15803d; border-color: #bbf7d0; }
.sav-chip-degree { background: #fce7f3; color: #be185d; border-color: #fbcfe8; }
.sav-chip-cycle  { background: #ffedd5; color: #c2410c; border-color: #fed7aa; }
.sav-chip-closing{ background: #fef3c7; color: #92400e; border-color: #fde68a; }

.sav-statgrid { display: grid; grid-template-columns: repeat(6, 1fr); gap: 8px; }
@media (max-width: 540px) { .sav-statgrid { grid-template-columns: repeat(3, 1fr); } }
.sav-statlabel {
  font-size: 0.65rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}

.sav-config { display: flex; flex-direction: column; gap: 8px; }
.sav-config-row { display: flex; flex-direction: column; gap: 3px; }
.sav-config-label { font-size: 0.88rem; }
.sav-slider { width: 100%; accent-color: var(--accent); cursor: pointer; }
.sav-hint { font-size: 0.75rem; color: var(--muted); }

.sav-controls { display: flex; gap: 8px; }
.sav-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer; font-family: inherit;
}
.sav-btn:hover:not(:disabled) { border-color: var(--accent); }
.sav-btn:disabled { opacity: 0.45; cursor: default; }
.sav-btn-primary { color: var(--accent); border-color: var(--accent); }

.sav-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 8px; border-top: 1px solid var(--line); color: var(--muted);
}
`

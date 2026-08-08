import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  CITIES, N_CITIES, components, makeInitState, stepOnce,
} from "./greedy-edge-algo"
import type { Edge, EventMode, RejectReason } from "./greedy-edge-algo"

// One stable colour per component root, so merges are visible as two groups
// collapsing into one colour. Sized for N_CITIES components (the start state).
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

// ---------------------------------------------------------------
// ScanCanvas — cities (coloured by component, degree badge), accepted
// edges, the candidate edge under evaluation, and the final tour.
// ---------------------------------------------------------------
interface ScanCanvasProps {
  accepted: Edge[]
  lastEdge: Edge | null
  lastEvent: EventMode | null
  rejectedTrail: Array<{ edge: Edge; reason: RejectReason }>
  degree: number[]
  parent: number[]
  tour: number[] | null
}
function ScanCanvas({ accepted, lastEdge, lastEvent, rejectedTrail, degree, parent, tour }: ScanCanvasProps) {
  const comps = useMemo(() => components(parent), [parent])
  // root lookup: city -> index into comps (its component slot)
  const rootOf = useMemo(() => {
    const m = new Map<number, number>()
    comps.forEach((group, gi) => group.forEach(c => m.set(c, gi)))
    return m
  }, [comps])

  const acceptedSet = useMemo(() => new Set(accepted.map(edgeKey)), [accepted])

  // tour as a closed polyline (only once done)
  const tourPts = tour
    ? tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ") + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}`
    : ""

  return (
    <svg viewBox="0 0 300 300" className="gec-canvas" role="img" aria-label="Greedy edge scan">
      <rect x={0} y={0} width={300} height={300} className="gec-bg" />

      {/* final closed tour underlay once complete */}
      {tour && <polygon points={tourPts} className="gec-tour" />}

      {/* accepted edges */}
      {accepted.map((e, i) => (
        <line key={"a" + i}
          x1={CITIES[e.u][0]} y1={CITIES[e.u][1]}
          x2={CITIES[e.v][0]} y2={CITIES[e.v][1]}
          className="gec-accepted"
        />
      ))}

      {/* faint trail of recent rejects (last few), so the scan is legible */}
      {rejectedTrail.map((r, i) => (
        <line key={"r" + i}
          x1={CITIES[r.edge.u][0]} y1={CITIES[r.edge.u][1]}
          x2={CITIES[r.edge.v][0]} y2={CITIES[r.edge.v][1]}
          className={r.reason === "degree" ? "gec-rejected gec-rejected-degree" : "gec-rejected gec-rejected-cycle"}
        />
      ))}

      {/* the candidate edge currently under evaluation (bold) */}
      {lastEdge && !acceptedSet.has(edgeKey(lastEdge)) && !tour && (
        <line
          x1={CITIES[lastEdge.u][0]} y1={CITIES[lastEdge.u][1]}
          x2={CITIES[lastEdge.v][0]} y2={CITIES[lastEdge.v][1]}
          className={
            lastEvent === "rejected-degree" ? "gec-cand gec-cand-degree"
            : lastEvent === "rejected-cycle" ? "gec-cand gec-cand-cycle"
            : lastEvent === "closing" ? "gec-cand gec-cand-closing"
            : "gec-cand gec-cand-accept"
          }
        />
      )}

      {/* cities: fill = component colour, badge = degree */}
      {CITIES.map(([x, y], i) => {
        const gi = rootOf.get(i) ?? 0
        const full = degree[i] >= 2
        return (
          <g key={i}>
            <circle cx={x} cy={y} r={5.5}
              fill={compColor(gi)}
              className={full ? "gec-city gec-city-full" : "gec-city"}
            />
            <text x={x + 7} y={y - 6} className="gec-city-label">{i}</text>
            <text x={x} y={y + 18} className="gec-degree-badge"
              style={{ opacity: degree[i] > 0 ? 1 : 0.25 }}>{degree[i]}</text>
          </g>
        )
      })}
    </svg>
  )
}

// ---------------------------------------------------------------
// ComponentPanel — union-find component groups (analogous to the tabu list)
// ---------------------------------------------------------------
function ComponentPanel({ parent }: { parent: number[] }) {
  const comps = useMemo(() => components(parent), [parent])
  return (
    <div className="gec-list-panel">
      <div className="gec-list-title">Union-Find</div>
      <div className="gec-list-subtitle">{comps.length} component{comps.length === 1 ? "" : "s"}</div>
      {comps.map((g, i) => (
        <div key={i} className="gec-comp-badge" style={{ borderLeftColor: compColor(i) }}>
          {"{" + g.join(",") + "}"}
        </div>
      ))}
    </div>
  )
}

// ---------------------------------------------------------------
// GreedyEdgeExplainer — main component
// ---------------------------------------------------------------
export default function GreedyEdgeExplainer() {
  const [speed, setSpeed] = useState(5)

  const simRef = useRef(makeInitState())
  const [accepted, setAccepted] = useState<Edge[]>(() => [])
  const [rejected, setRejected] = useState<Array<{ edge: Edge; reason: RejectReason }>>(() => [])
  const [degree, setDegree] = useState<number[]>(() => new Array(N_CITIES).fill(0))
  const [parent, setParent] = useState<number[]>(() => simRef.current.parent.slice())
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
    setStep(0); setDone(false); setTour(null); setRunning(false)
  }, [])

  const step_fn = useCallback(() => {
    const next = stepOnce(simRef.current)
    simRef.current = next
    setAccepted(next.accepted.slice())
    // keep only the last 6 rejects visible, so the canvas stays legible
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

  let chipText = "Press Step or Run to scan the shortest edges first"
  let chipClass = "gec-chip gec-chip-idle"
  if (lastEvent === "accepted" && lastEdge) {
    chipText = `✅ accepted  (${lastEdge.u}, ${lastEdge.v})  — degree/union updated`
    chipClass = "gec-chip gec-chip-accept"
  } else if (lastEvent === "rejected-degree" && lastEdge) {
    chipText = `✗ rejected  (${lastEdge.u}, ${lastEdge.v})  — would give a city degree 3`
    chipClass = "gec-chip gec-chip-degree"
  } else if (lastEvent === "rejected-cycle" && lastEdge) {
    chipText = `✗ rejected  (${lastEdge.u}, ${lastEdge.v})  — premature sub-cycle`
    chipClass = "gec-chip gec-chip-cycle"
  } else if (lastEvent === "closing" && lastEdge) {
    chipText = `🔒 closing edge  (${lastEdge.u}, ${lastEdge.v})  — the path becomes a cycle`
    chipClass = "gec-chip gec-chip-closing"
  }

  const totalEdges = (N_CITIES * (N_CITIES - 1)) / 2
  const comps = useMemo(() => components(parent), [parent])

  return (
    <div className="gec-root">
      <style>{CSS}</style>

      <header className="gec-header">
        <div className="gec-eyebrow">teeline · algorithms/greedy-edge</div>
        <h2 className="gec-title">Greedy Edge Construction</h2>
        <p className="gec-sub">
          Every pairwise edge is scanned shortest-first and accepted unless it would give a city
          <strong> degree 3+</strong> or close a <strong>premature sub-cycle</strong> (tracked by a
          union-find). Watch the components merge until one Hamiltonian cycle remains.
        </p>
      </header>

      <div className="gec-viz-row">
        <div className="gec-canvas-wrap">
          <ScanCanvas
            accepted={accepted} lastEdge={lastEdge} lastEvent={lastEvent}
            rejectedTrail={rejected} degree={degree} parent={parent} tour={tour}
          />
        </div>
        <ComponentPanel parent={parent} />
      </div>

      <div className="gec-legend">
        <span><span className="gec-swatch gec-swatch-accepted" /> accepted edge</span>
        <span><span className="gec-swatch gec-swatch-degree" /> rejected — degree</span>
        <span><span className="gec-swatch gec-swatch-cycle" /> rejected — cycle</span>
        <span><span className="gec-swatch gec-swatch-closing" /> closing edge</span>
        {done && <span><span className="gec-swatch gec-swatch-tour" /> final tour</span>}
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="gec-statgrid">
        <div>
          <div className="gec-statlabel">edges scanned</div>
          <div className="gec-mono">{accepted.length + rejected.length}/{totalEdges}</div>
        </div>
        <div>
          <div className="gec-statlabel">accepted</div>
          <div className="gec-mono">{accepted.length}/{N_CITIES}</div>
        </div>
        <div>
          <div className="gec-statlabel">rejected</div>
          <div className="gec-mono">{step > 0 ? simRef.current.rejected.length : 0}</div>
        </div>
        <div>
          <div className="gec-statlabel">components</div>
          <div className="gec-mono">{comps.length}</div>
        </div>
        <div>
          <div className="gec-statlabel">partial cost</div>
          <div className="gec-mono">{partialCost.toFixed(0)}</div>
        </div>
        <div>
          <div className="gec-statlabel">step</div>
          <div className="gec-mono">{step}</div>
        </div>
      </div>

      <div className="gec-config">
        <div className="gec-config-row">
          <label className="gec-config-label">Speed</label>
          <input type="range" min={1} max={10} step={1} value={speed}
            className="gec-slider"
            onInput={e => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
          <div className="gec-hint">Edges are parameter-free — no other knobs to tune.</div>
        </div>
      </div>

      <div className="gec-controls">
        <button className="gec-btn" onClick={step_fn} disabled={running || done}>◀ Step</button>
        <button className={`gec-btn ${!running ? "gec-btn-primary" : ""}`}
          onClick={() => setRunning(r => !r)} disabled={done}>
          {running ? "⏸ Pause" : done ? "✓ Done" : "▶ Run"}
        </button>
        <button className="gec-btn" onClick={reinit}>↺ Reset</button>
      </div>

      <footer className="gec-footer">
        <span className="gec-mono">cities: {N_CITIES}</span>
        <span className="gec-mono">candidate edges: {totalEdges}</span>
        <span className="gec-mono">sort: distance, ascending</span>
      </footer>
    </div>
  )
}

const CSS = `
.gec-root {
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
.gec-root * { box-sizing: border-box; }
.gec-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

/* Header */
.gec-header { margin-bottom: 2px; }
.gec-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.gec-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.gec-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.gec-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(13,148,136,0.1);
  color: var(--accent); padding: 1px 4px; border-radius: 4px;
}

/* Canvas */
.gec-canvas {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.gec-bg { fill: var(--panel); }
.gec-tour { fill: none; stroke: var(--tour); stroke-width: 3; stroke-linejoin: round; opacity: 0.35; }
.gec-accepted { stroke: var(--accept); stroke-width: 2.5; stroke-linecap: round; }
.gec-rejected { stroke-width: 1.4; stroke-dasharray: 4 3; opacity: 0.4; }
.gec-rejected-degree { stroke: var(--reject-degree); }
.gec-rejected-cycle  { stroke: var(--reject-cycle); }
.gec-cand { stroke-width: 3.2; stroke-linecap: round; }
.gec-cand-accept  { stroke: var(--accept); }
.gec-cand-closing { stroke: var(--closing); }
.gec-cand-degree  { stroke: var(--reject-degree); }
.gec-cand-cycle   { stroke: var(--reject-cycle); }
.gec-city { stroke: #fff; stroke-width: 1.2; }
.gec-city-full { stroke: #fff; stroke-width: 2; }
.gec-city-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8px; fill: #374151; stroke: white; stroke-width: 2.5;
  paint-order: stroke fill; dominant-baseline: auto;
  pointer-events: none; user-select: none;
}
.gec-degree-badge {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8.5px; font-weight: 700; fill: #1f2328;
  text-anchor: middle; pointer-events: none; user-select: none;
}

/* Viz row */
.gec-viz-row { display: flex; gap: 10px; align-items: stretch; }
.gec-canvas-wrap { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }

/* Component panel */
.gec-list-panel {
  width: 130px; flex-shrink: 0; display: flex; flex-direction: column; gap: 4px;
  padding: 8px; background: var(--panel); border: 1px solid var(--line); border-radius: 8px;
  overflow-y: auto; max-height: 300px;
}
.gec-list-title {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;
  color: var(--muted); margin-bottom: 0;
}
.gec-list-subtitle {
  font-size: 0.7rem; color: var(--muted); margin-bottom: 4px;
}
.gec-comp-badge {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.72rem; font-weight: 600;
  background: #fff; color: var(--text);
  border-left: 3px solid var(--accent);
  border-radius: 3px; padding: 2px 6px;
}

/* Legend */
.gec-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); align-items: center; }
.gec-swatch {
  display: inline-block; width: 22px; height: 3px;
  border-radius: 2px; margin-right: 3px; vertical-align: middle;
}
.gec-swatch-accept  { background: var(--accept); }
.gec-swatch-degree  { background: var(--reject-degree); }
.gec-swatch-cycle   { background: var(--reject-cycle); }
.gec-swatch-closing { background: var(--closing); }
.gec-swatch-tour    { background: var(--tour); }
.gec-swatch-accepted{ background: var(--accept); }

/* Event chip */
.gec-chip {
  font-size: 0.85rem; font-weight: 600; padding: 8px 12px;
  border-radius: 8px; border: 1px solid transparent;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.gec-chip-idle   { background: var(--panel); color: var(--muted); border-color: var(--line); font-weight: 400; }
.gec-chip-accept { background: #dcfce7; color: #15803d; border-color: #bbf7d0; }
.gec-chip-degree { background: #fce7f3; color: #be185d; border-color: #fbcfe8; }
.gec-chip-cycle  { background: #ffedd5; color: #c2410c; border-color: #fed7aa; }
.gec-chip-closing{ background: #fef3c7; color: #92400e; border-color: #fde68a; }

/* Stats */
.gec-statgrid { display: grid; grid-template-columns: repeat(6, 1fr); gap: 8px; }
@media (max-width: 540px) { .gec-statgrid { grid-template-columns: repeat(3, 1fr); } }
.gec-statlabel {
  font-size: 0.65rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}

/* Config */
.gec-config { display: flex; flex-direction: column; gap: 8px; }
.gec-config-row { display: flex; flex-direction: column; gap: 3px; }
.gec-config-label { font-size: 0.88rem; }
.gec-slider { width: 100%; accent-color: var(--accent); cursor: pointer; }
.gec-hint { font-size: 0.75rem; color: var(--muted); }

/* Controls */
.gec-controls { display: flex; gap: 8px; }
.gec-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer; font-family: inherit;
}
.gec-btn:hover:not(:disabled) { border-color: var(--accent); }
.gec-btn:disabled { opacity: 0.45; cursor: default; }
.gec-btn-primary { color: var(--accent); border-color: var(--accent); }

/* Footer */
.gec-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 8px; border-top: 1px solid var(--line); color: var(--muted);
}
`

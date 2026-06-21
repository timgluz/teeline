import { useState, useRef, useEffect, useCallback } from "preact/hooks"
import type { Agent, ForceBreakdown, SimState } from "./gsa-algo"
import { CITIES, N_CITIES, makeInitState, stepAgent } from "./gsa-algo"

const G0 = 20.0

function polyPts(tour: number[]): string {
  const pts = tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ")
  return pts + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}`
}

// ---------------------------------------------------------------
// AgentHeatmap — colored cells, hover reveals index + mass + dist
// ---------------------------------------------------------------
interface AgentHeatmapProps {
  agents: Agent[]
  gbest_cost: number
  activeIdx: number
  kbest: number[]
}
function AgentHeatmap({ agents, gbest_cost, activeIdx, kbest }: AgentHeatmapProps) {
  if (!agents.length) return null
  const costs = agents.map(a => a.cost)
  const minC = Math.min(...costs)
  const maxC = Math.max(...costs)
  const range = maxC - minC || 1
  const kbestSet = new Set(kbest)
  return (
    <div className="gsa-heatmap">
      <div className="gsa-heatmap-title">Agents</div>
      {agents.map((ag, i) => {
        const norm = (maxC - ag.cost) / range
        const hue = Math.round(norm * 120)
        const bg = `hsl(${hue},55%,42%)`
        const isActive = i === activeIdx
        const isKbest = kbestSet.has(i)
        const isGbest = Math.abs(ag.cost - gbest_cost) < 0.01
        const outline = isActive
          ? "2px solid #d97706"
          : isGbest
            ? "2px solid #16a34a"
            : "2px solid transparent"
        const borderLeft = isKbest && !isActive ? "3px solid #16a34a" : undefined
        return (
          <div
            key={i}
            className="gsa-heatmap-cell"
            style={{ background: bg, outline, outlineOffset: "1px", borderLeft }}
          >
            <span className="gsa-heatmap-idx">{i}</span>
            <span className="gsa-heatmap-cost">{ag.cost.toFixed(0)}</span>
            <div className="gsa-heatmap-overlay">
              #{i} · dist={ag.cost.toFixed(0)} · mass={ag.mass.toFixed(3)}
            </div>
          </div>
        )
      })}
    </div>
  )
}

// ---------------------------------------------------------------
// TourSVG — gbest (green), active agent (amber), ghosts (gray)
// ---------------------------------------------------------------
interface TourSVGProps {
  agents: Agent[]
  gbest: number[]
  activeIdx: number
  newGbest: boolean
}
function TourSVG({ agents, gbest, activeIdx, newGbest }: TourSVGProps) {
  if (!agents.length) return null
  return (
    <svg viewBox="0 0 300 300" className="gsa-canvas" role="img" aria-label="GSA tour canvas">
      <rect x={0} y={0} width={300} height={300} className="gsa-bg" />
      {agents.map((ag, i) => i !== activeIdx && (
        <polyline key={i} className="gsa-ghost" points={polyPts(ag.position)} />
      ))}
      <polyline className="gsa-active" points={polyPts(agents[activeIdx].position)} />
      <polyline
        className={`gsa-gbest${newGbest ? " gsa-gbest-pulse" : ""}`}
        points={polyPts(gbest)}
      />
      {CITIES.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y} r={4} className="gsa-city" />
      ))}
      {CITIES.map(([x, y], i) => (
        <text key={i} x={x + 6} y={y - 5} className="gsa-city-label">{i}</text>
      ))}
    </svg>
  )
}

// ---------------------------------------------------------------
// ForceChip — per-step annotation: agent, G, pulls, swaps
// ---------------------------------------------------------------
interface ForceChipProps {
  breakdown: ForceBreakdown | null
  agentIdx: number
  newGbest: boolean
}
function ForceChip({ breakdown, agentIdx, newGbest }: ForceChipProps) {
  if (!breakdown) {
    return <div className="gsa-chip gsa-chip-idle">Press Step or Run to begin</div>
  }
  const { pulls, totalApplied, G } = breakdown
  const prefix = newGbest ? "⭐ new gbest!  " : ""
  return (
    <div className={`gsa-chip ${newGbest ? "gsa-chip-gbest" : "gsa-chip-normal"}`}>
      {prefix}Agent {agentIdx} · G={G.toFixed(1)} · {pulls.length} pulls → {totalApplied} swaps applied
    </div>
  )
}

// ---------------------------------------------------------------
// GGauge — horizontal bar showing G decay from G0 → 0
// ---------------------------------------------------------------
function GGauge({ G }: { G: number }) {
  const pct = (G / G0) * 100
  const hint =
    G > 15 ? "High — broad exploration" :
    G > 8  ? "Mid — balanced" :
    "Low — exploitation focus"
  return (
    <div className="gsa-gauge">
      <div className="gsa-gauge-label">
        G (gravitational constant) = <strong>{G.toFixed(2)}</strong>
      </div>
      <div className="gsa-gauge-track">
        <div className="gsa-gauge-fill" style={{ width: `${Math.max(0, pct).toFixed(1)}%` }} />
      </div>
      <div className="gsa-gauge-hint">{hint}</div>
    </div>
  )
}

// ---------------------------------------------------------------
// GSAExplainer — main component
// ---------------------------------------------------------------
export default function GSAExplainer() {
  const [nAgents, setNAgents] = useState(8)
  const [speed, setSpeed] = useState(5)

  const simRef = useRef<SimState>(makeInitState(8))

  const [agents, setAgents] = useState<Agent[]>(() => simRef.current.agents)
  const [gbest, setGbest] = useState<number[]>(() => simRef.current.gbest)
  const [gbest_cost, setGbestCost] = useState(() => simRef.current.gbest_cost)
  const [epoch, setEpoch] = useState(0)
  const [agentIdx, setAgentIdx] = useState(0)
  const [G, setG] = useState(G0)
  const [kbest, setKbest] = useState<number[]>(() => simRef.current.kbest)
  const [breakdown, setBreakdown] = useState<ForceBreakdown | null>(null)
  const [newGbest, setNewGbest] = useState(false)
  const [running, setRunning] = useState(false)

  const reinit = useCallback((n: number) => {
    const s = makeInitState(n)
    simRef.current = s
    setAgents(s.agents.slice())
    setGbest(s.gbest.slice())
    setGbestCost(s.gbest_cost)
    setEpoch(0)
    setAgentIdx(0)
    setG(G0)
    setKbest(s.kbest.slice())
    setBreakdown(null)
    setNewGbest(false)
    setRunning(false)
  }, [])

  const stepOnce = useCallback(() => {
    const next = stepAgent(simRef.current)
    simRef.current = next
    setAgents(next.agents.slice())
    setGbest(next.gbest.slice())
    setGbestCost(next.gbest_cost)
    setEpoch(next.epoch)
    setAgentIdx(next.agentIdx)
    setG(next.G)
    setKbest(next.kbest.slice())
    setBreakdown(next.lastBreakdown)
    setNewGbest(next.newGbest)
  }, [])

  const delay = Math.max(50, 660 - speed * 66)
  useEffect(() => {
    if (!running) return
    const id = setInterval(stepOnce, delay)
    return () => clearInterval(id)
  }, [running, stepOnce, delay])

  const avgDist = agents.length
    ? (agents.reduce((s, a) => s + a.cost, 0) / agents.length).toFixed(0)
    : "—"

  const displayAgentIdx = breakdown ? simRef.current.agentIdx === 0
    ? nAgents - 1
    : (simRef.current.agentIdx - 1 + nAgents) % nAgents
    : 0

  return (
    <div className="gsa-root">
      <style>{CSS}</style>

      <header className="gsa-header">
        <div className="gsa-eyebrow">teeline · algorithms/gsa</div>
        <h2 className="gsa-title">Gravitational Search Algorithm</h2>
        <p className="gsa-sub">
          Each candidate TSP tour is an <strong>agent</strong> with a{" "}
          <strong>mass</strong> proportional to its fitness — better tours weigh
          more. Every step, the active agent is pulled toward the{" "}
          <strong>k-best</strong> (heaviest) neighbours via swap-move{" "}
          <strong>velocity</strong>. The gravitational constant{" "}
          <code>G</code> decays over epochs, shifting the swarm from broad
          exploration toward fine-grained exploitation.
        </p>
      </header>

      <div className="gsa-viz-row">
        <div className="gsa-canvas-wrap">
          <TourSVG agents={agents} gbest={gbest} activeIdx={agentIdx} newGbest={newGbest} />
        </div>
        <AgentHeatmap agents={agents} gbest_cost={gbest_cost} activeIdx={agentIdx} kbest={kbest} />
      </div>

      <div className="gsa-legend">
        <span><span className="gsa-dot gsa-dot-gbest">●</span> gbest tour</span>
        <span><span className="gsa-dot gsa-dot-active">●</span> active agent</span>
        <span><span className="gsa-dot gsa-dot-ghost">●</span> swarm</span>
        <span><span className="gsa-dot gsa-dot-city">●</span> city</span>
        <span><span className="gsa-kbest-marker" /> kbest source</span>
      </div>

      <ForceChip breakdown={breakdown} agentIdx={displayAgentIdx} newGbest={newGbest} />

      <GGauge G={G} />

      <div className="gsa-statgrid">
        <div>
          <div className="gsa-statlabel">epoch</div>
          <div className="gsa-mono">{epoch}</div>
        </div>
        <div>
          <div className="gsa-statlabel">step</div>
          <div className="gsa-mono">{agentIdx}/{nAgents}</div>
        </div>
        <div>
          <div className="gsa-statlabel">gbest dist</div>
          <div className="gsa-mono">{gbest_cost.toFixed(1)}</div>
        </div>
        <div>
          <div className="gsa-statlabel">avg dist</div>
          <div className="gsa-mono">{avgDist}</div>
        </div>
      </div>

      <div className="gsa-config">
        <div className="gsa-config-row">
          <label className="gsa-config-label">
            Agents = <strong>{nAgents}</strong>
          </label>
          <input
            type="range" min={4} max={12} step={1} value={nAgents}
            className="gsa-slider"
            onInput={(e) => {
              const n = Number((e.target as HTMLInputElement).value)
              setNAgents(n)
              reinit(n)
            }}
          />
          <div className="gsa-hint">
            {nAgents <= 5
              ? "Small swarm — fast but low diversity"
              : nAgents >= 10
                ? "Large swarm — high diversity, slower per epoch"
                : "Balanced swarm size"}
          </div>
        </div>
        <div className="gsa-config-row">
          <label className="gsa-config-label">Speed</label>
          <input
            type="range" min={1} max={10} step={1} value={speed}
            className="gsa-slider"
            onInput={(e) => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
        </div>
      </div>

      <div className="gsa-controls">
        <button className="gsa-btn" onClick={stepOnce} disabled={running}>◀ Step</button>
        <button
          className={`gsa-btn ${!running ? "gsa-btn-primary" : ""}`}
          onClick={() => setRunning(r => !r)}
        >
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="gsa-btn" onClick={() => reinit(nAgents)}>↺ Reset</button>
      </div>

      <footer className="gsa-footer">
        <span className="gsa-mono">cities: {N_CITIES}</span>
        <span className="gsa-mono">agents: {nAgents}</span>
        <span className="gsa-mono">G₀: {G0}</span>
        <span className="gsa-mono">α: 1</span>
        <span className="gsa-mono">kbest: ⌈{nAgents}/2⌉={Math.ceil(nAgents / 2)}</span>
      </footer>
    </div>
  )
}

// ---------------------------------------------------------------
// Styles — self-contained, scoped via gsa- prefix
// ---------------------------------------------------------------
const CSS = `
.gsa-root {
  --bg: #ffffff;
  --panel: #f6f8fa;
  --accent: #d97706;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --gbest: #16a34a;
  --active: #d97706;
  --ghost: #9ca3af;
  --city: #f2a154;
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
  gap: 12px;
}
.gsa-root * { box-sizing: border-box; }
.gsa-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

/* Header */
.gsa-header { margin-bottom: 2px; }
.gsa-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.gsa-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.gsa-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.gsa-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(217,119,6,0.1);
  padding: 1px 4px; border-radius: 4px;
}

/* Viz row */
.gsa-viz-row { display: flex; gap: 10px; align-items: stretch; }
.gsa-canvas-wrap { flex: 1; min-width: 0; }

/* Canvas SVG */
.gsa-canvas { width: 100%; display: block; border-radius: 8px; border: 1px solid var(--line); }
.gsa-bg { fill: var(--panel); }
.gsa-ghost { fill: none; stroke: var(--ghost); stroke-width: 1; stroke-opacity: 0.2; stroke-linejoin: round; }
.gsa-active { fill: none; stroke: var(--active); stroke-width: 1.8; stroke-linejoin: round; }
.gsa-gbest { fill: none; stroke: var(--gbest); stroke-width: 2.5; stroke-linejoin: round; }
.gsa-gbest-pulse { animation: gsa-pulse 0.5s ease; }
@keyframes gsa-pulse {
  0% { stroke-width: 5; opacity: 0.4; }
  100% { stroke-width: 2.5; opacity: 1; }
}
.gsa-city { fill: var(--city); stroke: var(--bg); stroke-width: 1.5; }
.gsa-city-label {
  font-size: 8px; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  fill: #374151; stroke: white; stroke-width: 2.5; paint-order: stroke fill;
  dominant-baseline: auto; pointer-events: none; user-select: none;
}

/* Agent heatmap */
.gsa-heatmap { width: 108px; flex-shrink: 0; display: flex; flex-direction: column; gap: 3px; }
.gsa-heatmap-title {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--muted); margin-bottom: 2px;
}
.gsa-heatmap-cell {
  flex: 1; border-radius: 5px; min-height: 20px;
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 7px; position: relative; overflow: hidden;
  transition: background 0.35s ease;
}
.gsa-heatmap-idx {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.72rem; font-weight: 700; color: rgba(255,255,255,0.9);
}
.gsa-heatmap-cost {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.68rem; color: rgba(255,255,255,0.7);
}
.gsa-heatmap-overlay {
  position: absolute; inset: 0; display: flex; align-items: center; justify-content: center;
  background: rgba(0,0,0,0.52); border-radius: 5px;
  font-size: 0.66rem; font-weight: 600; color: #fff;
  white-space: nowrap; letter-spacing: 0.02em;
  opacity: 0; transition: opacity 0.12s ease; pointer-events: none;
}
.gsa-heatmap-cell:hover .gsa-heatmap-overlay { opacity: 1; }

/* Legend */
.gsa-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); align-items: center; }
.gsa-dot { font-size: 1em; }
.gsa-dot-gbest  { color: var(--gbest); }
.gsa-dot-active { color: var(--active); }
.gsa-dot-ghost  { color: var(--ghost); }
.gsa-dot-city   { color: var(--city); }
.gsa-kbest-marker {
  display: inline-block; width: 3px; height: 14px;
  background: var(--gbest); border-radius: 2px; vertical-align: middle;
  margin-right: 4px;
}

/* Force chip */
.gsa-chip {
  font-size: 0.88rem; font-weight: 600; padding: 8px 14px;
  border-radius: 8px; border: 1px solid transparent;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.gsa-chip-idle   { background: var(--panel); color: var(--muted); border-color: var(--line); font-weight: 400; }
.gsa-chip-normal { background: rgba(217,119,6,0.08); color: #92400e; border-color: rgba(217,119,6,0.3); }
.gsa-chip-gbest  { background: rgba(22,163,74,0.12); color: #14532d; border-color: rgba(22,163,74,0.3); }

/* G gauge */
.gsa-gauge { display: flex; flex-direction: column; gap: 4px; }
.gsa-gauge-label { font-size: 0.88rem; }
.gsa-gauge-label strong {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.gsa-gauge-track {
  height: 10px; background: var(--panel);
  border: 1px solid var(--line); border-radius: 999px; overflow: hidden;
}
.gsa-gauge-fill {
  height: 100%; background: var(--accent); border-radius: 999px;
  transition: width 0.3s ease;
}
.gsa-gauge-hint { font-size: 0.78rem; color: var(--muted); }

/* Stats */
.gsa-statgrid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
@media (max-width: 480px) { .gsa-statgrid { grid-template-columns: repeat(2, 1fr); } }
.gsa-statlabel {
  font-size: 0.7rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}

/* Config */
.gsa-config { display: flex; flex-direction: column; gap: 10px; }
.gsa-config-row { display: flex; flex-direction: column; gap: 4px; }
.gsa-config-label { font-size: 0.88rem; }
.gsa-slider { width: 100%; accent-color: var(--accent); cursor: pointer; }
.gsa-hint { font-size: 0.78rem; color: var(--muted); }

/* Controls */
.gsa-controls { display: flex; gap: 8px; }
.gsa-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer; font-family: inherit;
}
.gsa-btn:hover:not(:disabled) { border-color: var(--accent); }
.gsa-btn:disabled { opacity: 0.45; cursor: default; }
.gsa-btn-primary { color: var(--accent); border-color: var(--accent); }

/* Footer */
.gsa-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 10px; border-top: 1px solid var(--line); color: var(--muted);
}
`

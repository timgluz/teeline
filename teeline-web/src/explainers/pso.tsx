import { useState, useRef, useEffect, useCallback } from "preact/hooks"
import type { Particle, VelocityBreakdown, SimState } from "./pso-algo"
import {
  CITIES, N_CITIES,
  makeInitState, stepEpoch,
} from "./pso-algo"

// PSO display constants — for UI labels and gauge only
const W_MAX = 0.9
const W_MIN = 0.4
const C1 = 1.5
const V_MAX = Math.max(1, Math.ceil(N_CITIES * 0.35))

// ---------------------------------------------------------------
// SVG helpers
// ---------------------------------------------------------------
function polyPts(tour: number[]): string {
  const pts = tour.map(i => `${CITIES[i][0]},${CITIES[i][1]}`).join(" ")
  return pts + ` ${CITIES[tour[0]][0]},${CITIES[tour[0]][1]}`
}

// ---------------------------------------------------------------
// SwarmHeatmap — color cells by current tour cost, outline gbest holder
// ---------------------------------------------------------------
interface SwarmHeatmapProps {
  particles: Particle[]
  gbest_cost: number
}
function SwarmHeatmap({ particles, gbest_cost }: SwarmHeatmapProps) {
  if (!particles.length) return null
  const costs = particles.map(p => p.cost)
  const minC = Math.min(...costs)
  const maxC = Math.max(...costs)
  const range = maxC - minC || 1
  return (
    <div className="pso-heatmap">
      <div className="pso-heatmap-title">Particles</div>
      {particles.map((p, i) => {
        const norm = (maxC - p.cost) / range
        const hue = Math.round(norm * 120)
        const bg = `hsl(${hue},55%,42%)`
        const isGbest = Math.abs(p.cost - gbest_cost) < 0.01
        const outline = isGbest ? "2px solid #16a34a" : "2px solid transparent"
        return (
          <div
            key={i}
            className="pso-heatmap-cell"
            style={{ background: bg, outline, outlineOffset: "1px" }}
          >
            <span className="pso-heatmap-idx">{i}</span>
            <span className="pso-heatmap-cost">{p.cost.toFixed(0)}</span>
            <div className="pso-heatmap-overlay">{isGbest ? "gbest" : `dist ${p.cost.toFixed(0)}`}</div>
          </div>
        )
      })}
    </div>
  )
}

// ---------------------------------------------------------------
// TourSVG — gbest (green), best-position particle (purple), ghosts (grey)
// ---------------------------------------------------------------
interface TourSVGProps {
  particles: Particle[]
  gbest: number[]
  newGbest: boolean
}
function TourSVG({ particles, gbest, newGbest }: TourSVGProps) {
  if (!particles.length) return null
  const bestIdx = particles.reduce((b, p, i) => p.cost < particles[b].cost ? i : b, 0)
  return (
    <svg viewBox="0 0 300 300" className="pso-canvas" role="img" aria-label="PSO tour canvas">
      <rect x={0} y={0} width={300} height={300} className="pso-bg" />
      {particles.map((p, i) => i !== bestIdx && (
        <polyline key={i} className="pso-ghost" points={polyPts(p.position)} />
      ))}
      <polyline className="pso-active" points={polyPts(particles[bestIdx].position)} />
      <polyline
        className={`pso-gbest${newGbest ? " pso-gbest-pulse" : ""}`}
        points={polyPts(gbest)}
      />
      {CITIES.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y} r={4} className="pso-city" />
      ))}
      {CITIES.map(([x, y], i) => (
        <text key={i} x={x + 6} y={y - 5} className="pso-city-label">{i}</text>
      ))}
    </svg>
  )
}

// ---------------------------------------------------------------
// VelocityChip — epoch-level aggregate breakdown of ω / C₁ / C₂ swaps
// ---------------------------------------------------------------
function VelocityChip({ breakdown, newGbest }: { breakdown: VelocityBreakdown | null; newGbest: boolean }) {
  if (!breakdown) {
    return <div className="pso-chip pso-chip-idle">Press Step or Run to begin</div>
  }
  return (
    <div className={`pso-chip ${newGbest ? "pso-chip-gbest" : "pso-chip-normal"}`}>
      {newGbest ? "⭐ new gbest!  " : ""}
      ω: {breakdown.inertia} · C₁: {breakdown.cognitive} · C₂: {breakdown.social} → {breakdown.applied} swaps applied
    </div>
  )
}

// ---------------------------------------------------------------
// InertiaGauge — horizontal bar showing current ω decaying from W_MAX to W_MIN
// ---------------------------------------------------------------
function InertiaGauge({ w }: { w: number }) {
  const pct = ((w - W_MIN) / (W_MAX - W_MIN)) * 100
  const hint =
    w > 0.7 ? "High — broad exploration" :
    w < 0.5 ? "Low — local exploitation" :
    "Mid — balanced"
  return (
    <div className="pso-gauge">
      <div className="pso-gauge-label">
        ω (inertia) = <strong>{w.toFixed(3)}</strong>
      </div>
      <div className="pso-gauge-track">
        <div className="pso-gauge-fill" style={{ width: `${Math.max(0, pct).toFixed(1)}%` }} />
      </div>
      <div className="pso-gauge-hint">{hint}</div>
    </div>
  )
}

// ---------------------------------------------------------------
// PSOExplainer — main component
// ---------------------------------------------------------------
export default function PSOExplainer() {
  const [nParticles, setNParticles] = useState(6)
  const [speed, setSpeed] = useState(5)

  const simRef = useRef<SimState>(makeInitState(6))

  const [particles, setParticles] = useState<Particle[]>(() => simRef.current.particles)
  const [gbest, setGbest] = useState<number[]>(() => simRef.current.gbest)
  const [gbest_cost, setGbestCost] = useState(() => simRef.current.gbest_cost)
  const [epoch, setEpoch] = useState(0)
  const [w, setW] = useState(W_MAX)
  const [breakdown, setBreakdown] = useState<VelocityBreakdown | null>(null)
  const [newGbest, setNewGbest] = useState(false)
  const [running, setRunning] = useState(false)

  const reinit = useCallback((n: number) => {
    const s = makeInitState(n)
    simRef.current = s
    setParticles(s.particles.slice())
    setGbest(s.gbest.slice())
    setGbestCost(s.gbest_cost)
    setEpoch(0)
    setW(W_MAX)
    setBreakdown(null)
    setNewGbest(false)
    setRunning(false)
  }, [])

  const stepOnce = useCallback(() => {
    const next = stepEpoch(simRef.current)
    simRef.current = next
    setParticles(next.particles.slice())
    setGbest(next.gbest.slice())
    setGbestCost(next.gbest_cost)
    setEpoch(next.epoch)
    setW(next.w)
    setBreakdown(next.lastBreakdown)
    setNewGbest(next.newGbest)
  }, [])

  const delay = Math.max(50, 660 - speed * 66)
  useEffect(() => {
    if (!running) return
    const id = setInterval(stepOnce, delay)
    return () => clearInterval(id)
  }, [running, stepOnce, delay])

  const avgDist = particles.length
    ? (particles.reduce((s, p) => s + p.cost, 0) / particles.length).toFixed(0)
    : "—"

  return (
    <div className="pso-root">
      <style>{CSS}</style>

      <header className="pso-header">
        <div className="pso-eyebrow">teeline · algorithms/pso</div>
        <h2 className="pso-title">Particle Swarm Optimisation</h2>
        <p className="pso-sub">
          A swarm of tour-particles updates each epoch. Each particle's{" "}
          <strong>velocity</strong> is a list of city-swap moves built from three
          components: <code>ω</code> keeps momentum from the previous epoch,{" "}
          <code>C₁</code> pulls toward the particle's own personal best, and{" "}
          <code>C₂</code> pulls toward the global best. Inertia <code>ω</code>{" "}
          decays over time, shifting the swarm from exploration toward exploitation.
        </p>
      </header>

      <div className="pso-viz-row">
        <div className="pso-canvas-wrap">
          <TourSVG particles={particles} gbest={gbest} newGbest={newGbest} />
        </div>
        <SwarmHeatmap particles={particles} gbest_cost={gbest_cost} />
      </div>

      <div className="pso-legend">
        <span><span className="pso-dot pso-dot-gbest">●</span> gbest tour</span>
        <span><span className="pso-dot pso-dot-active">●</span> best particle</span>
        <span><span className="pso-dot pso-dot-ghost">●</span> swarm</span>
        <span><span className="pso-dot pso-dot-city">●</span> city</span>
      </div>

      <VelocityChip breakdown={breakdown} newGbest={newGbest} />

      <InertiaGauge w={w} />

      <div className="pso-statgrid">
        <div>
          <div className="pso-statlabel">epoch</div>
          <div className="pso-mono">{epoch}</div>
        </div>
        <div>
          <div className="pso-statlabel">gbest dist</div>
          <div className="pso-mono">{gbest_cost.toFixed(1)}</div>
        </div>
        <div>
          <div className="pso-statlabel">avg dist</div>
          <div className="pso-mono">{avgDist}</div>
        </div>
        <div>
          <div className="pso-statlabel">v_max</div>
          <div className="pso-mono">{V_MAX} swaps</div>
        </div>
      </div>

      <div className="pso-config">
        <div className="pso-config-row">
          <label className="pso-config-label">
            Particles = <strong>{nParticles}</strong>
          </label>
          <input
            type="range" min={3} max={12} step={1} value={nParticles}
            className="pso-slider"
            onInput={(e) => {
              const n = Number((e.target as HTMLInputElement).value)
              setNParticles(n)
              reinit(n)
            }}
          />
          <div className="pso-hint">
            {nParticles <= 4
              ? "Small swarm — fast but low diversity"
              : nParticles >= 10
                ? "Large swarm — high diversity, slower per epoch"
                : "Balanced swarm size"}
          </div>
        </div>

        <div className="pso-config-row">
          <label className="pso-config-label">Speed</label>
          <input
            type="range" min={1} max={10} step={1} value={speed}
            className="pso-slider"
            onInput={(e) => setSpeed(Number((e.target as HTMLInputElement).value))}
          />
        </div>
      </div>

      <div className="pso-controls">
        <button className="pso-btn" onClick={stepOnce} disabled={running}>◀ Step</button>
        <button
          className={`pso-btn ${!running ? "pso-btn-primary" : ""}`}
          onClick={() => setRunning(r => !r)}
        >
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="pso-btn" onClick={() => reinit(nParticles)}>↺ Reset</button>
      </div>

      <footer className="pso-footer">
        <span className="pso-mono">cities: {N_CITIES}</span>
        <span className="pso-mono">particles: {nParticles}</span>
        <span className="pso-mono">ω: {W_MIN}→{W_MAX}</span>
        <span className="pso-mono">C₁=C₂={C1}</span>
        <span className="pso-mono">v_max: {V_MAX}</span>
      </footer>
    </div>
  )
}

// ---------------------------------------------------------------
// Styles — self-contained, scoped via pso- prefix
// ---------------------------------------------------------------
const CSS = `
.pso-root {
  --bg: #ffffff;
  --panel: #f6f8fa;
  --accent: #7c3aed;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --gbest: #16a34a;
  --active: #7c3aed;
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
.pso-root * { box-sizing: border-box; }
.pso-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

/* Header */
.pso-header { margin-bottom: 2px; }
.pso-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.pso-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.pso-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.pso-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(124,58,237,0.08);
  padding: 1px 4px; border-radius: 4px;
}

/* Viz row */
.pso-viz-row { display: flex; gap: 10px; align-items: stretch; }
.pso-canvas-wrap { flex: 1; min-width: 0; }

/* Canvas SVG */
.pso-canvas { width: 100%; display: block; border-radius: 8px; border: 1px solid var(--line); }
.pso-bg { fill: var(--panel); }
.pso-ghost { fill: none; stroke: var(--ghost); stroke-width: 1; stroke-opacity: 0.22; stroke-linejoin: round; }
.pso-active { fill: none; stroke: var(--active); stroke-width: 1.8; stroke-linejoin: round; }
.pso-gbest { fill: none; stroke: var(--gbest); stroke-width: 2.5; stroke-linejoin: round; }
.pso-gbest-pulse { animation: pso-pulse 0.5s ease; }
@keyframes pso-pulse {
  0% { stroke-width: 5; opacity: 0.4; }
  100% { stroke-width: 2.5; opacity: 1; }
}
.pso-city { fill: var(--city); stroke: var(--bg); stroke-width: 1.5; }
.pso-city-label {
  font-size: 8px; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  fill: #374151; stroke: white; stroke-width: 2.5; paint-order: stroke fill;
  dominant-baseline: auto; pointer-events: none; user-select: none;
}

/* Swarm heatmap */
.pso-heatmap { width: 100px; flex-shrink: 0; display: flex; flex-direction: column; gap: 3px; }
.pso-heatmap-title {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em;
  color: var(--muted); margin-bottom: 2px;
}
.pso-heatmap-cell {
  flex: 1; border-radius: 5px; min-height: 20px;
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 7px; position: relative; overflow: hidden;
  transition: background 0.35s ease;
}
.pso-heatmap-idx {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.72rem; font-weight: 700; color: rgba(255,255,255,0.9);
}
.pso-heatmap-cost {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.68rem; color: rgba(255,255,255,0.7);
}
.pso-heatmap-overlay {
  position: absolute; inset: 0; display: flex; align-items: center; justify-content: center;
  background: rgba(0,0,0,0.48); border-radius: 5px;
  font-size: 0.68rem; font-weight: 700; color: #fff;
  white-space: nowrap; letter-spacing: 0.02em;
  opacity: 0; transition: opacity 0.12s ease; pointer-events: none;
}
.pso-heatmap-cell:hover .pso-heatmap-overlay { opacity: 1; }

/* Legend */
.pso-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); }
.pso-dot { font-size: 1em; }
.pso-dot-gbest  { color: var(--gbest); }
.pso-dot-active { color: var(--active); }
.pso-dot-ghost  { color: var(--ghost); }
.pso-dot-city   { color: var(--city); }

/* Velocity chip */
.pso-chip {
  font-size: 0.88rem; font-weight: 600; padding: 8px 14px;
  border-radius: 8px; border: 1px solid transparent;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.pso-chip-idle   { background: var(--panel); color: var(--muted); border-color: var(--line); font-weight: 400; }
.pso-chip-normal { background: rgba(124,58,237,0.08); color: #5b21b6; border-color: rgba(124,58,237,0.25); }
.pso-chip-gbest  { background: rgba(22,163,74,0.12); color: #14532d; border-color: rgba(22,163,74,0.3); }

/* Inertia gauge */
.pso-gauge { display: flex; flex-direction: column; gap: 4px; }
.pso-gauge-label { font-size: 0.88rem; }
.pso-gauge-label strong {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.pso-gauge-track {
  height: 10px; background: var(--panel);
  border: 1px solid var(--line); border-radius: 999px; overflow: hidden;
}
.pso-gauge-fill {
  height: 100%; background: var(--accent); border-radius: 999px;
  transition: width 0.3s ease;
}
.pso-gauge-hint { font-size: 0.78rem; color: var(--muted); }

/* Stats */
.pso-statgrid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
@media (max-width: 480px) { .pso-statgrid { grid-template-columns: repeat(2, 1fr); } }
.pso-statlabel {
  font-size: 0.7rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}

/* Config sliders */
.pso-config { display: flex; flex-direction: column; gap: 10px; }
.pso-config-row { display: flex; flex-direction: column; gap: 4px; }
.pso-config-label { font-size: 0.88rem; }
.pso-slider { width: 100%; accent-color: var(--accent); cursor: pointer; }
.pso-hint { font-size: 0.78rem; color: var(--muted); }

/* Controls */
.pso-controls { display: flex; gap: 8px; }
.pso-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer; font-family: inherit;
}
.pso-btn:hover:not(:disabled) { border-color: var(--accent); }
.pso-btn:disabled { opacity: 0.45; cursor: default; }
.pso-btn-primary { color: var(--accent); border-color: var(--accent); }

/* Footer */
.pso-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 10px; border-top: 1px solid var(--line); color: var(--muted);
}
`

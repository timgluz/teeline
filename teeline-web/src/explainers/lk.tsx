// teeline-web/src/explainers/lk.tsx
import { useState, useEffect, useCallback, useMemo } from 'preact/hooks'
import type { LSFrame, ILSFrame } from './lk'
import {
  CITIES, DIST, INIT_TOUR,
  computeLocalSearchFrames, computeILSFrames,
  lcgRand,
} from './lk'

// ── TourSVG ──────────────────────────────────────────────────────────────────

interface TourSVGProps {
  tour: number[]
  scanEdges?: [[number, number], [number, number]] | null
  swapEdges?: [[number, number], [number, number]] | null
  bestTour?: number[] | null
  bridgePoints?: [number, number, number] | null
  highlight?: 'swap' | 'bridge' | 'best' | null
  overlay?: string | null
}

function TourSVG({
  tour, scanEdges = null, swapEdges = null,
  bestTour = null, bridgePoints = null, highlight = null, overlay = null,
}: TourSVGProps) {
  const cities = CITIES

  function edgePath(cityA: number, cityB: number) {
    const [x1, y1] = cities[cityA]
    const [x2, y2] = cities[cityB]
    return { x1, y1, x2, y2 }
  }

  const n = tour.length
  const tourEdges = Array.from({ length: n }, (_, i) => ({
    ...edgePath(tour[i], tour[(i + 1) % n]),
    key: `${tour[i]}-${tour[(i + 1) % n]}`,
  }))

  const bestEdges = bestTour
    ? Array.from({ length: bestTour.length }, (_, i) => ({
        ...edgePath(bestTour[i], bestTour[(i + 1) % bestTour.length]),
        key: `b${bestTour[i]}-${bestTour[(i + 1) % bestTour.length]}`,
      }))
    : []

  const tourColor = highlight === 'best' ? '#e8a000' : '#e0626b'

  return (
    <svg viewBox="0 0 300 300" className="lk-canvas" role="img" aria-label="TSP tour visualisation">
      <rect x="0" y="0" width="300" height="300" className="lk-bg" />

      {/* Best-tour underlay (thin dashed) */}
      {bestEdges.map(e => (
        <line key={e.key} x1={e.x1} y1={e.y1} x2={e.x2} y2={e.y2} className="lk-best-edge" />
      ))}

      {/* Current tour */}
      {tourEdges.map(e => (
        <line key={e.key} x1={e.x1} y1={e.y1} x2={e.x2} y2={e.y2}
          className="lk-tour-edge"
          style={{ stroke: tourColor, transition: 'stroke 0.25s ease' }} />
      ))}

      {/* Scan edges (gray dashed — shown in Step mode) */}
      {scanEdges?.map(([a, b], idx) => {
        const [x1, y1] = cities[a]
        const [x2, y2] = cities[b]
        return <line key={idx} x1={x1} y1={y1} x2={x2} y2={y2} className="lk-scan-edge" />
      })}

      {/* Swap edges (green — new edges accepted) */}
      {swapEdges?.map(([a, b], idx) => {
        const [x1, y1] = cities[a]
        const [x2, y2] = cities[b]
        return <line key={idx} x1={x1} y1={y1} x2={x2} y2={y2} className="lk-swap-edge" />
      })}

      {/* Bridge cut circles */}
      {bridgePoints?.map((cityId, idx) => {
        const [cx, cy] = cities[cityId]
        return <circle key={idx} cx={cx} cy={cy} r="10" className="lk-bridge-circle" />
      })}

      {/* City dots */}
      {cities.map(([cx, cy], i) => (
        <circle key={i} cx={cx} cy={cy} r="5" className="lk-city" />
      ))}

      {/* Overlay label */}
      {overlay && (
        <text x="150" y="155" className="lk-overlay">{overlay}</text>
      )}
    </svg>
  )
}

// ── Styles ────────────────────────────────────────────────────────────────────

const CSS = `
.lk-root {
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --accent: #0969da;
  --green: #1a7f37;
  --red: #cf222e;
  --gold: #e8a000;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 20px;
  max-width: 760px;
  box-sizing: border-box;
}
.lk-root * { box-sizing: border-box; }
.lk-mono { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.85em; }

.lk-header { margin-bottom: 14px; }
.lk-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.lk-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; }
.lk-sub { margin: 0; color: var(--muted); font-size: 0.92rem; line-height: 1.5; }

.lk-tabs { display: flex; gap: 6px; margin-bottom: 14px; border-bottom: 1px solid var(--line); }
.lk-tabbtn {
  background: transparent; border: none; color: var(--muted);
  font-size: 0.9rem; padding: 8px 12px; cursor: pointer;
  border-bottom: 2px solid transparent; margin-bottom: -1px;
  font-family: inherit;
}
.lk-tabbtn:hover { color: var(--text); }
.lk-tabbtn-active { color: var(--text); border-bottom-color: var(--accent); }

.lk-tab { display: flex; flex-direction: column; gap: 14px; }
@media (min-width: 620px) {
  .lk-tab { flex-direction: row; align-items: flex-start; }
  .lk-canvas { flex: 0 0 300px; }
  .lk-prose { flex: 1 1 auto; min-width: 0; }
}

.lk-canvas { width: 100%; max-width: 300px; border-radius: 8px; border: 1px solid var(--line); display: block; }
.lk-bg { fill: var(--panel); }
.lk-tour-edge { stroke-width: 1.8; }
.lk-best-edge { stroke: #aaa; stroke-width: 1; stroke-dasharray: 4 3; opacity: 0.5; }
.lk-scan-edge { stroke: #999; stroke-width: 1.5; stroke-dasharray: 5 3; opacity: 0.7; }
.lk-swap-edge { stroke: #1a7f37; stroke-width: 2.5; opacity: 0.9; }
.lk-bridge-circle { fill: none; stroke: #cf222e; stroke-width: 2; stroke-dasharray: 4 3; opacity: 0.85; }
.lk-city { fill: #f2a154; stroke: #fff; stroke-width: 1.5; }
.lk-overlay {
  text-anchor: middle; dominant-baseline: middle;
  font-family: ui-sans-serif, system-ui, sans-serif; font-size: 13px; font-weight: 600;
  fill: var(--text); paint-order: stroke; stroke: var(--panel); stroke-width: 4;
}

.lk-prose p { margin: 0 0 10px; font-size: 0.92rem; line-height: 1.55; }
.lk-note { font-size: 0.85rem; color: var(--muted); border-left: 2px solid var(--line); padding-left: 10px; }

.lk-controls { display: flex; gap: 8px; align-items: center; margin-bottom: 10px; flex-wrap: wrap; }
.lk-btn {
  background: var(--panel); color: var(--text); border: 1px solid var(--line);
  border-radius: 6px; padding: 6px 14px; font-size: 0.85rem; cursor: pointer;
  font-family: inherit;
}
.lk-btn:hover:not(:disabled) { border-color: var(--accent); }
.lk-btn:disabled { opacity: 0.45; cursor: default; }
.lk-btn-primary { color: var(--accent); border-color: var(--accent); }
.lk-speed-label { font-size: 0.8rem; color: var(--muted); white-space: nowrap; }
.lk-slider { accent-color: var(--accent); width: 80px; }

.lk-statgrid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; margin-bottom: 10px; }
.lk-statlabel { font-size: 0.72rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px; }

.lk-phase {
  display: inline-block; padding: 3px 10px; border-radius: 12px;
  font-size: 0.8rem; font-weight: 600; margin-bottom: 10px;
  background: var(--panel); border: 1px solid var(--line); color: var(--text);
}
.lk-phase-bridge { background: #fff0f0; border-color: var(--red); color: var(--red); }
.lk-phase-best   { background: #fff8e0; border-color: var(--gold); color: #7a5000; }
.lk-phase-pass   { background: #f0f8ff; border-color: var(--accent); color: var(--accent); }

.lk-footer { display: flex; flex-wrap: wrap; gap: 14px; margin-top: 16px; padding-top: 10px; border-top: 1px solid var(--line); color: var(--muted); }
`

// ── Controls ──────────────────────────────────────────────────────────────────

const SPEED_STEPS = [600, 300, 150, 80, 30] // ms per tick

interface ControlsProps {
  playing: boolean
  done: boolean
  speedIdx: number
  onPlayPause: () => void
  onStep: () => void
  onReset: () => void
  onSpeedChange: (idx: number) => void
}

function Controls({ playing, done, speedIdx, onPlayPause, onStep, onReset, onSpeedChange }: ControlsProps) {
  return (
    <div className="lk-controls">
      <button className="lk-btn lk-btn-primary" onClick={onPlayPause} disabled={done}>
        {playing ? '⏸ Pause' : done ? '⏹ Done' : '▶ Run'}
      </button>
      <button className="lk-btn" onClick={onStep} disabled={done}>⏭ Step</button>
      <button className="lk-btn" onClick={onReset}>↺ Reset</button>
      <span className="lk-speed-label">Speed:</span>
      <input
        type="range" min={0} max={4} step={1} value={speedIdx}
        className="lk-slider"
        aria-label="animation speed"
        onChange={e => onSpeedChange(Number((e.target as HTMLInputElement).value))}
      />
    </div>
  )
}

// ── StatsPanel ────────────────────────────────────────────────────────────────

interface StatEntry { label: string; value: string | number }

function StatsPanel({ stats }: { stats: StatEntry[] }) {
  return (
    <div className="lk-statgrid">
      {stats.map(({ label, value }) => (
        <div key={label}>
          <div className="lk-statlabel">{label}</div>
          <div className="lk-mono">
            {typeof value === 'number'
              ? Number.isInteger(value) ? value : value.toFixed(1)
              : value}
          </div>
        </div>
      ))}
    </div>
  )
}

// ── PhaseIndicator ────────────────────────────────────────────────────────────

function PhaseIndicator({ phase }: { phase: string }) {
  const cls = phase.includes('bridge') ? 'lk-phase-bridge'
    : phase.includes('best') || phase.includes('New') ? 'lk-phase-best'
    : phase.includes('LK') ? 'lk-phase-pass'
    : ''
  return <div className={`lk-phase ${cls}`}>{phase}</div>
}

import { useState, useRef, useEffect, useMemo } from 'preact/hooks'
import { CITIES_SFC, N_CITIES_SFC, buildLayout } from './sfc-algo'
import type { Cell, CurveType, Point, SfcLayout, TourEdge } from './sfc-algo'

const VIEW = 300
const PAD = 20
const SIZE = VIEW - PAD * 2
// 2x the 440px display cap, so the canvas stays crisp on HiDPI screens.
const CANVAS_PX = 880
const PLAY_INTERVAL_MS = 700

function cellPoint(cell: Cell, p: number): [number, number] {
  const n = 1 << p
  const [x, y] = cell
  return [PAD + ((x + 0.5) / n) * SIZE, VIEW - PAD - ((y + 0.5) / n) * SIZE]
}

function cityPoint(city: Point, layout: SfcLayout): [number, number] {
  const [x, y] = city
  return [
    PAD + ((x - layout.bounds.minX) / layout.bounds.span) * SIZE,
    VIEW - PAD - ((y - layout.bounds.minY) / layout.bounds.span) * SIZE,
  ]
}

function edgeKey(edge: TourEdge): string {
  return `${edge.from}-${edge.to}`
}

function cityLabel(city: number): string {
  return `city ${city + 1}`
}

function drawCurve(canvas: HTMLCanvasElement, layout: SfcLayout): void {
  canvas.width = CANVAS_PX
  canvas.height = CANVAS_PX
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const scale = CANVAS_PX / VIEW
  ctx.setTransform(scale, 0, 0, scale, 0, 0)
  ctx.clearRect(0, 0, VIEW, VIEW)
  ctx.fillStyle = '#f6f8fa'
  ctx.fillRect(0, 0, VIEW, VIEW)
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'

  const n = 1 << layout.p

  if (layout.p <= 5) {
    ctx.strokeStyle = '#d0d7de'
    ctx.lineWidth = 0.45
    ctx.globalAlpha = 0.55
    ctx.beginPath()
    for (let i = 0; i <= n; i++) {
      const offset = PAD + (i / n) * SIZE
      ctx.moveTo(PAD, offset)
      ctx.lineTo(VIEW - PAD, offset)
      ctx.moveTo(offset, PAD)
      ctx.lineTo(offset, VIEW - PAD)
    }
    ctx.stroke()
  }

  const cells = layout.cells
  const strokeCurve = (
    predicate: (manhattan: number) => boolean,
    color: string,
    alpha: number,
    width: number,
  ) => {
    ctx.beginPath()
    for (let i = 1; i < cells.length; i++) {
      const [x0, y0] = cells[i - 1]
      const [x1, y1] = cells[i]
      const manhattan = Math.abs(x1 - x0) + Math.abs(y1 - y0)
      if (!predicate(manhattan)) continue
      const [sx0, sy0] = cellPoint(cells[i - 1], layout.p)
      const [sx1, sy1] = cellPoint(cells[i], layout.p)
      ctx.moveTo(sx0, sy0)
      ctx.lineTo(sx1, sy1)
    }
    ctx.strokeStyle = color
    ctx.globalAlpha = alpha
    ctx.lineWidth = width
    ctx.stroke()
  }

  // Adjacent steps form the continuous Hilbert path.
  strokeCurve((manhattan) => manhattan === 1, '#0d9488', 0.3, 0.8)
  // Morton's non-adjacent steps are the locality-breaking jumps.
  strokeCurve((manhattan) => manhattan > 1, '#ea580c', 0.13, 0.7)
  strokeCurve(
    (manhattan) => manhattan >= Math.max(2, n / 4),
    '#ea580c',
    0.24,
    0.9,
  )
  strokeCurve(
    (manhattan) => manhattan >= Math.max(2, n / 2),
    '#dc2626',
    0.36,
    1,
  )

  ctx.globalAlpha = 1
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="sfc-statlabel">{label}</div>
      <div className="sfc-mono sfc-statvalue">{value}</div>
    </div>
  )
}

export default function SfcExplainer() {
  const [p, setP] = useState(1)
  const [curveType, setCurveType] = useState<CurveType>('hilbert')
  const [running, setRunning] = useState(false)
  const canvasRef = useRef<HTMLCanvasElement | null>(null)

  const layout = useMemo(
    () => buildLayout(CITIES_SFC, p, curveType),
    [p, curveType],
  )

  const orderPosition = useMemo(() => {
    const positions = new Map<number, number>()
    layout.order.forEach((city, index) => positions.set(city, index))
    return positions
  }, [layout])

  useEffect(() => {
    if (!running) return
    const id = window.setInterval(() => {
      setP((current) => Math.min(current + 1, 8))
    }, PLAY_INTERVAL_MS)
    return () => window.clearInterval(id)
  }, [running])

  useEffect(() => {
    if (running && p >= 8) setRunning(false)
  }, [p, running])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const frame = window.requestAnimationFrame(() => drawCurve(canvas, layout))
    return () => window.cancelAnimationFrame(frame)
  }, [layout])

  const stop = () => setRunning(false)

  const chooseCurve = (type: CurveType) => {
    stop()
    setCurveType(type)
  }

  const onOrderInput = (event: Event) => {
    stop()
    const value = Number((event.currentTarget as HTMLInputElement).value)
    setP(value)
  }

  const play = () => {
    if (running) return
    const reduceMotion =
      typeof window !== 'undefined' &&
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
    if (reduceMotion) {
      setP(8)
      return
    }
    if (p >= 8) setP(1)
    setRunning(true)
  }

  const reset = () => {
    stop()
    setP(1)
  }

  const gridSize = 1 << p
  const seamKey = edgeKey(layout.seam)
  const longestKey = edgeKey(layout.longest)
  const longestDiffers = seamKey !== longestKey
  const baseEdges = layout.edges.filter(
    (edge) => edgeKey(edge) !== seamKey && edgeKey(edge) !== longestKey,
  )
  const curveLabel = curveType === 'hilbert' ? 'Hilbert' : 'Morton'
  const jumpLabel =
    layout.stats.longJumps === 1 ? 'long curve jump' : 'long curve jumps'
  const summary = `${curveLabel} curve, grid order ${p} (${gridSize} by ${gridSize} grid). ${layout.stats.cities} cities in ${layout.stats.cellsUsed} cells. Tour ${Math.round(layout.stats.tourLength)} layout units, seam ${Math.round(layout.stats.seamLength)}, longest edge ${Math.round(layout.stats.longestEdgeLength)}. ${layout.stats.longJumps} ${jumpLabel}. The full city order is listed in the curve order panel.`

  const renderEdge = (edge: TourEdge, className: string) => {
    const [x1, y1] = cityPoint(CITIES_SFC[edge.from], layout)
    const [x2, y2] = cityPoint(CITIES_SFC[edge.to], layout)
    return (
      <g key={edgeKey(edge)}>
        <line x1={x1} y1={y1} x2={x2} y2={y2} className="sfc-edge-halo" />
        <line x1={x1} y1={y1} x2={x2} y2={y2} className={className} />
      </g>
    )
  }

  return (
    <div className="sfc-root">
      <style>{CSS}</style>

      <header className="sfc-header">
        <div className="sfc-eyebrow">
          teeline · algorithms/space-filling-curve
        </div>
        <h2 className="sfc-title">Space-Filling Curve Construction</h2>
        <p className="sfc-sub">
          Project cities onto a <code>2^p × 2^p</code> grid, order them by a
          Hilbert or Morton curve index, and connect that order into a tour. The
          closing seam is the one edge the curve cannot keep short.
        </p>
      </header>

      <div className="sfc-viz-row">
        <div className="sfc-canvas-wrap">
          <canvas
            ref={canvasRef}
            className="sfc-canvas"
            width={CANVAS_PX}
            height={CANVAS_PX}
            aria-hidden="true"
          />
          <svg
            className="sfc-overlay"
            viewBox={`0 0 ${VIEW} ${VIEW}`}
            role="img"
            aria-label={summary}
          >
            {baseEdges.map((edge) => renderEdge(edge, 'sfc-tour-edge'))}
            {longestDiffers &&
              renderEdge(layout.longest, 'sfc-tour-edge sfc-longest')}
            {renderEdge(layout.seam, 'sfc-tour-edge sfc-seam')}
            {CITIES_SFC.map((city, index) => {
              const [x, y] = cityPoint(city, layout)
              const position = orderPosition.get(index) ?? 0
              const isSeamEndpoint =
                position === 0 || position === N_CITIES_SFC - 1
              return (
                <g key={index}>
                  <circle
                    cx={x}
                    cy={y}
                    r={isSeamEndpoint ? 5.5 : 4.5}
                    className={
                      isSeamEndpoint ? 'sfc-city sfc-city-seam' : 'sfc-city'
                    }
                  />
                  <text
                    x={x}
                    y={y - 7}
                    className={
                      isSeamEndpoint
                        ? 'sfc-order-badge sfc-order-badge-seam'
                        : 'sfc-order-badge'
                    }
                  >
                    {position + 1}
                  </text>
                </g>
              )
            })}
          </svg>
        </div>

        <div className="sfc-order-panel">
          <div className="sfc-order-title">Curve order</div>
          <div className="sfc-order-subtitle">
            {curveLabel} · p = {p}
          </div>
          <ol className="sfc-order-list">
            {layout.order.map((city, index) => (
              <li
                key={city}
                className={
                  index === 0 || index === layout.order.length - 1
                    ? 'sfc-order-item sfc-order-item-seam'
                    : 'sfc-order-item'
                }
              >
                <span className="sfc-order-pos">{index + 1}</span>
                <span className="sfc-order-city">{cityLabel(city)}</span>
              </li>
            ))}
          </ol>
        </div>
      </div>

      <div className="sfc-legend">
        <span>
          <span className="sfc-swatch sfc-swatch-curve" /> curve
        </span>
        <span>
          <span className="sfc-swatch sfc-swatch-jump" /> long curve jump
        </span>
        <span>
          <span className="sfc-swatch sfc-swatch-tour" /> tour edge
        </span>
        <span>
          <span className="sfc-swatch sfc-swatch-seam" /> seam (closing edge)
        </span>
        <span>
          <span className="sfc-swatch sfc-swatch-longest" /> longest edge
        </span>
        <span>
          <span className="sfc-swatch sfc-swatch-city" /> city / visit order
        </span>
      </div>

      <div className="sfc-callout">
        <p>
          <strong>Seam:</strong> {cityLabel(layout.seam.from)} →{' '}
          {cityLabel(layout.seam.to)}, {Math.round(layout.seam.length)} layout
          units.
          {longestDiffers ? (
            <>
              {' '}
              <strong>Longest edge:</strong> {cityLabel(layout.longest.from)} →{' '}
              {cityLabel(layout.longest.to)},{' '}
              {Math.round(layout.longest.length)} layout units.
            </>
          ) : (
            <> It is also the longest edge.</>
          )}
        </p>
        <p>
          A 2-opt pass removes the seam and one other edge, then reconnects the
          four endpoints. That is why SFC tours are usually refined with{' '}
          <a href="/algorithms/two-opt/">2-opt</a> or{' '}
          <a href="/algorithms/lin-kernighan/">Lin-Kernighan</a>.
        </p>
      </div>

      <div className="sfc-statgrid">
        <Stat label="grid order" value={`p = ${p}`} />
        <Stat
          label="cities / cells"
          value={`${layout.stats.cities} / ${layout.stats.cellsUsed}`}
        />
        <Stat
          label="largest bucket"
          value={String(layout.stats.largestBucket)}
        />
        <Stat
          label="tour length"
          value={String(Math.round(layout.stats.tourLength))}
        />
        <Stat
          label="seam length"
          value={String(Math.round(layout.stats.seamLength))}
        />
        <Stat
          label="longest edge"
          value={String(Math.round(layout.stats.longestEdgeLength))}
        />
        <Stat label="long curve jumps" value={String(layout.stats.longJumps)} />
      </div>

      <div className="sfc-config">
        <div className="sfc-config-row">
          <label className="sfc-config-label" htmlFor="sfc-order">
            Grid order p
          </label>
          <div className="sfc-slider-row">
            <input
              id="sfc-order"
              className="sfc-slider"
              type="range"
              min={1}
              max={8}
              step={1}
              value={p}
              onInput={onOrderInput}
              aria-valuetext={`grid order ${p}, ${gridSize} by ${gridSize} grid`}
            />
            <span className="sfc-mono sfc-order-value">p = {p}</span>
          </div>
        </div>

        <div className="sfc-controls">
          <div className="sfc-toggle" role="group" aria-label="Curve type">
            <button
              type="button"
              className="sfc-toggle-btn"
              aria-pressed={curveType === 'hilbert'}
              onClick={() => chooseCurve('hilbert')}
            >
              Hilbert
            </button>
            <button
              type="button"
              className="sfc-toggle-btn"
              aria-pressed={curveType === 'morton'}
              onClick={() => chooseCurve('morton')}
            >
              Morton
            </button>
          </div>
          <button
            type="button"
            className="sfc-btn sfc-btn-primary sfc-play"
            onClick={play}
            disabled={running}
          >
            {running
              ? 'Playing…'
              : p >= 8
                ? 'Replay subdivision'
                : 'Play subdivision'}
          </button>
          <button type="button" className="sfc-btn sfc-reset" onClick={reset}>
            Reset
          </button>
        </div>
      </div>

      <div className="sfc-summary" role="status" aria-live="polite">
        {summary}
      </div>

      <footer className="sfc-footer">
        <span className="sfc-mono">cities: {N_CITIES_SFC}</span>
        <span className="sfc-mono">lengths: layout units</span>
        <span className="sfc-mono">curve: Hilbert / Morton</span>
      </footer>
    </div>
  )
}

const CSS = `
.sfc-root {
  --accent: #0d9488;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  --tour: #2563eb;
  --seam: #ca8a04;
  --longest: #db2777;
  --jump: #ea580c;
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
.sfc-root * { box-sizing: border-box; }
.sfc-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
}

/* Header */
.sfc-header { margin-bottom: 2px; }
.sfc-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.sfc-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.sfc-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.sfc-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.88em; background: rgba(13,148,136,0.1);
  color: var(--accent); padding: 1px 4px; border-radius: 4px;
}

/* Visualization */
.sfc-viz-row { display: flex; gap: 12px; align-items: stretch; justify-content: center; }
.sfc-canvas-wrap {
  position: relative; flex: 1; min-width: 0; max-width: 440px;
  aspect-ratio: 1 / 1; border: 1px solid var(--line); border-radius: 8px;
  overflow: hidden; background: var(--panel);
}
.sfc-canvas, .sfc-overlay {
  position: absolute; inset: 0; width: 100%; height: 100%; display: block;
}
.sfc-overlay { pointer-events: none; }
.sfc-edge-halo { stroke: var(--panel); stroke-width: 4.6; stroke-linecap: round; }
.sfc-tour-edge { stroke: var(--tour); stroke-width: 2.2; stroke-linecap: round; }
.sfc-seam { stroke: var(--seam); stroke-width: 3.6; }
.sfc-longest { stroke: var(--longest); stroke-width: 3.6; }
.sfc-city { fill: #0f766e; stroke: #fff; stroke-width: 1.2; }
.sfc-city-seam { stroke: var(--seam); stroke-width: 2; }
.sfc-order-badge {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8px; font-weight: 600; fill: #1f2328; text-anchor: middle;
  stroke: #fff; stroke-width: 2.5; paint-order: stroke fill;
  pointer-events: none; user-select: none;
}
.sfc-order-badge-seam { fill: #92400e; }

/* Order panel */
.sfc-order-panel {
  width: 150px; flex-shrink: 0; display: flex; flex-direction: column; gap: 4px;
  padding: 8px; background: var(--panel); border: 1px solid var(--line);
  border-radius: 8px; overflow-y: auto; max-height: 440px;
}
.sfc-order-title {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;
  color: var(--muted);
}
.sfc-order-subtitle { font-size: 0.7rem; color: var(--muted); margin-bottom: 4px; }
.sfc-order-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
.sfc-order-item {
  display: flex; gap: 6px; line-height: 1.2;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.7rem; padding: 0 4px; border-left: 2px solid transparent;
}
.sfc-order-item-seam { border-left-color: var(--seam); background: rgba(202,138,4,0.08); }
.sfc-order-pos { color: var(--muted); min-width: 1.4em; text-align: right; }
.sfc-order-city { color: var(--text); }

/* Legend */
.sfc-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); align-items: center; }
.sfc-swatch {
  display: inline-block; width: 22px; height: 3px;
  border-radius: 2px; margin-right: 3px; vertical-align: middle;
}
.sfc-swatch-curve { background: var(--accent); }
.sfc-swatch-jump { background: var(--jump); }
.sfc-swatch-tour { background: var(--tour); }
.sfc-swatch-seam { background: var(--seam); }
.sfc-swatch-longest { background: var(--longest); }
.sfc-swatch-city { background: #0f766e; width: 10px; height: 10px; border-radius: 50%; }

/* Seam callout */
.sfc-callout {
  font-size: 0.85rem; line-height: 1.55; color: var(--text);
  background: var(--panel); border: 1px solid var(--line);
  border-radius: 8px; padding: 8px 12px;
}
.sfc-callout p { margin: 0 0 4px; }
.sfc-callout p:last-child { margin-bottom: 0; }
.sfc-callout a { color: var(--accent); }

/* Stats */
.sfc-statgrid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
@media (max-width: 540px) { .sfc-statgrid { grid-template-columns: repeat(2, 1fr); } }
.sfc-statlabel {
  font-size: 0.65rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}
.sfc-statvalue { font-size: 0.9rem; }

/* Config */
.sfc-config { display: flex; flex-direction: column; gap: 8px; }
.sfc-config-row { display: flex; flex-direction: column; gap: 3px; }
.sfc-config-label { font-size: 0.88rem; }
.sfc-slider-row { display: flex; align-items: center; gap: 10px; }
.sfc-slider { flex: 1; accent-color: var(--accent); cursor: pointer; }
.sfc-order-value { min-width: 3.5em; text-align: right; }
.sfc-controls { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
.sfc-toggle { display: inline-flex; border: 1px solid var(--line); border-radius: 6px; overflow: hidden; }
.sfc-toggle-btn {
  background: var(--panel); color: var(--text); border: 0;
  border-right: 1px solid var(--line); padding: 6px 12px;
  font-size: 0.88rem; font-family: inherit; cursor: pointer;
}
.sfc-toggle-btn:last-child { border-right: 0; }
.sfc-toggle-btn[aria-pressed="true"] {
  background: rgba(13,148,136,0.12); color: var(--accent); font-weight: 600;
}
.sfc-btn {
  background: var(--panel); color: var(--text);
  border: 1px solid var(--line); border-radius: 6px;
  padding: 6px 16px; font-size: 0.88rem; cursor: pointer; font-family: inherit;
}
.sfc-btn:hover:not(:disabled) { border-color: var(--accent); }
.sfc-btn:disabled { opacity: 0.55; cursor: default; }
.sfc-btn-primary { color: var(--accent); border-color: var(--accent); }
.sfc-btn:focus-visible, .sfc-toggle-btn:focus-visible, .sfc-slider:focus-visible {
  outline: 2px solid var(--accent); outline-offset: 2px;
}

/* Summary */
.sfc-summary {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; line-height: 1.5; color: var(--muted);
  background: var(--panel); border: 1px solid var(--line);
  border-radius: 8px; padding: 8px 12px;
}

/* Footer */
.sfc-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 8px; border-top: 1px solid var(--line); color: var(--muted);
}

@media (max-width: 680px) {
  .sfc-viz-row { flex-direction: column; align-items: stretch; }
  .sfc-canvas-wrap { max-width: none; }
  .sfc-order-panel { width: auto; max-height: 180px; }
}
`

import { Fragment } from 'preact'
import { useState, useRef, useEffect, useMemo } from 'preact/hooks'
import {
  CITIES_SFC,
  N_CITIES_SFC,
  buildLayout,
  orderDiff,
  twoOptSeamPreview,
} from './sfc-algo'
import type {
  Cell,
  CurveType,
  Point,
  SfcBucket,
  SfcLayout,
  TourEdge,
  TwoOptEdge,
} from './sfc-algo'

const VIEW = 300
const PAD = 20
const SIZE = VIEW - PAD * 2
// 2x the 440px display cap, so the canvas stays crisp on HiDPI screens.
const CANVAS_PX = 880

const STRIP_W = 440
const STRIP_H = 100
const STRIP_PAD = 24
const STRIP_Y = 48

const INITIAL_P = 3
const CONSTRUCTION_STAGE_MS = 700
const CONSTRUCTION_EDGE_MS = 180
const SUBDIVISION_MS = 700

type Stage = 'project' | 'index' | 'sort' | 'connect'
type Playback = 'construction' | 'subdivision' | null

type StageInfo = { id: Stage; label: string; lesson: string }

const STAGES: StageInfo[] = [
  {
    id: 'project',
    label: 'Project',
    lesson:
      'Each city falls into one grid cell. Cities sharing a cell will share a curve index.',
  },
  {
    id: 'index',
    label: 'Index',
    lesson:
      'The curve visits every cell once. A cell’s position on the curve is its index — the city’s sort key.',
  },
  {
    id: 'sort',
    label: 'Sort',
    lesson:
      'Sort the cities by index. Same-cell cities tie, so input order breaks the tie.',
  },
  {
    id: 'connect',
    label: 'Connect',
    lesson:
      'Connect the sorted cities in order. The curve only chose the order; the tour does not follow the curve line.',
  },
]

const STAGE_ORDER: Stage[] = ['project', 'index', 'sort', 'connect']

function stageIndex(stage: Stage): number {
  return STAGE_ORDER.indexOf(stage)
}

function nextConstructionState(
  stage: Stage,
  connectStep: number,
): { stage: Stage; connectStep: number } {
  if (stage === 'project') return { stage: 'index', connectStep: 0 }
  if (stage === 'index') return { stage: 'sort', connectStep: 0 }
  if (stage === 'sort') return { stage: 'connect', connectStep: 1 }
  return {
    stage: 'connect',
    connectStep: Math.min(connectStep + 1, N_CITIES_SFC),
  }
}

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

function edgeKey(edge: TourEdge | TwoOptEdge): string {
  return `${edge.from}-${edge.to}`
}

function cityLabel(city: number): string {
  return `city ${city + 1}`
}

function formatLength(value: number): string {
  return String(Math.round(value))
}

function drawCurve(
  canvas: HTMLCanvasElement,
  layout: SfcLayout,
  stage: Stage,
): void {
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

  if (stage === 'project') {
    ctx.globalAlpha = 1
    return
  }

  const cells = layout.cells
  const alpha =
    layout.p <= 4 ? 0.3 : Math.max(0.02, 0.3 * Math.pow(0.5, layout.p - 4))

  const strokeSegments = (
    predicate: (manhattan: number) => boolean,
    color: string,
    opacity: number,
    width: number,
    dash: number[] = [],
  ) => {
    ctx.setLineDash(dash)
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
    ctx.globalAlpha = opacity
    ctx.lineWidth = width
    ctx.stroke()
    ctx.setLineDash([])
  }

  // Adjacent steps form the continuous Hilbert path.
  strokeSegments((manhattan) => manhattan === 1, '#0d9488', alpha, 0.8)
  // Morton's non-adjacent steps are the locality-breaking jumps.
  strokeSegments((manhattan) => manhattan > 1, '#ea580c', alpha * 0.55, 0.7)
  // The longest Morton jumps cross a quadrant boundary.
  const crossingThreshold = Math.max(2, 1 << (layout.p - 1))
  strokeSegments(
    (manhattan) => manhattan >= crossingThreshold,
    '#dc2626',
    0.6,
    1.1,
    [4, 3],
  )

  if (layout.p > 4) {
    const points = layout.order.map((city) =>
      cellPoint(layout.grid[city], layout.p),
    )
    ctx.setLineDash([])
    ctx.beginPath()
    points.forEach(([x, y], index) => {
      if (index === 0) ctx.moveTo(x, y)
      else ctx.lineTo(x, y)
    })
    ctx.strokeStyle = '#0d9488'
    ctx.globalAlpha = 0.62
    ctx.lineWidth = 2
    ctx.stroke()
  }

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

function IndexStrip({
  layout,
  stage,
  hoveredCity,
  visitOf,
  onHover,
  currentEdge,
}: {
  layout: SfcLayout
  stage: Stage
  hoveredCity: number | null
  visitOf: Map<number, number>
  onHover: (city: number | null) => void
  currentEdge: TourEdge | null
}) {
  const stageAt = stageIndex(stage)
  const maxIndex = 4 ** layout.p - 1
  const xOf = (index: number) =>
    maxIndex === 0
      ? STRIP_W / 2
      : STRIP_PAD + (index / maxIndex) * (STRIP_W - STRIP_PAD * 2)

  const ticks: Array<{ city: number; x: number; y: number; visit: number }> = []
  for (const bucket of layout.buckets) {
    const x = xOf(bucket.index)
    bucket.cities.forEach((city, stack) => {
      ticks.push({
        city,
        x,
        y: STRIP_Y - 10 - stack * 6,
        visit: visitOf.get(city) ?? 0,
      })
    })
  }

  const firstCity = layout.order[0]
  const lastCity = layout.order[layout.order.length - 1]
  const firstX = xOf(layout.indices[firstCity])
  const lastX = xOf(layout.indices[lastCity])
  const activeCity = hoveredCity ?? currentEdge?.to ?? null
  const activeTick = ticks.find((tick) => tick.city === activeCity)
  const currentFrom = currentEdge?.from ?? null
  const currentTo = currentEdge?.to ?? null
  const showSeam = stageAt >= 3 && currentEdge?.closing === true

  return (
    <svg
      className="sfc-strip"
      viewBox={`0 0 ${STRIP_W} ${STRIP_H}`}
      role="img"
      aria-label={`1D curve index from 0 to ${maxIndex}. ${layout.stats.cities} cities in ${layout.stats.cellsUsed} cells; ${layout.stats.sharedCells} shared cells.`}
    >
      <line
        x1={STRIP_PAD}
        y1={STRIP_Y}
        x2={STRIP_W - STRIP_PAD}
        y2={STRIP_Y}
        className="sfc-strip-axis"
      />
      <text x={STRIP_PAD} y={STRIP_Y + 16} className="sfc-strip-bound">
        0
      </text>
      <text
        x={STRIP_W - STRIP_PAD}
        y={STRIP_Y + 16}
        textAnchor="end"
        className="sfc-strip-bound"
      >
        {maxIndex}
      </text>

      {showSeam && (
        <path
          d={`M ${lastX} ${STRIP_Y + 8} L ${lastX} ${STRIP_Y + 18} L ${firstX} ${STRIP_Y + 18} L ${firstX} ${STRIP_Y + 8}`}
          className="sfc-strip-seam sfc-strip-seam-current"
        />
      )}

      {ticks.map((tick) => {
        const hovered = hoveredCity === tick.city
        const current = currentFrom === tick.city || currentTo === tick.city
        return (
          <g
            key={tick.city}
            onMouseEnter={() => onHover(tick.city)}
            onMouseLeave={() => onHover(null)}
          >
            <line
              x1={tick.x}
              y1={tick.y}
              x2={tick.x}
              y2={STRIP_Y}
              className="sfc-strip-tick-line"
            />
            <circle
              cx={tick.x}
              cy={tick.y}
              r={hovered || current ? 4.2 : 2.8}
              className={
                hovered || current
                  ? 'sfc-strip-tick sfc-strip-tick-active'
                  : 'sfc-strip-tick'
              }
            />
          </g>
        )
      })}

      {activeTick && (
        <text
          x={activeTick.x}
          y={Math.max(8, activeTick.y - 6)}
          textAnchor="middle"
          className="sfc-strip-label"
        >
          {stageAt >= 2
            ? `visit ${activeTick.visit + 1} · ${cityLabel(activeTick.city)}`
            : cityLabel(activeTick.city)}
        </text>
      )}

      {stageAt >= 2 && (
        <>
          <text
            x={firstX}
            y={STRIP_Y + 32}
            textAnchor="middle"
            className="sfc-strip-visit"
          >
            1
          </text>
          <text
            x={lastX}
            y={STRIP_Y + 32}
            textAnchor="middle"
            className="sfc-strip-visit"
          >
            {N_CITIES_SFC}
          </text>
        </>
      )}

      <text
        x={STRIP_W / 2}
        y={STRIP_Y + 32}
        textAnchor="middle"
        className="sfc-strip-caption"
      >
        1D curve index · hover a dot for its visit
      </text>

      {showSeam && (
        <text
          x={STRIP_W / 2}
          y={STRIP_Y + 46}
          textAnchor="middle"
          className="sfc-strip-seam-label"
        >
          seam wraps from visit {N_CITIES_SFC} back to visit 1
        </text>
      )}

      {currentEdge && activeTick && (
        <line
          x1={activeTick.x}
          y1={8}
          x2={activeTick.x}
          y2={STRIP_Y - 2}
          className="sfc-strip-playhead"
        />
      )}
    </svg>
  )
}

function SortTable({
  layout,
  stage,
  hoveredCity,
  visitOf,
  onHover,
  currentEdge,
  changedCities,
}: {
  layout: SfcLayout
  stage: Stage
  hoveredCity: number | null
  visitOf: Map<number, number>
  onHover: (city: number | null) => void
  currentEdge: TourEdge | null
  changedCities: number[]
}) {
  const stageAt = stageIndex(stage)
  const changedSet = new Set(changedCities)
  const currentFrom = currentEdge?.from ?? null
  const currentTo = currentEdge?.to ?? null
  const seamFirst = layout.order[0]
  const seamLast = layout.order[layout.order.length - 1]

  return (
    <div className="sfc-table-panel">
      <div className="sfc-panel-title">Curve order</div>
      <div className="sfc-panel-subtitle">
        {layout.curveType === 'hilbert' ? 'Hilbert' : 'Morton'} · p = {layout.p}{' '}
        · sort key
      </div>
      <div className="sfc-table-scroll">
        <table className="sfc-table">
          <caption className="sfc-table-caption">
            Cities sorted by curve index
          </caption>
          <thead>
            <tr>
              <th scope="col">Visit</th>
              <th scope="col">City</th>
              <th scope="col">Cell</th>
              <th scope="col">Index</th>
            </tr>
          </thead>
          <tbody>
            {layout.buckets.map((bucket) => (
              <Fragment key={`${bucket.cell[0]}-${bucket.cell[1]}`}>
                {bucket.cities.length > 1 && (
                  <tr className="sfc-group-row">
                    <th colSpan={4} scope="rowgroup">
                      cell ({bucket.cell[0]},{bucket.cell[1]}) · index{' '}
                      {stageAt >= 1 ? bucket.index : '—'} ·{' '}
                      {bucket.cities.length} cities · input order
                    </th>
                  </tr>
                )}
                {bucket.cities.map((city) => {
                  const visit = visitOf.get(city) ?? 0
                  const isChanged = changedSet.has(city)
                  const isCurrent = currentFrom === city || currentTo === city
                  const isSeam =
                    stageAt >= 2 && (city === seamFirst || city === seamLast)
                  const className = [
                    'sfc-row',
                    hoveredCity === city ? 'sfc-row-hover' : '',
                    isChanged ? 'sfc-row-changed' : '',
                    isCurrent ? 'sfc-row-current' : '',
                    isSeam ? 'sfc-row-seam' : '',
                  ]
                    .filter(Boolean)
                    .join(' ')
                  return (
                    <tr
                      key={city}
                      className={className}
                      onMouseEnter={() => onHover(city)}
                      onMouseLeave={() => onHover(null)}
                    >
                      <td>{stageAt >= 2 ? visit + 1 : '—'}</td>
                      <td>#{city + 1}</td>
                      <td>
                        ({bucket.cell[0]},{bucket.cell[1]})
                      </td>
                      <td>{stageAt >= 1 ? bucket.index : '—'}</td>
                    </tr>
                  )
                })}
              </Fragment>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

export default function SfcExplainer() {
  const [p, setP] = useState(INITIAL_P)
  const [curveType, setCurveType] = useState<CurveType>('hilbert')
  const [stage, setStage] = useState<Stage>('project')
  const [connectStep, setConnectStep] = useState(0)
  const [playback, setPlayback] = useState<Playback>(null)
  const [previousLayout, setPreviousLayout] = useState<SfcLayout | null>(null)
  const [hoveredCity, setHoveredCity] = useState<number | null>(null)
  const [preview2Opt, setPreview2Opt] = useState(false)
  const canvasRef = useRef<HTMLCanvasElement | null>(null)

  const layout = useMemo(
    () => buildLayout(CITIES_SFC, p, curveType),
    [p, curveType],
  )
  const changedCities = useMemo(
    () => orderDiff(previousLayout, layout),
    [previousLayout, layout],
  )
  const preview = useMemo(
    () =>
      preview2Opt && stage === 'connect' && connectStep >= N_CITIES_SFC
        ? twoOptSeamPreview(CITIES_SFC, layout.order)
        : null,
    [preview2Opt, stage, connectStep, layout],
  )
  const visitOf = useMemo(() => {
    const map = new Map<number, number>()
    layout.rows.forEach((row) => map.set(row.city, row.visit))
    return map
  }, [layout])

  const stageAt = stageIndex(stage)
  const stageInfo = STAGES[stageAt]
  const currentEdge =
    stage === 'connect' && connectStep > 0
      ? layout.edges[connectStep - 1]
      : null
  const visibleEdges =
    stage === 'connect' ? layout.edges.slice(0, connectStep) : []
  const seamKey = edgeKey(layout.seam)
  const longestKey = edgeKey(layout.longest)
  const seamIsLongest = seamKey === longestKey
  const previewRemovedKeys = new Set(
    (preview?.removed ?? []).map((edge) => edgeKey(edge)),
  )
  const baseEdges = visibleEdges.filter((edge) => {
    const key = edgeKey(edge)
    return key !== seamKey && key !== longestKey && !previewRemovedKeys.has(key)
  })
  const seamVisible =
    visibleEdges.some((edge) => edgeKey(edge) === seamKey) &&
    !previewRemovedKeys.has(seamKey)
  const longestVisible =
    visibleEdges.some((edge) => edgeKey(edge) === longestKey) &&
    !previewRemovedKeys.has(longestKey)
  const currentKey = currentEdge ? edgeKey(currentEdge) : null
  const hoveredBucket =
    hoveredCity === null
      ? null
      : (layout.buckets.find((bucket) => bucket.cities.includes(hoveredCity)) ??
        null)
  const changedSet = new Set(changedCities)
  const showDiff = stageAt >= 2 && previousLayout !== null
  const curveLabel = curveType === 'hilbert' ? 'Hilbert' : 'Morton'
  const gridSize = 1 << p
  const previousCurveLabel = previousLayout
    ? previousLayout.curveType === 'hilbert'
      ? 'Hilbert'
      : 'Morton'
    : ''

  const previousText = previousLayout
    ? ` since p = ${previousLayout.p} (${previousCurveLabel})`
    : ''
  const changedText =
    showDiff && changedCities.length > 0
      ? ` ${changedCities.length} cities changed order${previousText}.`
      : ''
  const edgeText =
    stage === 'connect' ? ` Edge ${connectStep} of ${N_CITIES_SFC}.` : ''
  const tourText =
    stage === 'connect'
      ? ` Tour ${formatLength(layout.stats.tourLength)} layout units; seam ${formatLength(layout.stats.seamLength)}.`
      : ''
  const summary = `${stageInfo.label} stage. ${curveLabel} curve, grid order ${p} (${gridSize} by ${gridSize}). ${layout.stats.cities} cities in ${layout.stats.cellsUsed} cells; ${layout.stats.sharedCells} shared cells.${changedText}${edgeText}${tourText}`

  useEffect(() => {
    if (playback !== 'construction') return
    if (stage === 'connect' && connectStep >= N_CITIES_SFC) {
      setPlayback(null)
      return
    }
    const delay =
      stage === 'connect' ? CONSTRUCTION_EDGE_MS : CONSTRUCTION_STAGE_MS
    const id = window.setTimeout(() => {
      const next = nextConstructionState(stage, connectStep)
      setStage(next.stage)
      setConnectStep(next.connectStep)
    }, delay)
    return () => window.clearTimeout(id)
  }, [playback, stage, connectStep])

  useEffect(() => {
    if (playback !== 'subdivision') return
    if (p >= 8) {
      setPlayback(null)
      return
    }
    const id = window.setTimeout(() => {
      setP((current) => Math.min(current + 1, 8))
    }, SUBDIVISION_MS)
    return () => window.clearTimeout(id)
  }, [playback, p])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const frame = window.requestAnimationFrame(() =>
      drawCurve(canvas, layout, stage),
    )
    return () => window.cancelAnimationFrame(frame)
  }, [layout, stage])

  const goToStage = (next: Stage) => {
    setPlayback(null)
    setPreview2Opt(false)
    setStage(next)
    setConnectStep(next === 'connect' ? N_CITIES_SFC : 0)
  }

  const stepForward = () => {
    setPlayback(null)
    setPreview2Opt(false)
    const next = nextConstructionState(stage, connectStep)
    setStage(next.stage)
    setConnectStep(next.connectStep)
  }

  const stepBack = () => {
    setPlayback(null)
    setPreview2Opt(false)
    if (stage === 'connect') {
      if (connectStep > 0) {
        setConnectStep(connectStep - 1)
        return
      }
      setStage('sort')
      return
    }
    if (stage === 'sort') {
      setStage('index')
      return
    }
    if (stage === 'index') {
      setStage('project')
    }
  }

  const prefersReducedMotion = () =>
    typeof window !== 'undefined' &&
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

  const playConstruction = () => {
    if (playback === 'construction') {
      setPlayback(null)
      return
    }
    setPreview2Opt(false)
    if (prefersReducedMotion()) {
      setPlayback(null)
      setStage('connect')
      setConnectStep(N_CITIES_SFC)
      return
    }
    if (stage === 'connect' && connectStep >= N_CITIES_SFC) {
      setStage('project')
      setConnectStep(0)
    }
    setPlayback('construction')
  }

  const playSubdivision = () => {
    if (playback === 'subdivision') {
      setPlayback(null)
      return
    }
    setPreview2Opt(false)
    if (prefersReducedMotion()) {
      setPlayback(null)
      setP(8)
      return
    }
    if (p >= 8) setP(1)
    setPlayback('subdivision')
  }

  const changeP = (next: number) => {
    const clamped = Math.min(8, Math.max(1, next))
    if (clamped === p) return
    setPlayback(null)
    setPreview2Opt(false)
    setPreviousLayout(layout)
    setP(clamped)
    if (stage === 'connect') setConnectStep(N_CITIES_SFC)
  }

  const changeCurve = (next: CurveType) => {
    if (next === curveType) return
    setPlayback(null)
    setPreview2Opt(false)
    setPreviousLayout(layout)
    setCurveType(next)
    if (stage === 'connect') setConnectStep(N_CITIES_SFC)
  }

  const reset = () => {
    setPlayback(null)
    setStage('project')
    setConnectStep(0)
    setP(INITIAL_P)
    setCurveType('hilbert')
    setPreviousLayout(null)
    setHoveredCity(null)
    setPreview2Opt(false)
  }

  const togglePreview = () => {
    setPlayback(null)
    setPreview2Opt((current) => !current)
  }

  const renderEdge = (edge: TourEdge, className: string) => {
    const [x1, y1] = cityPoint(CITIES_SFC[edge.from], layout)
    const [x2, y2] = cityPoint(CITIES_SFC[edge.to], layout)
    const key = edgeKey(edge)
    const current = currentKey === key
    const classes = `${className}${current ? ' sfc-current' : ''}`
    return (
      <g key={key}>
        <line x1={x1} y1={y1} x2={x2} y2={y2} className="sfc-edge-halo" />
        <line x1={x1} y1={y1} x2={x2} y2={y2} className={classes} />
        {seamIsLongest && className.includes('sfc-seam') && (
          <line
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            className="sfc-longest-overlap"
          />
        )}
      </g>
    )
  }

  const renderPreviewRemoved = (edge: TourEdge) => {
    const [x1, y1] = cityPoint(CITIES_SFC[edge.from], layout)
    const [x2, y2] = cityPoint(CITIES_SFC[edge.to], layout)
    return (
      <line
        key={`removed-${edgeKey(edge)}`}
        x1={x1}
        y1={y1}
        x2={x2}
        y2={y2}
        className="sfc-preview-removed"
      />
    )
  }

  const renderPreviewAdded = (edge: TwoOptEdge) => {
    const [x1, y1] = cityPoint(CITIES_SFC[edge.from], layout)
    const [x2, y2] = cityPoint(CITIES_SFC[edge.to], layout)
    return (
      <line
        key={`added-${edgeKey(edge)}`}
        x1={x1}
        y1={y1}
        x2={x2}
        y2={y2}
        className="sfc-preview-added"
      />
    )
  }

  const renderBucket = (bucket: SfcBucket) => {
    const n = 1 << p
    const cellSize = SIZE / n
    const x = PAD + bucket.cell[0] * cellSize
    const y = VIEW - PAD - (bucket.cell[1] + 1) * cellSize
    const hovered = hoveredBucket === bucket
    const shared = bucket.cities.length > 1
    const showIndex = stageAt >= 1 && p <= 4
    const showCount = shared && p <= 5
    if (p <= 5) {
      return (
        <g key={`${bucket.cell[0]}-${bucket.cell[1]}`}>
          <rect
            x={x}
            y={y}
            width={cellSize}
            height={cellSize}
            className={shared ? 'sfc-cell sfc-cell-shared' : 'sfc-cell'}
          />
          {hovered && (
            <rect
              x={x}
              y={y}
              width={cellSize}
              height={cellSize}
              className="sfc-cell-hover"
            />
          )}
          {showIndex && (
            <text
              x={x + cellSize / 2}
              y={y + cellSize / 2 + 2}
              textAnchor="middle"
              className="sfc-cell-index"
            >
              {bucket.index}
            </text>
          )}
          {showCount && (
            <text
              x={x + cellSize - 3}
              y={y + 9}
              textAnchor="end"
              className="sfc-cell-count"
            >
              {bucket.cities.length}
            </text>
          )}
        </g>
      )
    }
    return (
      <g key={`${bucket.cell[0]}-${bucket.cell[1]}`}>
        <circle
          cx={x + cellSize / 2}
          cy={y + cellSize / 2}
          r={hovered ? 5 : 2.4}
          className={
            hovered ? 'sfc-cell-dot sfc-cell-dot-hover' : 'sfc-cell-dot'
          }
        />
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
          The curve is a <strong>1D sort key</strong>, not the tour path. Each
          city falls into a cell, the cell gets a curve index, and the tour
          connects the cities in that index order. Step through the stages to
          watch the construction.
        </p>
      </header>

      <div
        className="sfc-stagebar"
        role="group"
        aria-label="Construction stage"
      >
        {STAGES.map((item) => (
          <button
            key={item.id}
            type="button"
            className={
              stage === item.id
                ? 'sfc-stage-btn sfc-stage-btn-active'
                : 'sfc-stage-btn'
            }
            aria-pressed={stage === item.id}
            onClick={() => goToStage(item.id)}
          >
            {item.label}
          </button>
        ))}
      </div>

      <div className="sfc-lesson">
        <div className="sfc-lesson-label">{stageInfo.label}</div>
        <p>{stageInfo.lesson}</p>
      </div>

      {showDiff && changedCities.length > 0 && (
        <div className="sfc-change-note">
          <strong>{changedCities.length} cities changed order</strong>
          {previousText}. The highlighted cities moved to different cells;
          same-cell cities still use input order.
        </div>
      )}

      <div className="sfc-viz-row">
        <div className="sfc-viz">
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
              {layout.buckets.map((bucket) => renderBucket(bucket))}
              {baseEdges.map((edge) => renderEdge(edge, 'sfc-tour-edge'))}
              {longestVisible &&
                !seamIsLongest &&
                renderEdge(layout.longest, 'sfc-tour-edge sfc-longest')}
              {seamVisible && renderEdge(layout.seam, 'sfc-tour-edge sfc-seam')}
              {preview?.removed.map((edge) => renderPreviewRemoved(edge))}
              {preview?.added.map((edge) => renderPreviewAdded(edge))}
              {CITIES_SFC.map((city, index) => {
                const [x, y] = cityPoint(city, layout)
                const position = visitOf.get(index) ?? 0
                const isSeamEndpoint =
                  stageAt >= 2 &&
                  (position === 0 || position === N_CITIES_SFC - 1)
                const isHovered = hoveredCity === index
                const isChanged = showDiff && changedSet.has(index)
                const isCurrent =
                  currentEdge?.from === index || currentEdge?.to === index
                const badge =
                  stageAt >= 2 ? String(position + 1) : `#${index + 1}`
                return (
                  <g
                    key={index}
                    onMouseEnter={() => setHoveredCity(index)}
                    onMouseLeave={() => setHoveredCity(null)}
                  >
                    {isHovered && (
                      <circle cx={x} cy={y} r={8} className="sfc-city-hover" />
                    )}
                    <circle
                      cx={x}
                      cy={y}
                      r={isSeamEndpoint ? 5.5 : 4.5}
                      className={[
                        'sfc-city',
                        isSeamEndpoint ? 'sfc-city-seam' : '',
                        isCurrent ? 'sfc-city-current' : '',
                      ]
                        .filter(Boolean)
                        .join(' ')}
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
                      {badge}
                    </text>
                    {isChanged && (
                      <circle
                        cx={x + 6}
                        cy={y + 6}
                        r={1.8}
                        className="sfc-city-changed"
                      />
                    )}
                  </g>
                )
              })}
            </svg>
          </div>

          {stageAt >= 1 ? (
            <IndexStrip
              layout={layout}
              stage={stage}
              hoveredCity={hoveredCity}
              visitOf={visitOf}
              onHover={setHoveredCity}
              currentEdge={currentEdge}
            />
          ) : (
            <div className="sfc-strip-placeholder">
              The 1D curve index appears in the Index stage.
            </div>
          )}
        </div>

        <SortTable
          layout={layout}
          stage={stage}
          hoveredCity={hoveredCity}
          visitOf={visitOf}
          onHover={setHoveredCity}
          currentEdge={currentEdge}
          changedCities={showDiff ? changedCities : []}
        />
      </div>

      <div className="sfc-legend">
        <span>
          <span className="sfc-swatch sfc-swatch-city" /> city ·{' '}
          {stageAt >= 2 ? 'badge = visit order' : '#index'}
        </span>
        <span>
          <span className="sfc-swatch sfc-swatch-cell" /> grid cell
        </span>
        <span>
          <span className="sfc-swatch sfc-swatch-cell sfc-swatch-cell-shared" />{' '}
          shared cell
        </span>
        {stageAt >= 1 && (
          <>
            <span>
              <span className="sfc-swatch sfc-swatch-curve" /> curve
            </span>
            <span>
              <span className="sfc-swatch sfc-swatch-jump" /> long curve jump
            </span>
          </>
        )}
        {stageAt >= 2 && (
          <span>
            <span className="sfc-swatch sfc-swatch-order" /> visit order
          </span>
        )}
        {stageAt >= 3 && (
          <>
            <span>
              <span className="sfc-swatch sfc-swatch-tour" /> tour edge
            </span>
            <span>
              <span className="sfc-swatch sfc-swatch-seam" /> seam (closing
              edge)
            </span>
            <span>
              <span className="sfc-swatch sfc-swatch-longest" /> longest edge
            </span>
          </>
        )}
        {preview2Opt && (
          <span>
            <span className="sfc-swatch sfc-swatch-preview" /> 2-opt preview
          </span>
        )}
      </div>

      {curveType === 'morton' && stageAt >= 1 && (
        <div className="sfc-curve-note">
          Morton jumps across quadrant boundaries. The dashed red steps are the
          longest of those Z-order crossings; Hilbert has no such jumps.
        </div>
      )}

      {stage === 'connect' && connectStep >= N_CITIES_SFC && (
        <div className="sfc-callout">
          <p>
            <strong>Seam:</strong> visit {N_CITIES_SFC} → visit 1 (
            {cityLabel(layout.seam.from)} → {cityLabel(layout.seam.to)}),{' '}
            {formatLength(layout.seam.length)} layout units.
            {seamIsLongest ? (
              <> It is also the longest edge.</>
            ) : (
              <>
                {' '}
                <strong>Longest edge:</strong> {cityLabel(layout.longest.from)}{' '}
                → {cityLabel(layout.longest.to)},{' '}
                {formatLength(layout.longest.length)} layout units.
              </>
            )}
          </p>
          <p>
            A 2-opt pass removes the seam and one other edge, then reconnects
            the four endpoints. That is why SFC tours are usually refined with{' '}
            <a href="/algorithms/two-opt/">2-opt</a> or{' '}
            <a href="/algorithms/lin-kernighan/">Lin-Kernighan</a>.
          </p>
          <div className="sfc-preview-row">
            <button
              type="button"
              className="sfc-btn sfc-preview-btn"
              onClick={togglePreview}
              aria-pressed={preview2Opt}
            >
              {preview2Opt ? 'Hide 2-opt preview' : 'Preview 2-opt fix'}
            </button>
            {preview && (
              <span className="sfc-mono sfc-preview-delta">
                {formatLength(preview.oldLength)} →{' '}
                {formatLength(preview.newLength)} (
                {preview.delta <= 0 ? '−' : '+'}
                {Math.abs((preview.delta / preview.oldLength) * 100).toFixed(1)}
                %)
                {preview.delta > 0 ? ' — no improving seam move' : ''}
              </span>
            )}
          </div>
        </div>
      )}

      <div className="sfc-statgrid">
        <Stat label="grid order" value={`p = ${p}`} />
        <Stat
          label="cities / cells"
          value={`${layout.stats.cities} / ${layout.stats.cellsUsed}`}
        />
        <Stat label="shared cells" value={String(layout.stats.sharedCells)} />
        <Stat
          label="largest bucket"
          value={String(layout.stats.largestBucket)}
        />
        {stageAt >= 1 && (
          <Stat
            label="long curve jumps"
            value={String(layout.stats.longJumps)}
          />
        )}
        {showDiff && changedCities.length > 0 && (
          <Stat label="changed cities" value={String(changedCities.length)} />
        )}
        {stage === 'connect' && (
          <>
            <Stat
              label="tour length"
              value={formatLength(layout.stats.tourLength)}
            />
            <Stat
              label="seam length"
              value={formatLength(layout.stats.seamLength)}
            />
            <Stat
              label="longest edge"
              value={formatLength(layout.stats.longestEdgeLength)}
            />
          </>
        )}
      </div>

      <div className="sfc-config">
        <div className="sfc-config-row">
          <label className="sfc-config-label" htmlFor="sfc-order">
            Grid order p
          </label>
          <div className="sfc-slider-row">
            <button
              type="button"
              className="sfc-btn sfc-step-btn sfc-prev-p"
              onClick={() => changeP(p - 1)}
              disabled={p <= 1}
              aria-label="Decrease grid order"
            >
              −
            </button>
            <input
              id="sfc-order"
              className="sfc-slider"
              type="range"
              min={1}
              max={8}
              step={1}
              value={p}
              onInput={(event) =>
                changeP(Number((event.currentTarget as HTMLInputElement).value))
              }
              aria-valuetext={`grid order ${p}, ${gridSize} by ${gridSize} grid`}
            />
            <button
              type="button"
              className="sfc-btn sfc-step-btn sfc-next-p"
              onClick={() => changeP(p + 1)}
              disabled={p >= 8}
              aria-label="Increase grid order"
            >
              +
            </button>
            <span className="sfc-mono sfc-order-value">p = {p}</span>
            <button
              type="button"
              className="sfc-btn sfc-play-subdivision"
              onClick={playSubdivision}
            >
              {playback === 'subdivision'
                ? '⏸ Pause'
                : p >= 8
                  ? '▶ Replay subdivision'
                  : '▶ Play subdivision'}
            </button>
          </div>
        </div>

        <div className="sfc-controls">
          <button
            type="button"
            className="sfc-btn sfc-back"
            onClick={stepBack}
            disabled={stage === 'project'}
          >
            ◀ Back
          </button>
          <button
            type="button"
            className="sfc-btn sfc-step"
            onClick={stepForward}
            disabled={stage === 'connect' && connectStep >= N_CITIES_SFC}
          >
            Step ▶
          </button>
          <button
            type="button"
            className="sfc-btn sfc-btn-primary sfc-run"
            onClick={playConstruction}
          >
            {playback === 'construction'
              ? '⏸ Pause'
              : stage === 'connect' && connectStep >= N_CITIES_SFC
                ? '↻ Replay'
                : '▶ Run'}
          </button>
          <button type="button" className="sfc-btn sfc-reset" onClick={reset}>
            ↺ Reset
          </button>
        </div>

        <div className="sfc-toggle-row">
          <span className="sfc-config-label">Curve</span>
          <div className="sfc-toggle" role="group" aria-label="Curve type">
            <button
              type="button"
              className="sfc-toggle-btn"
              aria-pressed={curveType === 'hilbert'}
              onClick={() => changeCurve('hilbert')}
            >
              Hilbert
            </button>
            <button
              type="button"
              className="sfc-toggle-btn"
              aria-pressed={curveType === 'morton'}
              onClick={() => changeCurve('morton')}
            >
              Morton
            </button>
          </div>
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
  --tour: #4f46e5;
  --seam: #ca8a04;
  --longest: #db2777;
  --jump: #ea580c;
  --preview: #16a34a;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 20px;
  max-width: 860px;
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
.sfc-sub strong { color: var(--text); }

/* Stage bar and lesson */
.sfc-stagebar { display: flex; flex-wrap: wrap; gap: 4px; }
.sfc-stage-btn {
  background: var(--panel); color: var(--muted); border: 1px solid var(--line);
  border-radius: 6px; padding: 5px 12px; font-size: 0.82rem;
  font-family: inherit; cursor: pointer;
}
.sfc-stage-btn-active {
  background: rgba(79,70,229,0.1); border-color: var(--tour);
  color: var(--tour); font-weight: 600;
}
.sfc-lesson {
  border-left: 3px solid var(--accent); background: var(--panel);
  border-radius: 0 8px 8px 0; padding: 7px 12px;
}
.sfc-lesson-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.68rem; letter-spacing: 0.06em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 2px;
}
.sfc-lesson p { margin: 0; font-size: 0.86rem; line-height: 1.5; }
.sfc-change-note {
  background: #fef3c7; border: 1px solid #fde68a; border-radius: 8px;
  padding: 7px 12px; font-size: 0.82rem; line-height: 1.5; color: #92400e;
}
.sfc-curve-note {
  background: #fff7ed; border: 1px solid #fed7aa; border-radius: 8px;
  padding: 7px 12px; font-size: 0.82rem; line-height: 1.5; color: #9a3412;
}

/* Viz layout */
.sfc-viz-row { display: flex; gap: 12px; align-items: stretch; justify-content: center; }
.sfc-viz { flex: 1; min-width: 0; max-width: 440px; display: flex; flex-direction: column; gap: 8px; }
.sfc-canvas-wrap {
  position: relative; width: 100%; aspect-ratio: 1 / 1;
  border: 1px solid var(--line); border-radius: 8px;
  overflow: hidden; background: var(--panel);
}
.sfc-canvas, .sfc-overlay {
  position: absolute; inset: 0; width: 100%; height: 100%; display: block;
}
.sfc-overlay { pointer-events: none; }

/* Cells */
.sfc-cell { fill: rgba(13,148,136,0.06); stroke: rgba(13,148,136,0.3); stroke-width: 0.6; }
.sfc-cell-shared { stroke: var(--seam); stroke-width: 1.2; stroke-dasharray: 3 2; }
.sfc-cell-hover { fill: rgba(79,70,229,0.12); stroke: var(--tour); stroke-width: 1.5; }
.sfc-cell-index {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 7px; fill: #0f766e; pointer-events: none;
}
.sfc-cell-count {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 7px; font-weight: 700; fill: #92400e; pointer-events: none;
}
.sfc-cell-dot { fill: rgba(13,148,136,0.45); }
.sfc-cell-dot-hover { fill: var(--tour); }

/* Edges */
.sfc-edge-halo { stroke: var(--panel); stroke-width: 4.6; stroke-linecap: round; }
.sfc-tour-edge { stroke: var(--tour); stroke-width: 2.2; stroke-linecap: round; }
.sfc-tour-edge.sfc-current { stroke-width: 3.1; }
.sfc-seam { stroke: var(--seam); stroke-width: 3.6; }
.sfc-longest { stroke: var(--longest); stroke-width: 3.6; }
.sfc-longest-overlap {
  stroke: var(--longest); stroke-width: 2.2; stroke-dasharray: 4 3;
  stroke-linecap: round;
}
.sfc-preview-removed {
  stroke: #9ca3af; stroke-width: 2; stroke-dasharray: 4 3;
  stroke-linecap: round; opacity: 0.7;
}
.sfc-preview-added {
  stroke: var(--preview); stroke-width: 2.6; stroke-dasharray: 5 3;
  stroke-linecap: round;
}

/* Cities */
.sfc-city { fill: #0f766e; stroke: #fff; stroke-width: 1.2; }
.sfc-city-seam { stroke: var(--seam); stroke-width: 2; }
.sfc-city-current { stroke: var(--tour); stroke-width: 2; }
.sfc-city-hover { fill: rgba(79,70,229,0.16); stroke: var(--tour); stroke-width: 1; }
.sfc-city-changed { fill: var(--seam); stroke: #fff; stroke-width: 0.7; }
.sfc-order-badge {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 8px; font-weight: 600; fill: #1f2328; text-anchor: middle;
  stroke: #fff; stroke-width: 2.5; paint-order: stroke fill;
  pointer-events: none; user-select: none;
}
.sfc-order-badge-seam { fill: #92400e; }

/* 1D strip */
.sfc-strip { width: 100%; display: block; background: var(--panel); border: 1px solid var(--line); border-radius: 8px; }
.sfc-strip-axis { stroke: var(--line); stroke-width: 1; }
.sfc-strip-bound {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 7px; fill: var(--muted);
}
.sfc-strip-tick-line { stroke: rgba(13,148,136,0.35); stroke-width: 0.8; }
.sfc-strip-tick { fill: #0d9488; stroke: #fff; stroke-width: 0.8; }
.sfc-strip-tick-active { fill: var(--tour); stroke: #fff; stroke-width: 1.2; }
.sfc-strip-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 7px; fill: var(--tour); font-weight: 600;
}
.sfc-strip-visit {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 7px; fill: var(--muted);
}
.sfc-strip-seam {
  fill: none; stroke: var(--seam); stroke-width: 1.3; stroke-dasharray: 3 2;
}
.sfc-strip-seam-current { stroke-width: 2.2; }
.sfc-strip-caption {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 6.5px; fill: var(--muted);
}
.sfc-strip-seam-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 6.5px; fill: #92400e;
}
.sfc-strip-placeholder {
  height: 100px; display: flex; align-items: center; justify-content: center;
  border: 1px dashed var(--line); border-radius: 8px;
  background: var(--panel); color: var(--muted); font-size: 0.8rem;
}
.sfc-strip-playhead { stroke: var(--tour); stroke-width: 1.6; stroke-dasharray: 3 2; }

/* Sort table */
.sfc-table-panel {
  width: 262px; flex-shrink: 0; display: flex; flex-direction: column;
  gap: 4px; padding: 8px; background: var(--panel);
  border: 1px solid var(--line); border-radius: 8px;
  max-height: 548px; overflow: hidden;
}
.sfc-panel-title {
  font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em;
  color: var(--muted);
}
.sfc-panel-subtitle { font-size: 0.7rem; color: var(--muted); margin-bottom: 2px; }
.sfc-table-scroll { overflow: auto; min-height: 0; }
.sfc-table { width: 100%; border-collapse: collapse; font-size: 0.7rem; }
.sfc-table-caption {
  caption-side: top; text-align: left; font-size: 0.62rem; color: var(--muted);
  padding: 0 0 2px;
}
.sfc-table th, .sfc-table td {
  padding: 0 4px; line-height: 1.2; text-align: left;
  border-bottom: 1px solid rgba(208,215,222,0.6);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  white-space: nowrap;
}
.sfc-table thead th {
  position: sticky; top: 0; background: var(--panel); color: var(--muted);
  font-size: 0.62rem; text-transform: uppercase; letter-spacing: 0.04em;
  padding: 2px 4px; z-index: 1;
}
.sfc-table .sfc-group-row th {
  background: rgba(202,138,4,0.1); color: #92400e; font-size: 0.64rem;
  font-weight: 600; border-bottom: 1px solid #fde68a; padding: 1px 4px;
}
.sfc-row { cursor: default; }
.sfc-row-hover { background: rgba(79,70,229,0.1); }
.sfc-row-current { background: rgba(79,70,229,0.16); font-weight: 600; }
.sfc-row-changed td:first-child { color: var(--seam); font-weight: 700; }
.sfc-row-seam td:first-child { border-left: 2px solid var(--seam); }

/* Legend, callout, stats */
.sfc-legend { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); align-items: center; }
.sfc-swatch {
  display: inline-block; width: 22px; height: 3px;
  border-radius: 2px; margin-right: 3px; vertical-align: middle;
}
.sfc-swatch-city { background: #0f766e; width: 10px; height: 10px; border-radius: 50%; }
.sfc-swatch-cell { background: rgba(13,148,136,0.2); border: 1px solid rgba(13,148,136,0.4); }
.sfc-swatch-cell-shared { background: rgba(202,138,4,0.15); border: 1px dashed var(--seam); }
.sfc-swatch-curve { background: var(--accent); }
.sfc-swatch-jump { background: var(--jump); }
.sfc-swatch-order { background: var(--tour); }
.sfc-swatch-tour { background: var(--tour); }
.sfc-swatch-seam { background: var(--seam); }
.sfc-swatch-longest { background: var(--longest); }
.sfc-swatch-preview { background: var(--preview); }

.sfc-callout {
  font-size: 0.85rem; line-height: 1.55; color: var(--text);
  background: var(--panel); border: 1px solid var(--line);
  border-radius: 8px; padding: 8px 12px;
}
.sfc-callout p { margin: 0 0 4px; }
.sfc-callout p:last-of-type { margin-bottom: 0; }
.sfc-callout a { color: var(--accent); }
.sfc-preview-row { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-top: 6px; }
.sfc-preview-delta { font-size: 0.78rem; color: #166534; }

.sfc-statgrid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
@media (max-width: 640px) { .sfc-statgrid { grid-template-columns: repeat(2, 1fr); } }
.sfc-statlabel {
  font-size: 0.65rem; color: var(--muted);
  text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 2px;
}
.sfc-statvalue { font-size: 0.9rem; }

/* Controls */
.sfc-config { display: flex; flex-direction: column; gap: 8px; }
.sfc-config-row { display: flex; flex-direction: column; gap: 3px; }
.sfc-config-label { font-size: 0.88rem; }
.sfc-slider-row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.sfc-slider { flex: 1; min-width: 120px; accent-color: var(--accent); cursor: pointer; }
.sfc-step-btn { padding: 4px 10px; }
.sfc-order-value { min-width: 3.5em; text-align: right; }
.sfc-controls { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
.sfc-toggle-row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
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
.sfc-btn:disabled { opacity: 0.5; cursor: default; }
.sfc-btn-primary { color: var(--tour); border-color: var(--tour); }
.sfc-stage-btn:focus-visible, .sfc-btn:focus-visible,
.sfc-toggle-btn:focus-visible, .sfc-slider:focus-visible {
  outline: 2px solid var(--tour); outline-offset: 2px;
}

/* Summary and footer */
.sfc-summary {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; line-height: 1.5; color: var(--muted);
  background: var(--panel); border: 1px solid var(--line);
  border-radius: 8px; padding: 8px 12px;
}
.sfc-footer {
  display: flex; flex-wrap: wrap; gap: 14px;
  padding-top: 8px; border-top: 1px solid var(--line); color: var(--muted);
}

@media (max-width: 720px) {
  .sfc-viz-row { flex-direction: column; align-items: stretch; }
  .sfc-viz { max-width: none; }
  .sfc-table-panel { width: auto; max-height: 260px; }
}
`

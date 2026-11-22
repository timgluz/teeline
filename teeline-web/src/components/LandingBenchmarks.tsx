// Runtime against quality for every solver measured on one instance — the landing chart.
//
// A client island for the same reason as the algorithm page's section: the shard is fetched at
// runtime so republishing does not need a redeploy, and the build must not fail when R2 is
// unreachable (decision 9). It replaces a hand-maintained array that had drifted from the
// published data (20 of 22 solvers, a phantom `hk` entry, and `aco`/`bhk`/`shuffle` missing).

import { useEffect, useState } from 'preact/hooks'

import { axisTop, hullPath, niceTicks, placeLabels } from '../lib/chart-layout'
import {
  FAMILY_COLORS,
  FAMILY_LABELS,
  UNKNOWN_FAMILY_COLOR,
} from '../lib/family-palette'
import { solverLabel } from '../lib/solver-label'
import {
  fetchBenchmarkIndex,
  fetchProblemShard,
  solverSeries,
  type BenchmarkIndex,
  type ChartAxis,
  type ProblemShard,
  type SolverFamily,
  type SolverPoint,
  type SolverSeries,
} from '../lib/benchmark'

export interface LandingBenchmarksProps {
  /** The instance the chart plots. berlin52 is the page's own story. */
  instance: string
  dev: boolean
}

type State =
  | { kind: 'loading' }
  | { kind: 'absent' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; shard: ProblemShard; index: BenchmarkIndex | null }

const AXIS_COLOR = '#232327'
const LABEL_COLOR = '#8A8A92'
const TEXT_COLOR = '#F5F5F6'
const FAMILIES: SolverFamily[] = [
  'exact',
  'constructive',
  'local',
  'meta',
  'utility',
]
/** Tooltip size estimate, so it can be clamped inside the viewport before it paints. */
const TOOLTIP_WIDTH = 224
const TOOLTIP_HEIGHT = 108
/** Shared by the caption and the chart's aria-label, so the two cannot drift apart. */
const CLUSTER_NOTE = 'Dashed outlines group solvers by family.'

function formatY(axis: ChartAxis, value: number): string {
  return axis === 'gap' ? `${value.toFixed(1)}%` : value.toFixed(0)
}

function timeLabel(seconds: number): string {
  return seconds < 1 ? `${Math.round(seconds * 1000)}ms` : `${seconds}s`
}

export default function LandingBenchmarks({
  instance,
  dev,
}: LandingBenchmarksProps) {
  const [state, setState] = useState<State>({ kind: 'loading' })
  const [hidden, setHidden] = useState<Set<SolverFamily>>(() => new Set())

  useEffect(() => {
    let cancelled = false
    void (async () => {
      const [result, index] = await Promise.all([
        fetchProblemShard(instance, dev),
        fetchBenchmarkIndex(dev),
      ])
      if (cancelled) return
      if (result.kind === 'ok')
        setState({ kind: 'ready', shard: result.shard, index })
      else if (result.kind === 'absent') setState({ kind: 'absent' })
      else setState({ kind: 'error', message: result.message })
    })()
    return () => {
      cancelled = true
    }
  }, [instance, dev])

  if (state.kind === 'absent') return null

  if (state.kind !== 'ready') {
    // Not `null`: a `client:visible` island with no box never intersects, so it would never
    // hydrate and never fetch. The placeholder is what makes the deferral work.
    return (
      <section
        id="benchmarks"
        class="mx-auto max-w-content px-6 py-16 lg:py-24"
        aria-busy="true"
      >
        <p class="mb-3 font-mono text-xs uppercase tracking-wider text-accent">
          benchmarks
        </p>
        <h2 class="text-2xl font-semibold tracking-tight lg:text-4xl">
          Every measured solver, one chart.
        </h2>
        <p class="mt-3 text-text-dim">
          {state.kind === 'error'
            ? `The published benchmark data could not be read (${state.message}).`
            : 'Loading published results…'}
        </p>
      </section>
    )
  }

  const series = solverSeries(state.shard)
  const cities = state.shard.cities
  const budgets = state.index?.budgets ?? null

  // Cross-family comparison is what this chart does, and the allowance differs per family, so
  // the budgets are stated wherever a gap is plotted (decision 4).
  const budgetLine =
    series.axis === 'gap' && budgets
      ? FAMILIES.filter((family) => budgets[family]).map(
          (family) =>
            `${FAMILY_LABELS[family]} ${budgets[family].seconds} s${
              budgets[family].agreed ? '' : ' (provisional)'
            }`,
        )
      : []

  const subtitle =
    series.axis === 'gap'
      ? `Runtime against the gap from the known optimum on ${instance}${
          cities ? ` (${cities} cities)` : ''
        }. Lower-left is better: faster and closer to optimal.`
      : `Runtime against tour length on ${instance}${
          cities ? ` (${cities} cities)` : ''
        }, because no optimum is published for this instance — so this shows scale, not quality.`

  const omitted = [
    series.omitted.noResult > 0
      ? `${series.omitted.noResult} did not finish`
      : '',
    series.omitted.noQuality > 0 && series.axis === 'gap'
      ? `${series.omitted.noQuality} have no published optimum`
      : '',
  ].filter(Boolean)

  // Only families with a point get a toggle: a chip that hides nothing is worse than no chip.
  const presentFamilies = FAMILIES.filter((family) =>
    series.points.some((point) => point.family === family),
  )
  const visiblePresent = presentFamilies.filter((family) => !hidden.has(family))
  const shownPoints = series.points.filter(
    (point) => point.family === null || !hidden.has(point.family),
  )
  const shownSolvers = new Set(shownPoints.map((point) => point.solver)).size
  const hiddenPoints = series.points.length - shownPoints.length

  // The cap is computed from every point, not the visible subset, so filtering a family does
  // not rescale the axis under the reader.
  const yTop = axisTop(series.points.map((point) => point.y))
  const offScale = shownPoints.filter((point) => point.y > yTop)
  const shufflePoint = series.points.find((point) => point.solver === 'shuffle')
  const capNote =
    offScale.length === 0
      ? null
      : [
          `The axis stops at ${formatY(series.axis, yTop)} so the comparable solvers fill it; ${
            offScale.length
          } ${offScale.length === 1 ? 'result is' : 'results are'} above it and not drawn to scale:`,
          `${offScale
            .slice()
            .sort((a, b) => b.y - a.y)
            .map(
              (point) =>
                `${solverLabel(point.solver)} ${formatY(series.axis, point.y)}`,
            )
            .join(', ')}.`,
          shufflePoint && offScale.some((point) => point.solver === 'shuffle')
            ? `Random shuffle, the sanity check that nothing should beat, is ${formatY(series.axis, shufflePoint.y)}.`
            : '',
        ]
          .filter(Boolean)
          .join(' ')

  function toggleFamily(family: SolverFamily): void {
    setHidden((current) => {
      const next = new Set(current)
      if (next.has(family)) {
        next.delete(family)
        return next
      }
      // Never hide the last family with points: an empty plot is a dead end.
      const remaining = presentFamilies.filter((other) => !next.has(other))
      if (remaining.length <= 1) return current
      next.add(family)
      return next
    })
  }

  return (
    <section id="benchmarks" class="mx-auto max-w-content px-6 py-16 lg:py-24">
      <div class="mb-10 max-w-2xl">
        <p class="mb-3 font-mono text-xs uppercase tracking-wider text-accent">
          benchmarks
        </p>
        <h2 class="text-2xl font-semibold tracking-tight lg:text-4xl">
          Every measured solver, one chart.
        </h2>
        <p class="mt-3 text-text-dim">{subtitle}</p>
      </div>

      <div class="rounded-lg border border-border bg-surface p-4 lg:p-6">
        {series.points.length > 0 ? (
          <Scatter series={series} points={shownPoints} yTop={yTop} />
        ) : (
          <p class="text-sm text-text-dim">
            No solver has a published result for this instance yet.
          </p>
        )}

        <div class="mt-4 flex flex-wrap items-center gap-x-6 gap-y-2 border-t border-border pt-4">
          {presentFamilies.map((family) => {
            const isHidden = hidden.has(family)
            const isLastVisible =
              !isHidden &&
              visiblePresent.length === 1 &&
              visiblePresent[0] === family
            return (
              <button
                key={family}
                type="button"
                aria-pressed={!isHidden}
                disabled={isLastVisible}
                title={
                  isLastVisible
                    ? 'At least one family stays visible'
                    : `${isHidden ? 'Show' : 'Hide'} ${FAMILY_LABELS[family]}`
                }
                onClick={() => toggleFamily(family)}
                class={`inline-flex items-center gap-2 font-mono text-xs transition-opacity disabled:cursor-not-allowed ${
                  isHidden ? 'opacity-40' : ''
                }`}
              >
                <svg
                  class="inline-block h-2 w-2"
                  viewBox="0 0 8 8"
                  aria-hidden="true"
                >
                  <circle cx="4" cy="4" r="4" fill={FAMILY_COLORS[family]} />
                </svg>
                <span class={isHidden ? 'text-text line-through' : 'text-text'}>
                  {FAMILY_LABELS[family]}
                </span>
              </button>
            )
          })}
          {hidden.size > 0 && (
            <button
              type="button"
              onClick={() => setHidden(new Set())}
              class="font-mono text-xs text-accent no-underline hover:underline"
            >
              show all
            </button>
          )}
          <span class="ml-auto font-mono text-xs text-text-dim">
            {shownSolvers} solver
            {shownSolvers === 1 ? '' : 's'}
            {/* A solver publishing more than one config is more than one point, so
                "solvers" must not be the point count — each config is a separate result. */}
            {shownPoints.length > shownSolvers
              ? ` · ${shownPoints.length} results`
              : ''}{' '}
            · {instance}
            {hiddenPoints > 0 ? ` · ${hiddenPoints} hidden` : ''}
            {omitted.length > 0 ? ` · not plotted: ${omitted.join(', ')}` : ''}
          </span>
        </div>

        {capNote && (
          <p class="mt-2 font-mono text-xs text-text-dim">{capNote}</p>
        )}

        {budgetLine.length > 0 && (
          <p class="mt-2 font-mono text-xs text-text-dim">
            Budgets differ by family, so gaps across families are not
            like-for-like: {budgetLine.join(' · ')}
          </p>
        )}
      </div>
    </section>
  )
}

function Scatter({
  series,
  points,
  yTop,
}: {
  series: SolverSeries
  /** The points the family filter left visible; the axes stay on the full series. */
  points: SolverPoint[]
  yTop: number
}) {
  const [hovered, setHovered] = useState<{
    point: SolverPoint
    left: number
    top: number
  } | null>(null)

  const W = 720
  const H = 420
  const PAD = { left: 64, right: 24, top: 24, bottom: 48 }
  const plotW = W - PAD.left - PAD.right
  const plotH = H - PAD.top - PAD.bottom

  // Runtime spans milliseconds to minutes, so the x axis is logarithmic: on a linear axis every
  // fast solver would collapse into a single column at the origin.
  const logs = series.points.map((point) =>
    Math.log10(Math.max(point.wallS, 1e-6)),
  )
  let xMin = Math.floor(Math.min(...logs))
  let xMax = Math.ceil(Math.max(...logs))
  if (xMax === xMin) {
    xMin -= 0.5
    xMax += 0.5
  }
  const sx = (wallS: number) =>
    PAD.left +
    ((Math.log10(Math.max(wallS, 1e-6)) - xMin) / (xMax - xMin)) * plotW

  // Rounded ticks, top tick bounding the plot: a linear split of a tour length gives labels like
  // 8293 and 16586, which the reader has to do arithmetic to use.
  const yTicks = niceTicks(yTop)
  const sy = (y: number) => PAD.top + (1 - y / yTop) * plotH

  const xTicks: number[] = []
  for (let power = xMin; power <= xMax; power += 1) {
    xTicks.push(power)
  }

  const inRange = points.filter((point) => point.y <= yTop)
  const offScale = points.filter((point) => point.y > yTop)

  // Families are the chart's grouping already (colour and chips); a hull makes the shape of
  // each group visible without changing what any point claims. Off-scale points are left out:
  // a hull around a clamped point would draw a boundary the data does not have.
  const familyClusters = FAMILIES.flatMap((family) => {
    const path = hullPath(
      inRange
        .filter((point) => point.family === family)
        .map((point) => ({ x: sx(point.wallS), y: sy(point.y) })),
    )
    return path ? [{ family, path }] : []
  })

  const labelled = new Set(
    series.labelled.map((entry) => `${entry.solver}/${entry.config}`),
  )
  const placements = placeLabels(
    inRange
      .filter((point) => labelled.has(`${point.solver}/${point.config}`))
      .map((point) => ({
        text: point.solver,
        x: sx(point.wallS),
        y: sy(point.y),
      })),
    { width: W, height: H },
  )

  function showTooltip(
    point: SolverPoint,
    event: { clientX: number; clientY: number },
  ): void {
    setHovered({
      point,
      left: Math.min(event.clientX + 14, window.innerWidth - TOOLTIP_WIDTH - 8),
      top: Math.min(
        event.clientY + 14,
        window.innerHeight - TOOLTIP_HEIGHT - 8,
      ),
    })
  }

  function pointLabel(point: SolverPoint): string {
    return `${solverLabel(point.solver)}: ${formatY(series.axis, point.y)}, ${timeLabel(point.wallS)}`
  }

  const configCount = (point: SolverPoint): number =>
    series.points.filter((other) => other.solver === point.solver).length

  return (
    <>
      <svg
        viewBox={`0 0 ${W} ${H}`}
        class="h-auto w-full"
        role="img"
        aria-label={`Runtime against ${series.axisLabel} on ${series.dataset}${
          familyClusters.length > 0 ? `; ${CLUSTER_NOTE}` : ''
        }`}
      >
        {/* Behind the grid and the points: a highlight, not a layer the reader has to see past. */}
        <g aria-hidden="true" pointer-events="none">
          {familyClusters.map(({ family, path }) => (
            <path
              key={family}
              d={path}
              fill={FAMILY_COLORS[family]}
              fill-opacity="0.04"
              stroke={FAMILY_COLORS[family]}
              stroke-opacity="0.45"
              stroke-width="1.25"
              stroke-dasharray="5 4"
              stroke-linejoin="round"
              data-family-cluster={family}
            />
          ))}
        </g>
        {yTicks.map((value) => (
          <g key={`y${value}`}>
            <line
              x1={PAD.left}
              y1={sy(value)}
              x2={PAD.left + plotW}
              y2={sy(value)}
              stroke={AXIS_COLOR}
              stroke-width="1"
            />
            <text
              x={PAD.left - 8}
              y={sy(value) + 3}
              text-anchor="end"
              font-size="10"
              fill={LABEL_COLOR}
              font-family="JetBrains Mono, monospace"
            >
              {formatY(series.axis, value)}
            </text>
          </g>
        ))}
        {xTicks.map((power) => (
          <g key={`x${power}`}>
            <line
              x1={sx(Math.pow(10, power))}
              y1={PAD.top}
              x2={sx(Math.pow(10, power))}
              y2={PAD.top + plotH}
              stroke={AXIS_COLOR}
              stroke-width="1"
            />
            <text
              x={sx(Math.pow(10, power))}
              y={H - 22}
              text-anchor="middle"
              font-size="10"
              fill={LABEL_COLOR}
              font-family="JetBrains Mono, monospace"
            >
              {timeLabel(Math.pow(10, power))}
            </text>
          </g>
        ))}
        <text
          x={PAD.left + plotW / 2}
          y={H - 6}
          text-anchor="middle"
          font-size="11"
          fill={LABEL_COLOR}
        >
          runtime (log scale) →
        </text>
        <text
          x={14}
          y={PAD.top + plotH / 2}
          text-anchor="middle"
          font-size="11"
          fill={LABEL_COLOR}
          transform={`rotate(-90 14 ${PAD.top + plotH / 2})`}
        >
          {series.axis === 'gap' ? 'gap vs optimal →' : 'tour length →'}
        </text>
        <text
          x={PAD.left + 6}
          y={PAD.top + 14}
          font-size="11"
          fill="#22D3A5"
          font-family="JetBrains Mono, monospace"
        >
          better
        </text>

        {inRange.map((point) => (
          <circle
            key={`${point.solver}/${point.config}`}
            cx={sx(point.wallS)}
            cy={sy(point.y)}
            r={hovered?.point === point ? 6 : 4}
            fill={
              point.family ? FAMILY_COLORS[point.family] : UNKNOWN_FAMILY_COLOR
            }
            opacity="0.9"
            stroke={hovered?.point === point ? TEXT_COLOR : 'none'}
            stroke-width="1.5"
            class="cursor-pointer"
            aria-label={pointLabel(point)}
            onPointerEnter={(event) => showTooltip(point, event)}
            onPointerMove={(event) => showTooltip(point, event)}
            onPointerLeave={() => setHovered(null)}
          />
        ))}

        {/* Above the cap: keep the runtime position so the point stays findable, but mark it as
            off-scale rather than letting it decide the axis for everyone else. */}
        {offScale.map((point) => (
          <path
            key={`off-${point.solver}/${point.config}`}
            d={`M${sx(point.wallS) - 5} ${PAD.top + 9} L${sx(point.wallS) + 5} ${PAD.top + 9} L${sx(point.wallS)} ${PAD.top} Z`}
            fill={
              point.family ? FAMILY_COLORS[point.family] : UNKNOWN_FAMILY_COLOR
            }
            opacity="0.9"
            class="cursor-pointer"
            aria-label={`${pointLabel(point)} (above the axis cap)`}
            onPointerEnter={(event) => showTooltip(point, event)}
            onPointerMove={(event) => showTooltip(point, event)}
            onPointerLeave={() => setHovered(null)}
          />
        ))}

        {placements.map((placement) => (
          <text
            key={placement.text}
            x={placement.x}
            y={placement.y}
            text-anchor={placement.anchor}
            font-size="10"
            fill={TEXT_COLOR}
            font-family="JetBrains Mono, monospace"
          >
            {placement.text}
          </text>
        ))}
      </svg>

      {familyClusters.length > 0 && (
        <p class="mt-2 font-mono text-xs text-text-dim">{CLUSTER_NOTE}</p>
      )}

      {hovered && (
        <div
          class="pointer-events-none fixed z-50 w-56 rounded-md border border-border bg-surface px-3 py-2 font-mono text-xs shadow-lg"
          style={{ left: `${hovered.left}px`, top: `${hovered.top}px` }}
          role="tooltip"
        >
          <div class="font-semibold text-text">
            {solverLabel(hovered.point.solver)}
          </div>
          {configCount(hovered.point) > 1 && (
            <div class="text-text-dim">{hovered.point.config}</div>
          )}
          <div class="mt-1 text-text-dim">
            runtime{' '}
            <span class="text-text">{timeLabel(hovered.point.wallS)}</span>
          </div>
          <div class="text-text-dim">
            {series.axis === 'gap' ? 'gap' : 'tour'}{' '}
            <span class="text-text">
              {formatY(series.axis, hovered.point.y)}
            </span>
          </div>
          {hovered.point.family && (
            <div class="text-text-dim">
              {FAMILY_LABELS[hovered.point.family]}
            </div>
          )}
          {hovered.point.y > yTop && (
            <div class="mt-1 text-text-dim">above the axis cap</div>
          )}
        </div>
      )}
    </>
  )
}

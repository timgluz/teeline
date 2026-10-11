// Runtime against quality for every solver measured on one instance — the landing chart.
//
// A client island for the same reason as the algorithm page's section: the shard is fetched at
// runtime so republishing does not need a redeploy, and the build must not fail when R2 is
// unreachable (decision 9). It replaces a hand-maintained array that had drifted from the
// published data (20 of 22 solvers, a phantom `hk` entry, and `aco`/`bhk`/`shuffle` missing).

import { useEffect, useState } from 'preact/hooks'

import { niceTicks, placeLabels } from '../lib/chart-layout'
import {
  FAMILY_COLORS,
  FAMILY_LABELS,
  UNKNOWN_FAMILY_COLOR,
} from '../lib/family-palette'
import {
  fetchBenchmarkIndex,
  fetchProblemShard,
  solverSeries,
  type BenchmarkIndex,
  type ProblemShard,
  type SolverFamily,
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

export default function LandingBenchmarks({
  instance,
  dev,
}: LandingBenchmarksProps) {
  const [state, setState] = useState<State>({ kind: 'loading' })

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
          <Scatter series={series} />
        ) : (
          <p class="text-sm text-text-dim">
            No solver has a published result for this instance yet.
          </p>
        )}

        <div class="mt-4 flex flex-wrap items-center gap-x-6 gap-y-2 border-t border-border pt-4">
          {FAMILIES.map((family) => (
            <span
              key={family}
              class="inline-flex items-center gap-2 font-mono text-xs"
            >
              <svg
                class="inline-block h-2 w-2"
                viewBox="0 0 8 8"
                aria-hidden="true"
              >
                <circle cx="4" cy="4" r="4" fill={FAMILY_COLORS[family]} />
              </svg>
              <span class="text-text">{FAMILY_LABELS[family]}</span>
            </span>
          ))}
          <span class="ml-auto font-mono text-xs text-text-dim">
            {series.points.length} solver
            {series.points.length === 1 ? '' : 's'} · {instance}
            {omitted.length > 0 ? ` · not plotted: ${omitted.join(', ')}` : ''}
          </span>
        </div>

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

function Scatter({ series }: { series: SolverSeries }) {
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

  const yMax = Math.max(...series.points.map((point) => point.y))
  // Rounded ticks, top tick bounding the plot: a linear split of a tour length gives labels like
  // 8293 and 16586, which the reader has to do arithmetic to use.
  const yTicks = niceTicks(yMax)
  const yTop = yTicks[yTicks.length - 1]
  const sy = (y: number) => PAD.top + (1 - y / yTop) * plotH

  const xTicks: number[] = []
  for (let power = xMin; power <= xMax; power += 1) {
    xTicks.push(power)
  }
  const yFormat =
    series.axis === 'gap'
      ? (v: number) => `${v.toFixed(1)}%`
      : (v: number) => v.toFixed(0)
  const timeLabel = (seconds: number) =>
    seconds < 1 ? `${Math.round(seconds * 1000)}ms` : `${seconds}s`

  const labelled = new Set(
    series.labelled.map((entry) => `${entry.solver}/${entry.config}`),
  )
  const placements = placeLabels(
    series.points
      .filter((point) => labelled.has(`${point.solver}/${point.config}`))
      .map((point) => ({
        text: point.solver,
        x: sx(point.wallS),
        y: sy(point.y),
      })),
    { width: W, height: H },
  )

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      class="h-auto w-full"
      role="img"
      aria-label={`Runtime against ${series.axisLabel} on ${series.dataset}`}
    >
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
            {yFormat(value)}
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

      {series.points.map((point) => (
        <circle
          key={`${point.solver}/${point.config}`}
          cx={sx(point.wallS)}
          cy={sy(point.y)}
          r="4"
          fill={
            point.family ? FAMILY_COLORS[point.family] : UNKNOWN_FAMILY_COLOR
          }
          opacity="0.9"
        >
          <title>
            {point.solver}
            {` (${point.config}): ${yFormat(point.y)}, ${timeLabel(point.wallS)}`}
          </title>
        </circle>
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
  )
}

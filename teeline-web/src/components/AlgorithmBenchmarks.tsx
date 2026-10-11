// Benchmark section for /algorithms/[id]/: what this solver scored on each published
// instance, and how it scales with instance size.
//
// A client island because the data is fetched at runtime: decision 9 keeps republishing a
// shard from requiring a site redeploy, and the build must not fail when R2 is unreachable.
// Every number comes from the solver's own shard; only display metadata (a problem's name
// and distance type, which the shard does not carry) arrives as a build-time prop.

import { useEffect, useState } from 'preact/hooks'

import { solverById } from '../lib/solver-index'
import {
  DASH,
  buildRows,
  budgetFor,
  chartSeries,
  chooseColumns,
  fetchAlgorithmShard,
  fetchBenchmarkIndex,
  formatGapPct,
  formatSeconds,
  formatTour,
  provenance,
  summarise,
  timeoutNote,
  type AlgorithmShard,
  type BenchmarkIndex,
  type BenchmarkRow,
  type ChartSeries,
  type InstanceMeta,
} from '../lib/benchmark'

export interface AlgorithmBenchmarksProps {
  /** Algorithm doc id, which is also the shard name on R2. */
  id: string
  /** Vite's dev flag: dev fetches through the proxy, production from R2 directly. */
  dev: boolean
  /** Problem metadata by dataset id, from the `problems` collection. */
  instances: Record<string, InstanceMeta>
}

type State =
  | { kind: 'loading' }
  /** No shard published for this solver: a normal state, and it renders nothing. */
  | { kind: 'absent' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; shard: AlgorithmShard; index: BenchmarkIndex | null }

const AXIS_COLOR = '#232327'
const LABEL_COLOR = '#8A8A92'
const TEXT_COLOR = '#F5F5F6'
const POINT_COLOR = '#22D3A5'

export default function AlgorithmBenchmarks({
  id,
  dev,
  instances,
}: AlgorithmBenchmarksProps) {
  const [state, setState] = useState<State>({ kind: 'loading' })

  useEffect(() => {
    let cancelled = false
    void (async () => {
      // The manifest is fetched alongside the shard because it is the only published
      // statement of the budget each family was measured under, and a gap without it is
      // not renderable. It is allowed to fail: the gap column then goes, nothing else.
      const [result, index] = await Promise.all([
        fetchAlgorithmShard(id, dev),
        fetchBenchmarkIndex(dev),
      ])
      if (cancelled) return
      if (result.kind === 'ok') {
        setState({ kind: 'ready', shard: result.shard, index })
      } else if (result.kind === 'absent') {
        setState({ kind: 'absent' })
      } else {
        setState({ kind: 'error', message: result.message })
      }
    })()
    return () => {
      cancelled = true
    }
  }, [id, dev])

  if (state.kind === 'absent') return null
  if (state.kind === 'loading') {
    // Deliberately not `null`. This island is `client:visible`, and an element that renders
    // nothing has no box to intersect: the observer would never fire and the fetch would
    // never start. The placeholder is what makes the deferral work at all.
    return (
      <section
        class="mt-10"
        aria-busy="true"
        aria-labelledby="benchmarks-heading"
      >
        <h2
          id="benchmarks-heading"
          class="text-lg font-semibold tracking-tight lg:text-xl"
        >
          Benchmarks
        </h2>
        <p class="mt-2 text-sm text-text-dim">Loading published results…</p>
      </section>
    )
  }
  if (state.kind === 'error') {
    return (
      <section
        class="mt-10 rounded-lg border border-border bg-surface p-4 lg:p-6"
        aria-labelledby="benchmarks-heading"
      >
        <h2
          id="benchmarks-heading"
          class="text-lg font-semibold tracking-tight lg:text-xl"
        >
          Benchmarks
        </h2>
        <p class="mt-2 text-sm text-text-dim">
          The published benchmark data could not be read ({state.message}). This
          is a failure to load, not a solver without results.
        </p>
      </section>
    )
  }

  const { shard, index } = state
  const rows = buildRows(shard, instances)
  // An empty shard is the same fact as no shard: nothing measured yet.
  if (rows.length === 0) return null

  const family = solverById(shard.solver)?.family
  const budget = family ? budgetFor(index, family) : null
  const columns = chooseColumns(rows, budget?.seconds ?? null)
  const summary = summarise(rows, budget)
  const series = chartSeries(rows)
  const lines = provenance(shard.measurement)

  const hasTour = rows.some((row) => row.tour !== null)
  const hasWall = rows.some((row) => row.wall !== null)

  return (
    <section class="mt-10" aria-labelledby="benchmarks-heading">
      <h2
        id="benchmarks-heading"
        class="text-lg font-semibold tracking-tight lg:text-xl"
      >
        Benchmarks
      </h2>

      <p class="mt-2 text-sm text-text-dim">
        {summary.instances} instance{summary.instances === 1 ? '' : 's'}{' '}
        measured under the{' '}
        {budget
          ? `${family} budget of ${budget.seconds} s${
              budget.agreed ? '' : ' (provisional)'
            }`
          : 'published budget, which could not be read'}
        .{' '}
        {columns.gap
          ? `Median gap ${formatGapPct(summary.medianGapPct)} across ${
              summary.instancesWithOptimum
            } instance${
              summary.instancesWithOptimum === 1 ? '' : 's'
            } with a known optimum.`
          : 'No instance here has a published optimum, so there is no gap to show.'}{' '}
        {hasWall && summary.medianWallS !== null
          ? `Median wall time ${formatSeconds(summary.medianWallS)}.`
          : ''}
        {summary.timeouts > 0
          ? ` ${summary.timeouts} run${
              summary.timeouts === 1 ? '' : 's'
            } did not finish within the budget.`
          : ''}
      </p>

      {!columns.gap && summary.instancesWithOptimum > 0 && (
        <p class="mt-2 text-sm text-text-dim">
          The gap column is hidden until the published budget can be read,
          because a gap without the allowance it was measured under cannot be
          compared.
        </p>
      )}

      <div class="mt-4 overflow-x-auto rounded-lg border border-border bg-surface">
        <table class="w-full border-collapse text-sm">
          <caption class="sr-only">
            Published benchmark results per instance
            {columns.spread
              ? '; median, best and worst over the runs of a solver whose tours vary between runs'
              : ''}
          </caption>
          <thead>
            <tr class="border-b border-border">
              <Th>Instance</Th>
              <Th right>Cities</Th>
              <Th>Type</Th>
              {columns.optimal && <Th right>Optimal</Th>}
              {columns.gap && <Th right>Gap (median)</Th>}
              {columns.gap && columns.spread && <Th right>Gap (best)</Th>}
              {columns.gap && columns.spread && <Th right>Gap (worst)</Th>}
              {hasTour && <Th right>Tour (median)</Th>}
              {hasTour && columns.spread && <Th right>Tour (best–worst)</Th>}
              {hasWall && <Th right>Wall (median)</Th>}
              <Th right>Runs</Th>
              <Th right>Lost runs</Th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <Row
                key={`${row.dataset}/${row.config}`}
                row={row}
                columns={columns}
                hasTour={hasTour}
                hasWall={hasWall}
                multiConfig={summary.configs.length > 1}
              />
            ))}
          </tbody>
        </table>
      </div>

      {series.points.length > 0 && (
        <figure class="mt-6 rounded-lg border border-border bg-surface p-4 lg:p-6">
          <Scatter series={series} />
          <figcaption class="mt-3 border-t border-border pt-3 font-mono text-xs text-text-dim">
            {series.axis === 'gap'
              ? `Gap against known optimum, one point per instance. ${
                  series.omitted.noOptimum > 0
                    ? `${series.omitted.noOptimum} instance${
                        series.omitted.noOptimum === 1 ? '' : 's'
                      } without a published optimum ${series.omitted.noOptimum === 1 ? 'is' : 'are'} not plotted — a gap and a tour length are not the same axis.`
                    : ''
                }`
              : 'Tour length against instance size, because none of these instances has a published optimum. Lower is better for a given size, but the values are not comparable across instances.'}
            {series.omitted.noResult > 0
              ? ` ${series.omitted.noResult} cell${
                  series.omitted.noResult === 1 ? '' : 's'
                } had no completed run and ${series.omitted.noResult === 1 ? 'is' : 'are'} not plotted.`
              : ''}
            {summary.configs.length > 1
              ? ' One point per instance per config.'
              : ''}
          </figcaption>
        </figure>
      )}

      <div class="mt-6 border-t border-border pt-4">
        <h3 class="font-mono text-xs uppercase tracking-wider text-text-dim">
          Provenance
        </h3>
        <dl class="mt-2 grid grid-cols-1 gap-x-6 gap-y-1 text-xs sm:grid-cols-2">
          {lines.map((line) => (
            <div key={line.label} class="flex gap-2">
              <dt class="min-w-24 text-text-dim">{line.label}</dt>
              <dd class="m-0 font-mono">{line.value}</dd>
            </div>
          ))}
        </dl>
        <p class="mt-2 text-xs text-text-dim">
          These numbers were measured from this solver's own build. Another
          algorithm's page may legitimately show a different commit.
        </p>
      </div>
    </section>
  )
}

function Th({
  children,
  right = false,
}: {
  children: unknown
  right?: boolean
}) {
  return (
    <th
      scope="col"
      class={`px-3 py-2 font-semibold text-text-dim ${
        right ? 'text-right' : 'text-left'
      }`}
    >
      {children}
    </th>
  )
}

function Row({
  row,
  columns,
  hasTour,
  hasWall,
  multiConfig,
}: {
  row: BenchmarkRow
  columns: { gap: boolean; optimal: boolean; spread: boolean }
  hasTour: boolean
  hasWall: boolean
  multiConfig: boolean
}) {
  return (
    <tr class="border-b border-border last:border-b-0">
      <td class="px-3 py-2">
        <a
          href={`/problems/${row.dataset}/`}
          class="text-accent no-underline hover:underline"
        >
          {row.name}
        </a>
        {multiConfig && (
          <span class="ml-2 font-mono text-xs text-text-dim">{row.config}</span>
        )}
      </td>
      <td class="px-3 py-2 text-right font-mono">
        {row.cities === null ? DASH : row.cities}
      </td>
      <td class="px-3 py-2 font-mono text-xs">{row.edgeWeightType ?? DASH}</td>
      {columns.optimal && (
        <td class="px-3 py-2 text-right font-mono">
          {row.optimal === null ? DASH : row.optimal.toLocaleString()}
        </td>
      )}
      {columns.gap && (
        <td class="px-3 py-2 text-right font-mono">
          {formatGapPct(row.gap?.median ?? null)}
        </td>
      )}
      {columns.gap && columns.spread && (
        <td class="px-3 py-2 text-right font-mono">
          {formatGapPct(row.gap?.best ?? null)}
        </td>
      )}
      {columns.gap && columns.spread && (
        <td class="px-3 py-2 text-right font-mono">
          {formatGapPct(row.gap?.worst ?? null)}
        </td>
      )}
      {hasTour && (
        <td class="px-3 py-2 text-right font-mono">
          {formatTour(row.tour?.median ?? null)}
        </td>
      )}
      {hasTour && columns.spread && (
        <td class="px-3 py-2 text-right font-mono">
          {row.tour
            ? `${formatTour(row.tour.best)}–${formatTour(row.tour.worst)}`
            : DASH}
        </td>
      )}
      {hasWall && (
        <td class="px-3 py-2 text-right font-mono">
          {formatSeconds(row.wall?.median ?? null)}
        </td>
      )}
      <td class="px-3 py-2 text-right font-mono">{row.runs}</td>
      <td class="px-3 py-2 text-right font-mono">{timeoutNote(row)}</td>
    </tr>
  )
}

/**
 * Instance size against quality.
 *
 * The y value is whatever chartSeries chose — never both. Points are labelled with their
 * dataset id because there are few of them and a legend would be longer than the chart.
 */
function Scatter({ series }: { series: ChartSeries }) {
  const W = 720
  const H = 340
  const PAD = { left: 64, right: 28, top: 20, bottom: 44 }
  const plotW = W - PAD.left - PAD.right
  const plotH = H - PAD.top - PAD.bottom

  const xs = series.points.map((point) => point.x)
  const ys = series.points.map((point) => point.y)
  const xMin = Math.min(...xs)
  const xMax = Math.max(...xs)
  // Both axes start at zero: an axis that starts near the minimum would exaggerate the
  // differences between solvers on the same instance.
  const yMax = Math.max(...ys)
  const yTop = yMax > 0 ? yMax * 1.15 : 1

  const sx = (x: number) =>
    xMax === xMin
      ? PAD.left + plotW / 2
      : PAD.left + ((x - xMin) / (xMax - xMin)) * plotW
  const sy = (y: number) => PAD.top + (1 - y / yTop) * plotH

  const yTicks = [0, 0.25, 0.5, 0.75, 1].map((f) => f * yTop)
  const xTicks = [0, 0.5, 1].map((f) => xMin + f * (xMax - xMin))
  const yFormat =
    series.axis === 'gap'
      ? (v: number) => `${v.toFixed(1)}%`
      : (v: number) => v.toFixed(0)

  // Labels are placed greedily: the first offset clear of every label already placed wins.
  // Instances a few cities apart (ulysses16 and ulysses22 differ by six) would otherwise
  // draw their labels over each other, and a chart whose labels cannot be read hides the
  // numbers it exists to show. Widths are estimated from the monospace advance.
  const placed: { x: number; y: number; w: number; h: number }[] = []
  const labels = series.points.map((point) => {
    const cx = sx(point.x)
    const cy = sy(point.y)
    const w = point.dataset.length * 7
    const h = 13
    const candidates = [
      { dx: 8, dy: 3, anchor: 'start' as const },
      { dx: 8, dy: 15, anchor: 'start' as const },
      { dx: -8, dy: 3, anchor: 'end' as const },
      { dx: 8, dy: -8, anchor: 'start' as const },
    ]
    for (const candidate of candidates) {
      const x =
        candidate.anchor === 'end' ? cx + candidate.dx - w : cx + candidate.dx
      const y = cy + candidate.dy - 8
      const box = { x, y, w, h }
      const outside = box.x < 2 || box.x + box.w > W - 2
      const collides = placed.some(
        (other) =>
          box.x < other.x + other.w &&
          other.x < box.x + box.w &&
          box.y < other.y + other.h &&
          other.y < box.y + box.h,
      )
      if (!outside && !collides) {
        placed.push(box)
        return { point, cx, cy, ...candidate }
      }
    }
    // Every candidate collided: keep the label beside its point rather than dropping it.
    const fallback = candidates[0]
    placed.push({ x: cx + fallback.dx, y: cy + fallback.dy - 8, w, h })
    return { point, cx, cy, ...fallback }
  })

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      class="h-auto w-full"
      role="img"
      aria-label={series.axisLabel}
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
      {xTicks.map((value) => (
        <g key={`x${value}`}>
          <text
            x={sx(value)}
            y={H - 20}
            text-anchor="middle"
            font-size="10"
            fill={LABEL_COLOR}
            font-family="JetBrains Mono, monospace"
          >
            {Math.round(value)}
          </text>
        </g>
      ))}
      <text
        x={PAD.left + plotW / 2}
        y={H - 4}
        text-anchor="middle"
        font-size="11"
        fill={LABEL_COLOR}
      >
        cities →
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
      {labels.map(({ point, cx, cy, dx, dy, anchor }) => (
        <g key={`${point.dataset}/${point.config}`}>
          <circle cx={cx} cy={cy} r="4" fill={POINT_COLOR} />
          <text
            x={cx + dx}
            y={cy + dy}
            text-anchor={anchor}
            font-size="10"
            fill={TEXT_COLOR}
            font-family="JetBrains Mono, monospace"
          >
            {point.dataset}
          </text>
          <title>
            {point.label}
            {`: x ${point.x} cities, y ${yFormat(point.y)}`}
          </title>
        </g>
      ))}
    </svg>
  )
}

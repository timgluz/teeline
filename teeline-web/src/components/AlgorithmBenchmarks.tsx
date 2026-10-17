// Benchmark section for /algorithms/[id]/: what this solver scored on each published
// instance, and how it scales with instance size.
//
// A client island because the data is fetched at runtime: decision 9 keeps republishing a
// shard from requiring a site redeploy, and the build must not fail when R2 is unreachable.
// Every number comes from the solver's own shard; only display metadata (a problem's name
// and distance type, which the shard does not carry) arrives as a build-time prop.

import type { ComponentChildren } from 'preact'
import { useEffect, useState } from 'preact/hooks'

import { niceTicks, placeLabels } from '../lib/chart-layout'
import { SOLVER_INDEX, solverById } from '../lib/solver-index'
import { solverLabel } from '../lib/solver-label'
import {
  DASH,
  algorithmShardUrl,
  buildRows,
  budgetFor,
  chartSeries,
  chooseColumns,
  comparisonSeries,
  fetchAlgorithmShard,
  fetchBenchmarkIndex,
  formatGapPct,
  formatSeconds,
  formatTour,
  indexUrl,
  lostRunsNote,
  observations,
  pairRows,
  provenance,
  sharedInstances,
  summarise,
  timeoutNote,
  type AlgorithmShard,
  type BenchmarkIndex,
  type BenchmarkRow,
  type BudgetEntry,
  type ChartAxis,
  type ChartSeries,
  type ComparisonPair,
  type ComparisonSeries,
  type InstanceMeta,
  type ProvenanceLine,
  type Summary,
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

/** The second shard, once a reader has chosen something to compare against. */
type CompareState =
  | { kind: 'none' }
  | { kind: 'loading' }
  | { kind: 'absent' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; shard: AlgorithmShard }

const AXIS_COLOR = '#232327'
const LABEL_COLOR = '#8A8A92'
const TEXT_COLOR = '#F5F5F6'
const POINT_COLOR = '#22D3A5'
/** The comparator's series, distinct from the primary's at a glance. */
const COMPARED_COLOR = '#6366F1'
/** The data contract, which lives in the repo rather than on the site. */
const SCHEMA_URL =
  'https://github.com/timgluz/teeline/blob/master/docs/benchmarks/schema.md'

const SOLVER_IDS = new Set(SOLVER_INDEX.map((solver) => solver.id))

/** Every solver that publishes a shard, by display name — the picker's options. */
const COMPARE_OPTIONS = [...SOLVER_INDEX]
  .map((solver) => ({ id: solver.id, name: solverLabel(solver.id) }))
  .sort((a, b) => a.name.localeCompare(b.name))

/**
 * A gap with its spread when the runs disagreed.
 *
 * A comparator whose tours vary must not be summarised as one number, the same rule the primary
 * follows with its own best/worst columns. A deterministic solver's best and worst *are* its
 * median, so the spread would only be noise in that cell.
 */
function gapWithSpread(row: BenchmarkRow): string {
  if (row.gap === null) return DASH
  if (row.deterministic) return formatGapPct(row.gap.median)
  return `${formatGapPct(row.gap.median)} (${formatGapPct(
    row.gap.best,
  )}–${formatGapPct(row.gap.worst)})`
}

function summaryGapWithSpread(summary: Summary): string {
  if (summary.deterministic) return formatGapPct(summary.medianGapPct)
  return `${formatGapPct(summary.medianGapPct)} (${formatGapPct(
    summary.medianGapBestPct,
  )}–${formatGapPct(summary.medianGapWorstPct)})`
}

interface Comparison {
  solver: string
  name: string
  rows: BenchmarkRow[]
  budget: BudgetEntry | null
  summary: Summary
  pairs: ComparisonPair[]
  series: ComparisonSeries
  /** Instances both solvers were measured on, under the same config. */
  shared: number
  lines: ProvenanceLine[]
}

function buildComparison(
  primary: AlgorithmShard,
  compared: AlgorithmShard,
  primaryRows: BenchmarkRow[],
  instances: Record<string, InstanceMeta>,
  index: BenchmarkIndex | null,
): Comparison {
  const rows = buildRows(compared, instances)
  const family = solverById(compared.solver)?.family
  const budget = family ? budgetFor(index, family) : null
  const pairs = pairRows(primaryRows, rows)
  return {
    solver: compared.solver,
    name: solverLabel(compared.solver),
    rows,
    budget,
    summary: summarise(rows, budget),
    pairs,
    series: comparisonSeries(pairs, primary.solver, compared.solver),
    shared: sharedInstances(pairs),
    lines: provenance(compared.measurement),
  }
}

/** One drawn point, whichever chart is being rendered. */
interface ChartDatum {
  key: string
  x: number
  y: number
  colour: string
  /** Drawn beside the point. Omitted for a second series, whose instances are already labelled. */
  label?: string
  tooltip: string
}

function singleSeriesData(series: ChartSeries): ChartDatum[] {
  return series.points.map((point) => ({
    key: `${point.dataset}/${point.config}`,
    x: point.x,
    y: point.y,
    colour: POINT_COLOR,
    label: point.dataset,
    tooltip: `${point.label}: ${yValue(series.axis, point.y)}`,
  }))
}

function comparisonData(
  comparison: Comparison,
  primarySolver: string,
): ChartDatum[] {
  const { axis } = comparison.series
  return comparison.series.points.map((point) => ({
    key: `${point.solver}/${point.dataset}/${point.config}`,
    x: point.x,
    y: point.y,
    colour: point.solver === primarySolver ? POINT_COLOR : COMPARED_COLOR,
    // Only the primary's points are labelled: the comparator sits on the same instances, and
    // twenty labels on a scatter is noise rather than information.
    label: point.solver === primarySolver ? point.dataset : undefined,
    tooltip: `${point.solver}: ${yValue(axis, point.y)}`,
  }))
}

function yValue(axis: ChartAxis, value: number): string {
  return axis === 'gap' ? formatGapPct(value) : formatTour(value)
}

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

  const [compareId, setCompareId] = useState<string | null>(null)
  const [compare, setCompare] = useState<CompareState>({ kind: 'none' })

  // `?compare=<id>` makes a comparison a link worth sharing: read it once at hydration, and
  // rewrite it on change so the address bar keeps matching what is on screen.
  useEffect(() => {
    const requested = new URLSearchParams(window.location.search).get('compare')
    if (requested !== null && requested !== id && SOLVER_IDS.has(requested)) {
      setCompareId(requested)
    }
  }, [id])

  useEffect(() => {
    if (compareId === null) {
      setCompare({ kind: 'none' })
      return
    }
    let cancelled = false
    setCompare({ kind: 'loading' })
    void (async () => {
      const result = await fetchAlgorithmShard(compareId, dev)
      if (cancelled) return
      if (result.kind === 'ok') {
        setCompare({ kind: 'ready', shard: result.shard })
      } else if (result.kind === 'absent') {
        setCompare({ kind: 'absent' })
      } else {
        setCompare({ kind: 'error', message: result.message })
      }
    })()
    return () => {
      cancelled = true
    }
  }, [compareId, dev])

  function chooseCompare(next: string): void {
    const value = next === '' ? null : next
    setCompareId(value)
    const url = new URL(window.location.href)
    if (value === null) url.searchParams.delete('compare')
    else url.searchParams.set('compare', value)
    window.history.replaceState(null, '', url)
  }

  if (state.kind === 'absent') return null
  if (state.kind === 'loading') {
    // Deliberately not `null`. This island is `client:visible`, and an element that renders
    // nothing has no box to intersect: the observer would never fire and the fetch would
    // never start. The placeholder is what makes the deferral work at all.
    return (
      <section
        id="benchmarks"
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
        id="benchmarks"
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
  const notes = observations(rows, summary)
  const series = chartSeries(rows)
  const lines = provenance(shard.measurement)

  const comparison =
    compare.kind === 'ready'
      ? buildComparison(shard, compare.shard, rows, instances, index)
      : null
  const compareNote =
    compare.kind === 'absent'
      ? `${solverLabel(compareId ?? '')} has no published shard, so there is nothing to compare.`
      : compare.kind === 'error'
        ? `The comparison could not be read (${compare.message}).`
        : compare.kind === 'loading'
          ? 'Loading the comparison…'
          : null

  const hasTour = rows.some((row) => row.tour !== null)
  const hasWall = rows.some((row) => row.wall !== null)

  const primaryFamily = solverById(shard.solver)?.family
  const comparedFamily = comparison
    ? solverById(comparison.solver)?.family
    : undefined
  const crossFamily =
    comparison !== null &&
    primaryFamily !== undefined &&
    comparedFamily !== undefined &&
    primaryFamily !== comparedFamily

  // Stated next to the numbers, because a comparison invites exactly the reading these
  // sentences prevent: that the two columns were measured under the same allowance.
  const compareSentence =
    comparison === null
      ? ''
      : comparison.shared === 0
        ? `${comparison.name} shares no instance and config with this solver, so there is nothing to compare. `
        : `Compared with ${comparison.name} on ${comparison.shared} shared instance${
            comparison.shared === 1 ? '' : 's'
          }. ${
            crossFamily
              ? `They sit in different families, so their gaps are not like-for-like: ${primaryFamily} ${budget?.seconds ?? '?'} s against ${comparedFamily} ${comparison.budget?.seconds ?? '?'} s. `
              : ''
          }`

  // One chart, whichever view is on: the same component draws one series or two, so the axis
  // rule has to hold for whichever set of points it is given.
  const chartData = comparison
    ? comparisonData(comparison, shard.solver)
    : singleSeriesData(series)
  const chartAxis = comparison ? comparison.series.axis : series.axis
  const chartAxisLabel = comparison
    ? comparison.series.axisLabel
    : series.axisLabel
  const chartOmitted = {
    noOptimum: comparison ? 0 : series.omitted.noOptimum,
    noResult: comparison ? 0 : series.omitted.noResult,
    comparator: comparison ? comparison.series.omitted[comparison.solver] : 0,
  }

  return (
    <section id="benchmarks" class="mt-10" aria-labelledby="benchmarks-heading">
      <div class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-2">
        <h2
          id="benchmarks-heading"
          class="text-lg font-semibold tracking-tight lg:text-xl"
        >
          Benchmarks
        </h2>
        <div class="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-xs">
          <label class="flex items-center gap-2 text-text-dim">
            compare with
            <select
              class="rounded border border-border bg-surface px-2 py-1 font-mono text-xs text-text"
              value={compareId ?? ''}
              onChange={(event) =>
                chooseCompare((event.target as HTMLSelectElement).value)
              }
            >
              <option value="">nobody</option>
              {COMPARE_OPTIONS.filter((option) => option.id !== id).map(
                (option) => (
                  <option key={option.id} value={option.id}>
                    {option.name}
                  </option>
                ),
              )}
            </select>
          </label>
          {/* Cross-origin `download` is ignored by browsers, so this opens the shard rather
              than promising a save that will not happen. */}
          <a
            class="text-accent no-underline hover:underline"
            href={algorithmShardUrl(id, dev)}
            target="_blank"
            rel="noopener"
            title="Open this solver's published shard (JSON)"
          >
            shard ↗
          </a>
        </div>
      </div>

      {/* The prose says what the numbers amount to; the numbers themselves are in the table
          below, including its summary row. Repeating aggregates here would give the reader two
          places to read the same figure from and one more place for them to disagree. */}
      <p class="mt-2 text-sm text-text-dim">
        {summary.instances} instance{summary.instances === 1 ? '' : 's'}{' '}
        measured under the{' '}
        {budget
          ? `${family} budget of ${budget.seconds} s${
              budget.agreed ? '' : ' (provisional)'
            }`
          : 'published budget, which could not be read'}
        .{' '}
        {!columns.gap &&
          'No instance here has a published optimum, so there is no gap to show. '}
        {notes.join(' ')} {compareSentence}
      </p>

      {compareNote && (
        <p class="mt-2 text-sm text-text-dim" role="status">
          {compareNote}
        </p>
      )}

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
            . The summary row reports the median of the per-instance medians;
            run counts are totals, and tour length is left blank there because
            it is not comparable between instances.
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
              {comparison && <Th right>Gap ({comparison.name})</Th>}
              {comparison && <Th right>Wall ({comparison.name})</Th>}
              <Th right>Runs</Th>
              <Th right>Lost runs</Th>
            </tr>
          </thead>
          <tbody>
            {comparison
              ? comparison.pairs.map((pair) => (
                  <Row
                    key={`${pair.dataset}/${pair.config}`}
                    row={pair.primary}
                    columns={columns}
                    hasTour={hasTour}
                    hasWall={hasWall}
                    multiConfig={summary.configs.length > 1}
                    comparing
                    compared={pair.comparator}
                  />
                ))
              : rows.map((row) => (
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
          <tfoot>
            {/* Two rows when comparing, each under its own columns: one merged row would put
                two solvers' medians in the same cell and invite reading them as one figure. */}
            <tr class="border-t-2 border-border font-medium">
              <td class="px-3 py-2">
                {comparison
                  ? solverLabel(shard.solver)
                  : `All ${summary.instances} instances`}
              </td>
              <td class="px-3 py-2 text-right font-mono">{DASH}</td>
              <td class="px-3 py-2 font-mono text-xs">{DASH}</td>
              {columns.optimal && (
                <td class="px-3 py-2 text-right font-mono">{DASH}</td>
              )}
              {columns.gap && (
                <td class="px-3 py-2 text-right font-mono">
                  {formatGapPct(summary.medianGapPct)}
                </td>
              )}
              {columns.gap && columns.spread && (
                <td class="px-3 py-2 text-right font-mono">
                  {formatGapPct(summary.medianGapBestPct)}
                </td>
              )}
              {columns.gap && columns.spread && (
                <td class="px-3 py-2 text-right font-mono">
                  {formatGapPct(summary.medianGapWorstPct)}
                </td>
              )}
              {hasTour && (
                <td class="px-3 py-2 text-right font-mono">{DASH}</td>
              )}
              {hasTour && columns.spread && (
                <td class="px-3 py-2 text-right font-mono">{DASH}</td>
              )}
              {hasWall && (
                <td class="px-3 py-2 text-right font-mono">
                  {formatSeconds(summary.medianWallS)}
                </td>
              )}
              {comparison && (
                <td class="px-3 py-2 text-right font-mono">{DASH}</td>
              )}
              {comparison && (
                <td class="px-3 py-2 text-right font-mono">{DASH}</td>
              )}
              <td class="px-3 py-2 text-right font-mono">
                {summary.totalRuns}
              </td>
              <td class="px-3 py-2 text-right font-mono">
                {lostRunsNote(summary.totalTimeouts, summary.totalErrors)}
              </td>
            </tr>
            {comparison && (
              <tr class="border-t border-border font-medium">
                <td class="px-3 py-2">{comparison.name}</td>
                <td class="px-3 py-2 text-right font-mono">{DASH}</td>
                <td class="px-3 py-2 font-mono text-xs">{DASH}</td>
                {columns.optimal && (
                  <td class="px-3 py-2 text-right font-mono">{DASH}</td>
                )}
                {columns.gap && (
                  <td class="px-3 py-2 text-right font-mono">{DASH}</td>
                )}
                {columns.gap && columns.spread && (
                  <td class="px-3 py-2 text-right font-mono">{DASH}</td>
                )}
                {columns.gap && columns.spread && (
                  <td class="px-3 py-2 text-right font-mono">{DASH}</td>
                )}
                {hasTour && (
                  <td class="px-3 py-2 text-right font-mono">{DASH}</td>
                )}
                {hasTour && columns.spread && (
                  <td class="px-3 py-2 text-right font-mono">{DASH}</td>
                )}
                {hasWall && (
                  <td class="px-3 py-2 text-right font-mono">{DASH}</td>
                )}
                <td class="px-3 py-2 text-right font-mono">
                  {summaryGapWithSpread(comparison.summary)}
                </td>
                <td class="px-3 py-2 text-right font-mono">
                  {formatSeconds(comparison.summary.medianWallS)}
                </td>
                <td class="px-3 py-2 text-right font-mono">
                  {comparison.summary.totalRuns}
                </td>
                <td class="px-3 py-2 text-right font-mono">
                  {lostRunsNote(
                    comparison.summary.totalTimeouts,
                    comparison.summary.totalErrors,
                  )}
                </td>
              </tr>
            )}
          </tfoot>
        </table>
      </div>

      {chartData.length > 0 && (
        <figure class="mt-6 rounded-lg border border-border bg-surface p-4 lg:p-6">
          <Scatter
            data={chartData}
            axis={chartAxis}
            axisLabel={chartAxisLabel}
          />
          <div class="mt-4 flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-border pt-3 font-mono text-xs">
            <Legend colour={POINT_COLOR} label={solverLabel(shard.solver)} />
            {comparison && (
              <Legend colour={COMPARED_COLOR} label={comparison.name} />
            )}
          </div>
          <figcaption class="mt-3 font-mono text-xs text-text-dim">
            {chartAxis === 'gap'
              ? comparison
                ? 'Gap against known optimum, one point per instance per solver.'
                : `Gap against known optimum, one point per instance. ${
                    chartOmitted.noOptimum > 0
                      ? `${chartOmitted.noOptimum} point${
                          chartOmitted.noOptimum === 1 ? '' : 's'
                        } without a published optimum ${chartOmitted.noOptimum === 1 ? 'is' : 'are'} not plotted — a gap and a tour length are not the same axis.`
                      : ''
                  }`
              : 'Tour length against instance size, because none of these instances has a published optimum. Lower is better for a given size, but the values are not comparable across instances.'}
            {chartOmitted.noResult > 0
              ? ` ${chartOmitted.noResult} cell${
                  chartOmitted.noResult === 1 ? '' : 's'
                } had no completed run and ${chartOmitted.noResult === 1 ? 'is' : 'are'} not plotted.`
              : ''}
            {comparison && chartOmitted.comparator > 0
              ? ` ${comparison.name} has no value to plot for ${chartOmitted.comparator} of these instance${
                  chartOmitted.comparator === 1 ? '' : 's'
                }.`
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
        {comparison && (
          <dl class="mt-3 grid grid-cols-1 gap-x-6 gap-y-1 text-xs sm:grid-cols-2">
            <div class="flex gap-2 sm:col-span-2">
              <dt class="min-w-24 text-text-dim">{comparison.name}</dt>
              <dd class="m-0 text-text-dim">
                measured from its own build, not this solver's
              </dd>
            </div>
            {comparison.lines.map((line) => (
              <div key={line.label} class="flex gap-2">
                <dt class="min-w-24 text-text-dim">{line.label}</dt>
                <dd class="m-0 font-mono">{line.value}</dd>
              </div>
            ))}
          </dl>
        )}
        <p class="mt-2 text-xs text-text-dim">
          These numbers were measured from each solver's own build. Two
          algorithms may legitimately show different commits.
        </p>
        {/* Cross-origin `download` is ignored, so the header link opens the shard rather than
            promising a save. This line is the rest of the tree: the manifest and the contract. */}
        <p class="mt-2 font-mono text-xs text-text-dim">
          data:{' '}
          <a
            class="text-accent no-underline hover:underline"
            href={indexUrl(dev)}
            target="_blank"
            rel="noopener"
          >
            index.json ↗
          </a>{' '}
          (the manifest, with the per-family budgets) ·{' '}
          <a
            class="text-accent no-underline hover:underline"
            href={SCHEMA_URL}
            target="_blank"
            rel="noopener"
          >
            contract ↗
          </a>{' '}
          (docs/benchmarks/schema.md)
        </p>
      </div>
    </section>
  )
}

function Legend({ colour, label }: { colour: string; label: string }) {
  return (
    <span class="inline-flex items-center gap-2">
      <svg class="inline-block h-2 w-2" viewBox="0 0 8 8" aria-hidden="true">
        <circle cx="4" cy="4" r="4" fill={colour} />
      </svg>
      <span class="text-text">{label}</span>
    </span>
  )
}

function Th({
  children,
  right = false,
}: {
  children: ComponentChildren
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
  comparing = false,
  compared = null,
}: {
  row: BenchmarkRow
  columns: { gap: boolean; optimal: boolean; spread: boolean }
  hasTour: boolean
  hasWall: boolean
  multiConfig: boolean
  comparing?: boolean
  /** The comparator's cell for this instance and config, or null when it has none. */
  compared?: BenchmarkRow | null
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
      {comparing && (
        <td class="px-3 py-2 text-right font-mono">
          {compared === null ? 'not measured' : gapWithSpread(compared)}
        </td>
      )}
      {comparing && (
        <td class="px-3 py-2 text-right font-mono">
          {compared === null
            ? 'not measured'
            : formatSeconds(compared.wall?.median ?? null)}
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
 * The y value is whatever the caller chose — never both, and never one series of each. Points
 * carry their own colour so the same component draws one solver or two.
 */
function Scatter({
  data,
  axis,
  axisLabel,
}: {
  data: ChartDatum[]
  axis: ChartAxis
  axisLabel: string
}) {
  const W = 720
  const H = 340
  const PAD = { left: 64, right: 28, top: 20, bottom: 44 }
  const plotW = W - PAD.left - PAD.right
  const plotH = H - PAD.top - PAD.bottom

  const xs = data.map((datum) => datum.x)
  const ys = data.map((datum) => datum.y)
  const xMin = Math.min(...xs)
  const xMax = Math.max(...xs)
  // Both axes start at zero: an axis that starts near the minimum would exaggerate the
  // differences between solvers on the same instance.
  const yMax = Math.max(...ys)
  const yTicks = niceTicks(yMax)
  // The top tick bounds the plot, so the labels stay round and the tallest point stays inside.
  const yTop = yTicks[yTicks.length - 1]

  const sx = (x: number) =>
    xMax === xMin
      ? PAD.left + plotW / 2
      : PAD.left + ((x - xMin) / (xMax - xMin)) * plotW
  const sy = (y: number) => PAD.top + (1 - y / yTop) * plotH

  const xTicks = [0, 0.5, 1].map((f) => xMin + f * (xMax - xMin))
  const yFormat =
    axis === 'gap'
      ? (v: number) => `${v.toFixed(1)}%`
      : (v: number) => v.toFixed(0)

  // Placement is shared with the landing chart so the two cannot drift. A comparison labels
  // only the primary's points, since both series sit on the same instances.
  const placements = placeLabels(
    data
      .filter((datum) => datum.label !== undefined)
      .map((datum) => ({
        text: datum.label!,
        x: sx(datum.x),
        y: sy(datum.y),
      })),
    { width: W, height: H },
  )

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      class="h-auto w-full"
      role="img"
      aria-label={axisLabel}
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
        {axis === 'gap' ? 'gap vs optimal →' : 'tour length →'}
      </text>
      {data.map((datum) => (
        <circle
          key={datum.key}
          cx={sx(datum.x)}
          cy={sy(datum.y)}
          r="4"
          fill={datum.colour}
        >
          <title>
            {datum.tooltip}
            {` · ${datum.x} cities`}
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

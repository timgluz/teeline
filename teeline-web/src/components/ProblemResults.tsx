// Published leaderboard for /problems/[id]/: how every measured solver does on this instance.
//
// A client island for the same reason as the algorithm page's benchmark section: the shard is
// fetched at runtime so republishing does not need a redeploy, and the build must not fail when
// R2 is unreachable (decision 9). The page's own `optimalCost` arrives as a prop from the problem
// doc, so a disagreement between what the page claims and what the shard ranked on can be stated
// rather than silently shown twice.

import type { ComponentChildren } from 'preact'
import { useEffect, useState } from 'preact/hooks'

import { niceTicks } from '../lib/chart-layout'
import {
  FAMILY_COLORS,
  FAMILY_LABELS,
  UNKNOWN_FAMILY_COLOR,
} from '../lib/family-palette'
import { solverHref, solverLabel } from '../lib/solver-label'
import {
  DASH,
  budgetFor,
  fetchBenchmarkIndex,
  fetchProblemShard,
  formatGapPct,
  formatSeconds,
  formatTour,
  indexUrl,
  leaderboardAxis,
  leaderboardFamilies,
  leaderboardRows,
  lostRunsNote,
  problemShardUrl,
  type BenchmarkIndex,
  type BudgetEntry,
  type ChartAxis,
  type LeaderboardRow,
  type ProblemShard,
  type SolverFamily,
} from '../lib/benchmark'

export interface ProblemResultsProps {
  /** Dataset id, which is also the shard name on R2. */
  id: string
  dev: boolean
  /** The optimum the page itself states, for the consistency check against the shard's. */
  optimalCost: number | null
}

type State =
  | { kind: 'loading' }
  /** No shard published for this instance: a normal state, and it renders nothing. */
  | { kind: 'absent' }
  | { kind: 'error'; message: string }
  | { kind: 'ready'; shard: ProblemShard; index: BenchmarkIndex | null }

const AXIS_COLOR = '#232327'
const LABEL_COLOR = '#8A8A92'
const TEXT_COLOR = '#F5F5F6'
const SCHEMA_URL =
  'https://github.com/timgluz/teeline/blob/master/docs/benchmarks/schema.md'

export default function ProblemResults({
  id,
  dev,
  optimalCost,
}: ProblemResultsProps) {
  const [state, setState] = useState<State>({ kind: 'loading' })

  useEffect(() => {
    let cancelled = false
    void (async () => {
      // The manifest is fetched for the per-family budgets: a leaderboard mixes families by
      // definition, and a gap without its allowance is not comparable (decision 4).
      const [result, index] = await Promise.all([
        fetchProblemShard(id, dev),
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

  if (state.kind !== 'ready') {
    // Deliberately not `null`: a `client:visible` island with no box never intersects, so it
    // would never hydrate and never fetch.
    return (
      <section
        id="results"
        class="mt-10"
        aria-busy="true"
        aria-labelledby="results-heading"
      >
        <h2
          id="results-heading"
          class="text-lg font-semibold tracking-tight lg:text-xl"
        >
          Published results
        </h2>
        <p class="mt-2 text-sm text-text-dim">
          {state.kind === 'error'
            ? `The published leaderboard could not be read (${state.message}).`
            : 'Loading published results…'}
        </p>
      </section>
    )
  }

  const { shard, index } = state

  // A 200 does not mean "published": the publisher omits retired objects from the manifest without
  // deleting them from R2, so a shard can keep answering with pre-fix numbers (a null optimum, one
  // or two solvers) and nothing in the response says so. The manifest is what published means, so
  // an unlisted dataset renders nothing — the same as a 404. With no manifest to check against, the
  // board is shown but says its currency could not be confirmed.
  const listed =
    index === null ? null : index.problems.some((problem) => problem.id === id)
  if (listed === false) return null

  const rows = leaderboardRows(shard)
  // A shard with no rows of any kind is the same fact as no shard.
  if (rows.length === 0) return null

  const axis = leaderboardAxis(shard)
  const families = leaderboardFamilies(rows)
  const budgets = new Map<SolverFamily, BudgetEntry | null>(
    families.map((family) => [family, budgetFor(index, family)]),
  )
  const rankable = rows.filter((row) => row.rank !== null)
  const rowsWithLostRuns = rows.filter(
    (row) => row.noResult || row.timeouts > 0 || row.errors > 0,
  )
  const solverCount = new Set(rows.map((row) => row.solver)).size
  const configCount = new Set(rows.map((row) => row.config)).size

  // The page states an optimum from its own frontmatter; the shard ranked on its own copy. They
  // come from the same docs, but a republish between the two can separate them, and a page that
  // showed both without comment would be contradicting itself.
  const mismatch =
    optimalCost !== shard.optimalCost &&
    !(optimalCost === null && shard.optimalCost === null)

  return (
    <section id="results" class="mt-10" aria-labelledby="results-heading">
      <h2
        id="results-heading"
        class="text-lg font-semibold tracking-tight lg:text-xl"
      >
        Published results
      </h2>

      <p class="mt-2 text-sm text-text-dim">
        {solverCount} solver{solverCount === 1 ? '' : 's'} with a completed run
        on this instance
        {configCount > 1 ? `, in ${rows.length} configurations` : ''}.{' '}
        {axis === 'gap'
          ? 'Ranked on median gap from the known optimum, best of the runs beside it.'
          : 'No optimum is published for this instance, so the board ranks on tour cost and shows no gap.'}
      </p>

      {/* A leaderboard mixes families, and the allowances differ, so both are stated rather than
          leaving the reader to assume the ranking is like-for-like. */}
      <p class="mt-2 font-mono text-xs text-text-dim">
        {families
          .map((family) => {
            const budget = budgets.get(family)
            return `${FAMILY_LABELS[family]} ${
              budget
                ? `${budget.seconds} s${budget.agreed ? '' : ' (provisional)'}`
                : 'budget not published'
            }`
          })
          .join(' · ')}
        {' — cross-family gaps are not like-for-like'}
      </p>

      {mismatch && (
        <p class="mt-2 text-sm text-text-dim">
          This page states an optimal tour of{' '}
          {optimalCost === null ? 'unknown' : optimalCost.toLocaleString()}; the
          published board was ranked against{' '}
          {shard.optimalCost === null
            ? 'no optimum'
            : shard.optimalCost.toLocaleString()}
          . The board is shown as published.
        </p>
      )}

      <div class="mt-4 overflow-x-auto rounded-lg border border-border bg-surface">
        <table class="w-full border-collapse text-sm">
          <caption class="sr-only">
            Published leaderboard for this instance, ranked on{' '}
            {axis === 'gap' ? 'median gap' : 'median tour cost'}
            {configCount > 1
              ? '; rows are (solver, configuration) pairs, so one solver may appear more than once'
              : ''}
          </caption>
          <thead>
            <tr class="border-b border-border">
              <Th right>#</Th>
              <Th>Solver</Th>
              {configCount > 1 && <Th>Config</Th>}
              <Th>Family</Th>
              {axis === 'gap' ? (
                <>
                  <Th right>Gap (median)</Th>
                  <Th right>Gap (best)</Th>
                </>
              ) : (
                <Th right>Tour (median)</Th>
              )}
              <Th right>Wall (median)</Th>
              <Th right>Runs</Th>
              <Th right>Lost runs</Th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <LeaderboardTableRow
                key={`${row.solver}/${row.config}`}
                row={row}
                axis={axis}
                showConfig={configCount > 1}
              />
            ))}
          </tbody>
        </table>
      </div>

      {rankable.length > 0 && (
        <figure class="mt-6 rounded-lg border border-border bg-surface p-4 lg:p-6">
          <FamilyBars rows={rankable} axis={axis} />
          <div class="mt-4 flex flex-wrap items-center gap-x-6 gap-y-2 border-t border-border pt-4">
            {families.map((family) => {
              const budget = budgets.get(family)
              return (
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
                  <span class="text-text">
                    {FAMILY_LABELS[family]}
                    {budget ? ` · ${budget.seconds} s` : ''}
                  </span>
                </span>
              )
            })}
          </div>
          <figcaption class="mt-3 font-mono text-xs text-text-dim">
            {axis === 'gap'
              ? 'Median gap from the known optimum, one bar per configuration. Shorter is better.'
              : 'Median tour cost, one bar per configuration, because no optimum is published for this instance — so this compares solvers with each other, not with optimal.'}
            {rowsWithLostRuns.length > 0
              ? ` ${rowsWithLostRuns.length} row${
                  rowsWithLostRuns.length === 1 ? '' : 's'
                } lost runs to the budget; a row with no completed run is not plotted.`
              : ''}
          </figcaption>
        </figure>
      )}

      <div class="mt-6 border-t border-border pt-4">
        <h3 class="font-mono text-xs uppercase tracking-wider text-text-dim">
          Provenance
        </h3>
        {/* The shard is an aggregate: problem shards carry no measurement block, only the
            per-solver algorithm shards do. Each row links to where its own build is stated. */}
        <p class="mt-2 text-xs text-text-dim">
          Each row was measured from that solver's own build, and two rows here
          may legitimately come from different commits — follow a solver to see
          which. Leaderboards are aggregates and carry no provenance of their
          own.
        </p>
        {/* The publisher drops (solver, config) cells with no completed run, so an absent row is
            ambiguous and the page must not let the board read as a full roster. */}
        <p class="mt-2 text-xs text-text-dim">
          A solver whose every run hit the budget has no row here at all: the
          publisher omits cells with no completed run, so this board cannot tell
          that apart from a solver never measured on this instance.
        </p>
        {listed === null && (
          <p class="mt-2 text-xs text-text-dim" role="status">
            The published manifest could not be read, so this board being
            current could not be confirmed.
          </p>
        )}
        <p class="mt-2 font-mono text-xs text-text-dim">
          data:{' '}
          <a
            class="text-accent no-underline hover:underline"
            href={problemShardUrl(id, dev)}
            target="_blank"
            rel="noopener"
          >
            problems/{id}.json ↗
          </a>{' '}
          ·{' '}
          <a
            class="text-accent no-underline hover:underline"
            href={indexUrl(dev)}
            target="_blank"
            rel="noopener"
          >
            index.json ↗
          </a>{' '}
          ·{' '}
          <a
            class="text-accent no-underline hover:underline"
            href={SCHEMA_URL}
            target="_blank"
            rel="noopener"
          >
            contract ↗
          </a>
        </p>
      </div>
    </section>
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

function LeaderboardTableRow({
  row,
  axis,
  showConfig,
}: {
  row: LeaderboardRow
  axis: ChartAxis
  showConfig: boolean
}) {
  const href = solverHref(row.solver)
  const name = solverLabel(row.solver)
  // A row with no completed run has nothing to rank, and says so rather than showing a zero.
  const unranked = row.rank === null

  return (
    <tr class="border-b border-border last:border-b-0">
      <td class="px-3 py-2 text-right font-mono">
        {unranked ? DASH : row.rank}
      </td>
      <td class="px-3 py-2">
        {href ? (
          <a href={href} class="text-accent no-underline hover:underline">
            {name}
          </a>
        ) : (
          <span>{name}</span>
        )}
      </td>
      {showConfig && <td class="px-3 py-2 font-mono text-xs">{row.config}</td>}
      <td class="px-3 py-2 font-mono text-xs">
        {row.family ? FAMILY_LABELS[row.family] : DASH}
      </td>
      {axis === 'gap' ? (
        <>
          <td class="px-3 py-2 text-right font-mono">
            {formatGapPct(row.gap?.median ?? null)}
          </td>
          {/* The best of the runs, so a solver whose tours vary is not summarised as one number. */}
          <td class="px-3 py-2 text-right font-mono">
            {formatGapPct(row.gap?.best ?? null)}
          </td>
        </>
      ) : (
        <td class="px-3 py-2 text-right font-mono">
          {formatTour(row.tour?.median ?? null)}
        </td>
      )}
      <td class="px-3 py-2 text-right font-mono">
        {formatSeconds(row.wall?.median ?? null)}
      </td>
      <td class="px-3 py-2 text-right font-mono">{row.runs}</td>
      <td class="px-3 py-2 text-right font-mono">
        {lostRunsNote(row.timeouts, row.errors)}
      </td>
    </tr>
  )
}

/**
 * The ranking as horizontal bars: the same numbers as the table, chosen for shape — a bar chart
 * answers "who is ahead" at a glance, where a table answers "what did each score".
 */
function FamilyBars({
  rows,
  axis,
}: {
  rows: LeaderboardRow[]
  axis: ChartAxis
}) {
  const W = 720
  const ROW_H = 24
  // 156px of gutter fits the longest label in the set ("Stochastic Hill Climbing", 24 chars at
  // the monospace advance); anything narrower clips the name off the left edge of the viewBox.
  const PAD = { left: 156, right: 56, top: 26, bottom: 30 }
  const plotW = W - PAD.left - PAD.right
  const H = PAD.top + rows.length * ROW_H + PAD.bottom

  const value = (row: LeaderboardRow): number =>
    axis === 'gap' ? (row.gap?.median ?? 0) : (row.tour?.median ?? 0)

  const ticks = niceTicks(Math.max(...rows.map(value)))
  const top = ticks[ticks.length - 1]
  const sx = (v: number) => PAD.left + (v / top) * plotW
  const format = axis === 'gap' ? formatGapPct : formatTour

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      class="h-auto w-full"
      role="img"
      aria-label={`Median ${axis === 'gap' ? 'gap' : 'tour cost'} by solver`}
    >
      {ticks.map((tick) => (
        <g key={`t${tick}`}>
          <line
            x1={sx(tick)}
            y1={PAD.top - 6}
            x2={sx(tick)}
            y2={H - PAD.bottom + 4}
            stroke={AXIS_COLOR}
            stroke-width="1"
          />
          <text
            x={sx(tick)}
            y={H - 10}
            text-anchor="middle"
            font-size="10"
            fill={LABEL_COLOR}
            font-family="JetBrains Mono, monospace"
          >
            {format(tick)}
          </text>
        </g>
      ))}
      {rows.map((row, index) => {
        const y = PAD.top + index * ROW_H
        const colour = row.family
          ? FAMILY_COLORS[row.family]
          : UNKNOWN_FAMILY_COLOR
        return (
          <g key={`${row.solver}/${row.config}`}>
            <text
              x={PAD.left - 8}
              y={y + ROW_H / 2 + 3}
              text-anchor="end"
              font-size="10"
              fill={TEXT_COLOR}
              font-family="JetBrains Mono, monospace"
            >
              {solverLabel(row.solver)}
            </text>
            <rect
              x={PAD.left}
              y={y + 4}
              width={Math.max(sx(value(row)) - PAD.left, 1)}
              height={ROW_H - 9}
              fill={colour}
              opacity="0.9"
            />
            <text
              x={sx(value(row)) + 6}
              y={y + ROW_H / 2 + 3}
              font-size="10"
              fill={LABEL_COLOR}
              font-family="JetBrains Mono, monospace"
            >
              {format(value(row))}
            </text>
          </g>
        )
      })}
    </svg>
  )
}

// Read layer for the published benchmark contract (docs/benchmarks/schema.md, v2).
//
// Pure and injectable: no DOM, no global fetch at module scope. That is what lets the rules
// that matter be tested without a server — an unknown optimum is not a zero gap, a timeout is
// not a bad result, and a gap never appears without the budget it was measured under.
//
// Wire names are snake_case and are normalised at this boundary, as solver-index.ts does for
// bench/solvers.json, so nothing downstream has to know the wire spelling.

import { isDeterministic, solverById } from './solver-index'
import type { SolverFamily } from './solver-index'

/** The only contract version this module understands. See parseAlgorithmShard. */
export const SCHEMA_VERSION = 2

export type { SolverFamily }

/** Median/best/worst are always present; `mean` is omitted when a cell has a single run. */
export interface Stat {
  median: number
  best: number
  worst: number
  mean?: number
}

/** One `(instance, config)` cell. Counts are always present; summaries may be absent. */
export interface ShardCell {
  runs: number
  timeouts: number
  errors: number
  gapPct: Stat | null
  tourCost: Stat | null
  wallS: Stat | null
  peakRssKb: Stat | null
  optimalCost: number | null
}

export interface ShardInstance {
  cities: number | null
  optimalCost: number | null
  byConfig: Record<string, ShardCell>
}

export interface Measurement {
  gitCommit: string
  gitCommitSource: string
  /** Tri-state: null means the run log did not record it, which is not the same as "clean". */
  dirty: boolean | null
  teelineVersion: string | null
  measuredAt: string | null
  environment: {
    os: string | null
    build: string | null
    rustVersion: string | null
  } | null
}

export interface AlgorithmShard {
  solver: string
  configs: string[]
  measurement: Measurement
  results: Record<string, ShardInstance>
}

export interface BudgetEntry {
  seconds: number
  agreed: boolean
}

export interface BenchmarkIndex {
  generatedAt: string
  tier: string | null
  budgets: Record<string, BudgetEntry>
  solvers: { id: string; family: SolverFamily }[]
  problems: { id: string; cities: number | null }[]
}

/** Build-time metadata for an instance, from the `problems` collection. */
export interface InstanceMeta {
  name: string
  edgeWeightType: string
  cities: number | null
}

// ---------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------

/**
 * The slice of `fetch` this module uses, so tests can inject a stub instead of a server.
 */
export type FetchLike = (
  url: string,
) => Promise<{ status: number; ok: boolean; json(): Promise<unknown> }>

export type ShardOutcome<T> =
  | { kind: 'ok'; shard: T }
  /** HTTP 404 — an unpublished page is a normal state, not a failure. */
  | { kind: 'absent' }
  | { kind: 'error'; status: number | null; message: string }

export type ShardResult = ShardOutcome<AlgorithmShard>
export type ProblemResult = ShardOutcome<ProblemShard>

export function benchmarkBaseUrl(dev: boolean): string {
  // Dev goes through the Vite proxy (astro.config.mjs), production hits R2 directly. Without
  // the proxy the section silently renders nothing locally and reads as a bug in the component.
  return dev ? '/bench/v2' : 'https://static.tspsolver.com/bench/v2'
}

export function algorithmShardUrl(id: string, dev: boolean): string {
  return `${benchmarkBaseUrl(dev)}/algorithms/${encodeURIComponent(id)}.json`
}

export function problemShardUrl(id: string, dev: boolean): string {
  return `${benchmarkBaseUrl(dev)}/problems/${encodeURIComponent(id)}.json`
}

export function indexUrl(dev: boolean): string {
  return `${benchmarkBaseUrl(dev)}/index.json`
}

/**
 * Fetch and parse one shard.
 *
 * Only a 404 is treated as absence. Any other failure — a 500, a DNS failure, a shard this
 * version cannot parse — is an error the caller is expected to surface, because swallowing it
 * would leave a page that looks like "no results yet" when the data is present but broken.
 */
async function fetchShard<T>(
  url: string,
  parse: (raw: unknown) => T,
  fetchImpl: FetchLike,
): Promise<ShardOutcome<T>> {
  try {
    const response = await fetchImpl(url)
    if (response.status === 404) return { kind: 'absent' }
    if (!response.ok) {
      return {
        kind: 'error',
        status: response.status,
        message: `HTTP ${response.status}`,
      }
    }
    return { kind: 'ok', shard: parse(await response.json()) }
  } catch (err) {
    return {
      kind: 'error',
      status: null,
      message: err instanceof Error ? err.message : String(err),
    }
  }
}

export function fetchAlgorithmShard(
  id: string,
  dev: boolean,
  fetchImpl: FetchLike = fetch,
): Promise<ShardResult> {
  return fetchShard(algorithmShardUrl(id, dev), parseAlgorithmShard, fetchImpl)
}

export function fetchProblemShard(
  id: string,
  dev: boolean,
  fetchImpl: FetchLike = fetch,
): Promise<ProblemResult> {
  return fetchShard(problemShardUrl(id, dev), parseProblemShard, fetchImpl)
}

/**
 * Fetch the manifest, which is the only published source of the per-family budgets.
 *
 * Returns null rather than throwing: the manifest decides whether gaps may be shown, not
 * whether the page renders at all. The caller hides the gap column and says why.
 */
export async function fetchBenchmarkIndex(
  dev: boolean,
  fetchImpl: FetchLike = fetch,
): Promise<BenchmarkIndex | null> {
  try {
    const response = await fetchImpl(indexUrl(dev))
    if (!response.ok) return null
    return parseBenchmarkIndex(await response.json())
  } catch {
    return null
  }
}

/** The budget covering a solver's family, or null when the manifest does not state one. */
export function budgetFor(
  index: BenchmarkIndex | null,
  family: SolverFamily,
): BudgetEntry | null {
  return index?.budgets[family] ?? null
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

function asRecord(value: unknown, what: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error(`${what} is not an object`)
  }
  return value as Record<string, unknown>
}

function asString(value: unknown, what: string): string {
  if (typeof value !== 'string' || value === '') {
    throw new Error(`${what} is not a non-empty string`)
  }
  return value
}

function asNumber(value: unknown, what: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    throw new Error(`${what} is not a finite number`)
  }
  return value
}

function optionalNumber(value: unknown, what: string): number | null {
  if (value === null || value === undefined) return null
  return asNumber(value, what)
}

function optionalString(value: unknown): string | null {
  return typeof value === 'string' && value !== '' ? value : null
}

function stringArray(value: unknown, what: string): string[] {
  if (!Array.isArray(value)) throw new Error(`${what} is not an array`)
  return value.map((entry, i) => asString(entry, `${what}[${i}]`))
}

function asStat(value: unknown, what: string): Stat | null {
  // Absent and null are the same fact: the publisher had no successful run to summarise.
  if (value === null || value === undefined) return null
  const obj = asRecord(value, what)
  const stat: Stat = {
    median: asNumber(obj.median, `${what}.median`),
    best: asNumber(obj.best, `${what}.best`),
    worst: asNumber(obj.worst, `${what}.worst`),
  }
  const mean = optionalNumber(obj.mean, `${what}.mean`)
  if (mean !== null) stat.mean = mean
  return stat
}

function parseEnvironment(raw: unknown): Measurement['environment'] {
  if (raw === null || raw === undefined) return null
  const env = asRecord(raw, 'measurement.environment')
  return {
    os: optionalString(env.os),
    build: optionalString(env.build),
    rustVersion: optionalString(env.rust_version),
  }
}

function parseMeasurement(raw: unknown): Measurement {
  const m = asRecord(raw, 'measurement')
  return {
    // Required by the publisher: a measurement that cannot name the build is not provenance.
    gitCommit: asString(m.git_commit, 'measurement.git_commit'),
    gitCommitSource: optionalString(m.git_commit_source) ?? 'unknown',
    dirty: typeof m.dirty === 'boolean' ? m.dirty : null,
    teelineVersion: optionalString(m.teeline_version),
    measuredAt: optionalString(m.measured_at),
    environment: parseEnvironment(m.environment),
  }
}

function parseCell(raw: unknown, what: string): ShardCell {
  const cell = asRecord(raw, what)
  return {
    runs: asNumber(cell.runs, `${what}.runs`),
    timeouts: asNumber(cell.timeouts, `${what}.timeouts`),
    errors: asNumber(cell.errors, `${what}.errors`),
    gapPct: asStat(cell.gap_pct, `${what}.gap_pct`),
    tourCost: asStat(cell.tour_cost, `${what}.tour_cost`),
    wallS: asStat(cell.wall_s, `${what}.wall_s`),
    peakRssKb: asStat(cell.peak_rss_kb, `${what}.peak_rss_kb`),
    optimalCost: optionalNumber(cell.optimal_cost, `${what}.optimal_cost`),
  }
}

function parseResults(raw: unknown): Record<string, ShardInstance> {
  const results = asRecord(raw, 'results')
  const out: Record<string, ShardInstance> = {}
  for (const [dataset, value] of Object.entries(results)) {
    const instance = asRecord(value, `results.${dataset}`)
    const byConfigRaw = asRecord(
      instance.by_config,
      `results.${dataset}.by_config`,
    )
    const byConfig: Record<string, ShardCell> = {}
    for (const [config, cell] of Object.entries(byConfigRaw)) {
      byConfig[config] = parseCell(
        cell,
        `results.${dataset}.by_config.${config}`,
      )
    }
    out[dataset] = {
      // Null when the dataset has no problem doc: the shard still describes a real run, so
      // that is a row without a size rather than a malformed shard.
      cities: optionalNumber(instance.cities, `results.${dataset}.cities`),
      optimalCost: optionalNumber(
        instance.optimal_cost,
        `results.${dataset}.optimal_cost`,
      ),
      byConfig,
    }
  }
  return out
}

/**
 * Validate a shard and normalise its field names.
 *
 * A version this module does not know is refused rather than read best-effort: v3 rendered by
 * v2 rules is exactly the plausible number carrying a false claim that the schema's version
 * prefix exists to prevent.
 */
export function parseAlgorithmShard(raw: unknown): AlgorithmShard {
  const obj = asRecord(raw, 'algorithm shard')
  if (obj.schema_version !== SCHEMA_VERSION) {
    throw new Error(
      `unsupported benchmark schema_version ${JSON.stringify(
        obj.schema_version,
      )}; this page reads v${SCHEMA_VERSION}`,
    )
  }
  return {
    solver: asString(obj.solver, 'solver'),
    configs: stringArray(obj.configs, 'configs'),
    measurement: parseMeasurement(obj.measurement),
    results: parseResults(obj.results),
  }
}

export function parseBenchmarkIndex(raw: unknown): BenchmarkIndex {
  const obj = asRecord(raw, 'benchmark index')
  if (obj.schema_version !== SCHEMA_VERSION) {
    throw new Error(
      `unsupported benchmark index schema_version ${JSON.stringify(
        obj.schema_version,
      )}; this page reads v${SCHEMA_VERSION}`,
    )
  }
  const budgetsRaw = asRecord(obj.budgets, 'budgets')
  const budgets: Record<string, BudgetEntry> = {}
  for (const [family, value] of Object.entries(budgetsRaw)) {
    const entry = asRecord(value, `budgets.${family}`)
    budgets[family] = {
      seconds: asNumber(entry.seconds, `budgets.${family}.seconds`),
      agreed: entry.agreed === true,
    }
  }
  const solvers = Array.isArray(obj.solvers)
    ? obj.solvers.map((value, i) => {
        const s = asRecord(value, `solvers[${i}]`)
        return {
          id: asString(s.id, `solvers[${i}].id`),
          family: asString(s.family, `solvers[${i}].family`) as SolverFamily,
        }
      })
    : []
  const problems = Array.isArray(obj.problems)
    ? obj.problems.map((value, i) => {
        const p = asRecord(value, `problems[${i}]`)
        return {
          id: asString(p.id, `problems[${i}].id`),
          cities: optionalNumber(p.cities, `problems[${i}].cities`),
        }
      })
    : []
  return {
    generatedAt: optionalString(obj.generated_at) ?? '',
    tier: optionalString(obj.tier),
    budgets,
    solvers,
    problems,
  }
}

/**
 * One instance, every solver measured on it — the landing chart's shape, and the inverse of
 * an algorithm shard.
 */
export interface LeaderboardEntry {
  solver: string
  config: string
  runs: number
  timeouts: number
  errors: number
  gapPct: Stat | null
  tourCost: Stat | null
  wallS: Stat | null
  peakRssKb: Stat | null
  optimalCost: number | null
}

export interface ProblemShard {
  dataset: string
  cities: number | null
  optimalCost: number | null
  leaderboard: LeaderboardEntry[]
}

function parseLeaderboard(raw: unknown): LeaderboardEntry[] {
  if (!Array.isArray(raw)) throw new Error('leaderboard is not an array')
  return raw.map((value, i) => {
    const entry = asRecord(value, `leaderboard[${i}]`)
    return {
      // A solver may appear more than once (branch_bound publishes `budget` and `exact`):
      // these are (solver, config) rows, not solvers, and collapsing them would present a
      // beam search and an exact run as one entry.
      solver: asString(entry.solver, `leaderboard[${i}].solver`),
      config: optionalString(entry.config) ?? 'default',
      runs: asNumber(entry.runs, `leaderboard[${i}].runs`),
      timeouts: asNumber(entry.timeouts, `leaderboard[${i}].timeouts`),
      errors: asNumber(entry.errors, `leaderboard[${i}].errors`),
      gapPct: asStat(entry.gap_pct, `leaderboard[${i}].gap_pct`),
      tourCost: asStat(entry.tour_cost, `leaderboard[${i}].tour_cost`),
      wallS: asStat(entry.wall_s, `leaderboard[${i}].wall_s`),
      peakRssKb: asStat(entry.peak_rss_kb, `leaderboard[${i}].peak_rss_kb`),
      optimalCost: optionalNumber(
        entry.optimal_cost,
        `leaderboard[${i}].optimal_cost`,
      ),
    }
  })
}

export function parseProblemShard(raw: unknown): ProblemShard {
  const obj = asRecord(raw, 'problem shard')
  if (obj.schema_version !== SCHEMA_VERSION) {
    throw new Error(
      `unsupported benchmark schema_version ${JSON.stringify(
        obj.schema_version,
      )}; this page reads v${SCHEMA_VERSION}`,
    )
  }
  return {
    dataset: asString(obj.dataset, 'dataset'),
    // Null when the dataset has no problem doc, not a malformed shard.
    cities: optionalNumber(obj.cities, 'cities'),
    optimalCost: optionalNumber(obj.optimal_cost, 'optimal_cost'),
    leaderboard: parseLeaderboard(obj.leaderboard),
  }
}

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

export interface BenchmarkRow {
  dataset: string
  /** Display name from the problem doc, falling back to the dataset id. */
  name: string
  cities: number | null
  edgeWeightType: string | null
  config: string
  /** Known optimum, or null when TSPLIB has none published for this instance. */
  optimal: number | null
  gap: Stat | null
  tour: Stat | null
  wall: Stat | null
  peakRssKb: Stat | null
  runs: number
  timeouts: number
  errors: number
  /** No run completed: the cell reports counts and nothing else. */
  noResult: boolean
  /** From bench/solvers.json: whether a single number may stand alone. */
  deterministic: boolean
}

function byInstanceThenConfig(a: BenchmarkRow, b: BenchmarkRow): number {
  if (a.cities !== b.cities) {
    if (a.cities === null) return 1
    if (b.cities === null) return -1
    return a.cities - b.cities
  }
  if (a.dataset !== b.dataset) return a.dataset < b.dataset ? -1 : 1
  return a.config < b.config ? -1 : a.config > b.config ? 1 : 0
}

/**
 * One row per `(instance, config)` cell, smallest instance first.
 *
 * Determinism comes from bench/solvers.json rather than being passed in, so a caller cannot
 * forget it and accidentally present one run as *the* result.
 */
export function buildRows(
  shard: AlgorithmShard,
  meta: Record<string, InstanceMeta> = {},
): BenchmarkRow[] {
  const deterministic = isDeterministic(shard.solver)
  const rows: BenchmarkRow[] = []
  for (const [dataset, instance] of Object.entries(shard.results)) {
    const info = meta[dataset]
    for (const [config, cell] of Object.entries(instance.byConfig)) {
      rows.push({
        dataset,
        name: info?.name ?? dataset,
        cities: instance.cities ?? info?.cities ?? null,
        edgeWeightType: info?.edgeWeightType ?? null,
        config,
        // The instance-level optimum is the authoritative one; the cell's copy agrees with it
        // when the publisher knew it at all.
        optimal: instance.optimalCost ?? cell.optimalCost,
        gap: cell.gapPct,
        tour: cell.tourCost,
        wall: cell.wallS,
        peakRssKb: cell.peakRssKb,
        runs: cell.runs,
        timeouts: cell.timeouts,
        errors: cell.errors,
        noResult: cell.runs === 0,
        deterministic,
      })
    }
  }
  return rows.sort(byInstanceThenConfig)
}

export interface ColumnChoice {
  /** At least one row has a gap *and* the allowance it was measured under is known. */
  gap: boolean
  optimal: boolean
  /** True when the solver is non-deterministic, so a median must never stand alone. */
  spread: boolean
}

export function chooseColumns(
  rows: BenchmarkRow[],
  budgetSeconds: number | null,
): ColumnChoice {
  return {
    // A gap without its budget is uninterpretable, so the column goes instead (decision 4).
    gap: budgetSeconds !== null && rows.some((row) => row.gap !== null),
    optimal: rows.some((row) => row.optimal !== null),
    spread: rows.some((row) => !row.deterministic),
  }
}

// ---------------------------------------------------------------------------
// Summary and chart
// ---------------------------------------------------------------------------

function median(values: number[]): number | null {
  if (values.length === 0) return null
  const sorted = [...values].sort((a, b) => a - b)
  const mid = Math.floor(sorted.length / 2)
  return sorted.length % 2 === 1
    ? sorted[mid]
    : (sorted[mid - 1] + sorted[mid]) / 2
}

export interface Summary {
  instances: number
  configs: string[]
  deterministic: boolean
  budgetSeconds: number | null
  budgetAgreed: boolean | null
  instancesWithOptimum: number
  /** Median of the per-instance median gaps, over the instances that have one. */
  medianGapPct: number | null
  medianGapBestPct: number | null
  medianGapWorstPct: number | null
  /** Median of the per-instance median wall times: a typical run on a typical instance. */
  medianWallS: number | null
  medianPeakRssKb: number | null
  /**
   * Totals, not medians. Tour length is deliberately absent: it is not comparable between
   * instances, so a median across them would be a number that means nothing.
   */
  totalRuns: number
  totalTimeouts: number
  totalErrors: number
}

export function summarise(
  rows: BenchmarkRow[],
  budget: BudgetEntry | null,
): Summary {
  const gaps = rows.flatMap((row) => (row.gap ? [row.gap.median] : []))
  const walls = rows.flatMap((row) => (row.wall ? [row.wall.median] : []))
  return {
    instances: new Set(rows.map((row) => row.dataset)).size,
    configs: [...new Set(rows.map((row) => row.config))].sort(),
    deterministic: rows.every((row) => row.deterministic),
    budgetSeconds: budget?.seconds ?? null,
    budgetAgreed: budget?.agreed ?? null,
    instancesWithOptimum: new Set(
      rows.filter((row) => row.gap !== null).map((row) => row.dataset),
    ).size,
    medianGapPct: median(gaps),
    medianGapBestPct: median(
      rows.flatMap((row) => (row.gap ? [row.gap.best] : [])),
    ),
    medianGapWorstPct: median(
      rows.flatMap((row) => (row.gap ? [row.gap.worst] : [])),
    ),
    medianWallS: median(walls),
    medianPeakRssKb: median(
      rows.flatMap((row) => (row.peakRssKb ? [row.peakRssKb.median] : [])),
    ),
    totalRuns: rows.reduce((total, row) => total + row.runs, 0),
    totalTimeouts: rows.reduce((total, row) => total + row.timeouts, 0),
    totalErrors: rows.reduce((total, row) => total + row.errors, 0),
  }
}

/**
 * Short, high-level readings of the table, so the prose above it can say what the numbers
 * amount to instead of repeating them.
 *
 * Each rule fires only on evidence that would distinguish it from noise and stays silent
 * otherwise: a page claiming "scales well" from four points that do not support it is the same
 * class of false claim as a fabricated gap. Capped, because a paragraph that mentions
 * everything is a paragraph nobody reads.
 */
export function observations(rows: BenchmarkRow[], summary: Summary): string[] {
  const out: string[] = []

  if (summary.totalTimeouts > 0) {
    out.push(
      `${summary.totalTimeouts} run${
        summary.totalTimeouts === 1 ? '' : 's'
      } exhausted the budget and produced no tour.`,
    )
  }
  if (summary.totalErrors > 0) {
    out.push(
      `${summary.totalErrors} run${
        summary.totalErrors === 1 ? '' : 's'
      } failed outright.`,
    )
  }

  // Instance size against gap, split into halves rather than fitted: with a handful of
  // instances a slope would be noise dressed up as a finding.
  const withGap = rows
    .filter((row) => row.gap !== null && row.cities !== null)
    .sort((a, b) => a.cities! - b.cities!)
  if (withGap.length >= 4) {
    const mid = Math.floor(withGap.length / 2)
    const smaller = median(withGap.slice(0, mid).map((row) => row.gap!.median))
    const larger = median(withGap.slice(mid).map((row) => row.gap!.median))
    if (smaller !== null && larger !== null) {
      if (larger - smaller > Math.max(1, larger * 0.25)) {
        out.push(
          'It stays closest to the optimum on the smaller instances and loses ground as they grow.',
        )
      } else if (smaller - larger > Math.max(1, smaller * 0.25)) {
        out.push('It does better on the larger instances here.')
      } else {
        out.push('Its gap is broadly flat across the instance sizes measured.')
      }
    }
  }

  if (summary.budgetSeconds !== null && summary.medianWallS !== null) {
    const used = summary.medianWallS / summary.budgetSeconds
    if (used < 0.05) {
      out.push(
        'A typical run finishes in a small fraction of its budget, so the allowance is not what limits it.',
      )
    } else if (used > 0.5) {
      out.push('A typical run spends most of its budget.')
    }
  }

  if (!summary.deterministic) {
    out.push(
      'Its tours vary between runs, so the median is shown with the best and worst beside it.',
    )
  }

  const rss = summary.medianPeakRssKb
  if (rss !== null) {
    if (rss > 200_000) {
      out.push('It is memory-hungry, taking hundreds of megabytes per run.')
    } else if (rss < 20_000) {
      out.push('Memory stays in the tens of megabytes.')
    }
  }

  return out.slice(0, 3)
}

export type ChartAxis = 'gap' | 'tour'

export interface ChartPoint {
  dataset: string
  label: string
  config: string
  x: number
  y: number
}

export interface ChartSeries {
  axis: ChartAxis
  /** What the y value is, for the axis label. */
  axisLabel: string
  points: ChartPoint[]
  omitted: { noOptimum: number; noResult: number; noSize: number }
}

/**
 * Scatter input: instance size against quality.
 *
 * The two quantities are never mixed. When any row has a gap the chart is all gaps and the
 * rows without a published optimum are omitted — and counted, so the caption can say how many
 * and why. Only when no row has a gap does it fall back to tour length, which answers "how does
 * it scale" without pretending to answer "how close to optimal".
 */
export function chartSeries(rows: BenchmarkRow[]): ChartSeries {
  const noResult = rows.filter((row) => row.noResult).length
  const sized = rows.filter((row) => !row.noResult && row.cities !== null)
  const noSize = rows.filter(
    (row) => !row.noResult && row.cities === null,
  ).length

  const gapRows = sized.filter((row) => row.gap !== null)
  if (gapRows.length > 0) {
    return {
      axis: 'gap',
      axisLabel: 'gap vs known optimum (%)',
      points: gapRows.map((row) => toPoint(row, row.gap!.median)),
      omitted: { noOptimum: sized.length - gapRows.length, noResult, noSize },
    }
  }

  const tourRows = sized.filter((row) => row.tour !== null)
  return {
    axis: 'tour',
    axisLabel: 'tour length (no published optimum)',
    points: tourRows.map((row) => toPoint(row, row.tour!.median)),
    omitted: { noOptimum: 0, noResult, noSize },
  }
}

function toPoint(row: BenchmarkRow, y: number): ChartPoint {
  return {
    dataset: row.dataset,
    label: row.name,
    config: row.config,
    x: row.cities!,
    y,
  }
}

// ---------------------------------------------------------------------------
// The landing chart: one instance, every solver
// ---------------------------------------------------------------------------

export interface SolverPoint {
  solver: string
  config: string
  /** Null for a solver that has a shard but no entry in bench/solvers.json. */
  family: SolverFamily | null
  /** Wall-clock seconds. The x axis is runtime, so this is never the quality value. */
  wallS: number
  /** Gap percent or tour length, whichever the axis turned out to be. */
  y: number
}

export interface SolverSeries {
  dataset: string
  axis: ChartAxis
  axisLabel: string
  points: SolverPoint[]
  /** The best point in each family, so the chart labels a few points rather than all of them. */
  labelled: { solver: string; config: string }[]
  omitted: { noResult: number; noTime: number; noQuality: number }
}

/**
 * Runtime against quality for every solver measured on one instance.
 *
 * Same axis rule as the per-algorithm chart and for the same reason: a gap and a tour length
 * are different quantities, so the series is all of one or all of the other, never a mix.
 * Families are kept on the point rather than folded into the y value, because the allowance
 * differs per family and the chart has to be able to say so.
 */
export function solverSeries(shard: ProblemShard): SolverSeries {
  const ran = shard.leaderboard.filter((entry) => entry.runs > 0)
  const noResult = shard.leaderboard.length - ran.length
  const timed = ran.filter((entry) => entry.wallS !== null)
  const noTime = ran.length - timed.length

  const base = { dataset: shard.dataset, omitted: { noResult, noTime } }

  const withGap = timed.filter((entry) => entry.gapPct !== null)
  if (withGap.length > 0) {
    const points = withGap.map((entry) =>
      toSolverPoint(entry, entry.gapPct!.median),
    )
    return {
      ...base,
      axis: 'gap',
      axisLabel: 'gap vs known optimum (%)',
      points,
      labelled: bestPerFamily(points),
      omitted: { ...base.omitted, noQuality: timed.length - withGap.length },
    }
  }

  const withTour = timed.filter((entry) => entry.tourCost !== null)
  const points = withTour.map((entry) =>
    toSolverPoint(entry, entry.tourCost!.median),
  )
  return {
    ...base,
    axis: 'tour',
    axisLabel: 'tour length (no published optimum)',
    points,
    labelled: bestPerFamily(points),
    omitted: { ...base.omitted, noQuality: timed.length - withTour.length },
  }
}

function toSolverPoint(entry: LeaderboardEntry, y: number): SolverPoint {
  return {
    solver: entry.solver,
    config: entry.config,
    family: solverById(entry.solver)?.family ?? null,
    wallS: entry.wallS!.median,
    y,
  }
}

function bestPerFamily(
  points: SolverPoint[],
): { solver: string; config: string }[] {
  const best = new Map<SolverFamily | null, SolverPoint>()
  for (const point of points) {
    const current = best.get(point.family)
    if (!current || point.y < current.y) best.set(point.family, point)
  }
  return [...best.values()].map((point) => ({
    solver: point.solver,
    config: point.config,
  }))
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

export const DASH = '—'

export function formatGapPct(value: number | null): string {
  return value === null ? DASH : `${value.toFixed(2)}%`
}

export function formatTour(value: number | null): string {
  return value === null ? DASH : value.toFixed(2)
}

export function formatSeconds(value: number | null): string {
  if (value === null) return DASH
  if (value < 1) return `${value.toFixed(3)} s`
  if (value < 60) return `${value.toFixed(2)} s`
  return `${(value / 60).toFixed(1)} min`
}

/**
 * How many runs were lost, and to what.
 *
 * A killed run has no tour to report, so it is never rendered as a value: the column says how
 * many runs were lost instead (decision 5). One function for both a row and the table's
 * summary row, so the two cannot describe the same losses differently.
 */
export function lostRunsNote(timeouts: number, errors: number): string {
  const parts: string[] = []
  if (timeouts > 0) {
    parts.push(`${timeouts} timeout${timeouts === 1 ? '' : 's'}`)
  }
  if (errors > 0) {
    parts.push(`${errors} error${errors === 1 ? '' : 's'}`)
  }
  return parts.length > 0 ? parts.join(', ') : DASH
}

export function timeoutNote(row: BenchmarkRow): string {
  return lostRunsNote(row.timeouts, row.errors)
}

export function formatDirty(dirty: boolean | null): string {
  if (dirty === null) return 'dirty state not recorded'
  return dirty ? 'dirty tree' : 'clean tree'
}

export interface ProvenanceLine {
  label: string
  value: string
}

/**
 * Provenance for one shard.
 *
 * Always called with the shard being rendered, never with the manifest: solvers are measured at
 * different commits, so a page-level version would be a false claim about all but one of them.
 */
export function provenance(measurement: Measurement): ProvenanceLine[] {
  const commit =
    measurement.gitCommitSource === 'publish-time'
      ? `${measurement.gitCommit} (publish-time, not the measured binary)`
      : `${measurement.gitCommit} (${measurement.gitCommitSource})`
  const env = measurement.environment
  return [
    { label: 'measured at', value: commit },
    { label: 'teeline', value: measurement.teelineVersion ?? 'not recorded' },
    { label: 'measured', value: measurement.measuredAt ?? 'not recorded' },
    { label: 'tree', value: formatDirty(measurement.dirty) },
    {
      label: 'environment',
      value: env
        ? `${env.os ?? '?'} / ${env.build ?? '?'} / rust ${
            env.rustVersion ?? 'not recorded'
          }`
        : 'not recorded',
    },
  ]
}

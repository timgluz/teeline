import { describe, expect, it } from 'vitest'

// Fixtures are pinned to the 2026-10-11 prelim publish (`index.json`
// generated_at 2026-10-11T11:00:18Z, measurements at commit e4588d6f) and are trimmed
// copies of the live shards. `algorithm-synthetic.json` is hand-written: it covers the
// two cases the live tree cannot — a cell where every run was killed, and a
// non-deterministic solver's spread — plus a shard whose provenance is publish-time
// only. Read with `?raw`, as solver-index.test.ts does, so this test needs no Node
// filesystem types.
import twoOptRaw from './__fixtures__/algorithm-2opt.json?raw'
import branchBoundRaw from './__fixtures__/algorithm-branch-bound.json?raw'
import syntheticRaw from './__fixtures__/algorithm-synthetic.json?raw'
import berlinRaw from './__fixtures__/problem-berlin52.json?raw'
import problemSyntheticRaw from './__fixtures__/problem-synthetic.json?raw'

import {
  DASH,
  SCHEMA_VERSION,
  algorithmShardUrl,
  benchmarkBaseUrl,
  budgetFor,
  buildRows,
  chartSeries,
  chooseColumns,
  fetchAlgorithmShard,
  fetchBenchmarkIndex,
  fetchProblemShard,
  formatDirty,
  formatGapPct,
  formatSeconds,
  formatTour,
  lostRunsNote,
  observations,
  parseAlgorithmShard,
  parseBenchmarkIndex,
  parseProblemShard,
  provenance,
  solverSeries,
  summarise,
  timeoutNote,
  type BenchmarkRow,
  type FetchLike,
} from './benchmark'

const twoOpt = parseAlgorithmShard(JSON.parse(twoOptRaw))
const branchBound = parseAlgorithmShard(JSON.parse(branchBoundRaw))
const synthetic = parseAlgorithmShard(JSON.parse(syntheticRaw))

/** A fetch stub that answers every request with the same status and body. */
function stubFetch(status: number, body: unknown): FetchLike {
  return async () => ({
    status,
    ok: status >= 200 && status < 300,
    json: async () => body,
  })
}

const budget = { seconds: 300, agreed: false }

describe('parseAlgorithmShard', () => {
  it('normalises the wire field names', () => {
    expect(twoOpt.solver).toBe('2opt')
    expect(twoOpt.configs).toEqual(['budget'])
    expect(twoOpt.results.eil76.byConfig.budget.gapPct?.median).toBe(15.2472)
    expect(twoOpt.results.eil76.byConfig.budget.wallS?.median).toBeGreaterThan(
      0,
    )
    expect(twoOpt.measurement.teelineVersion).toBe('1.0.12')
    expect(twoOpt.measurement.gitCommit).toBe('e4588d6f')
  })

  it('keeps an unknown optimum as null rather than a zero gap', () => {
    // berlin52 has no published optimum, so no gap exists for it. A 0 here would read
    // as "reached the optimum", which is the opposite of what the data says.
    expect(twoOpt.results.berlin52.optimalCost).toBeNull()
    expect(twoOpt.results.berlin52.byConfig.budget.gapPct).toBeNull()
    expect(
      twoOpt.results.berlin52.byConfig.budget.tourCost?.median,
    ).toBeGreaterThan(0)
  })

  it('reads a killed cell whose summaries are explicitly null', () => {
    const cell = synthetic.results.a280.byConfig.default
    expect(cell.runs).toBe(0)
    expect(cell.timeouts).toBe(5)
    expect(cell.tourCost).toBeNull()
    expect(cell.wallS).toBeNull()
  })

  it('refuses a schema version it does not know', () => {
    const future = {
      ...JSON.parse(twoOptRaw),
      schema_version: SCHEMA_VERSION + 1,
    }
    expect(() => parseAlgorithmShard(future)).toThrow(
      /unsupported benchmark schema_version 3/,
    )
  })

  it('refuses a measurement that cannot name the build', () => {
    const nameless = { ...JSON.parse(twoOptRaw), measurement: {} }
    expect(() => parseAlgorithmShard(nameless)).toThrow(/git_commit/)
  })

  it('rejects a shard whose results are not an object', () => {
    const broken = { ...JSON.parse(twoOptRaw), results: [] }
    expect(() => parseAlgorithmShard(broken)).toThrow(
      /results is not an object/,
    )
  })
})

describe('fetchAlgorithmShard', () => {
  it('treats 404 as absence, not as a failure', async () => {
    expect(
      await fetchAlgorithmShard('nope', false, stubFetch(404, null)),
    ).toEqual({
      kind: 'absent',
    })
  })

  it('reports any other status as a real error', async () => {
    const result = await fetchAlgorithmShard(
      '2opt',
      false,
      stubFetch(500, null),
    )
    expect(result.kind).toBe('error')
    if (result.kind === 'error') expect(result.status).toBe(500)
  })

  it('reports a transport failure without throwing', async () => {
    const failing: FetchLike = async () => {
      throw new Error('network down')
    }
    const result = await fetchAlgorithmShard('2opt', false, failing)
    expect(result.kind).toBe('error')
    if (result.kind === 'error') {
      expect(result.status).toBeNull()
      expect(result.message).toBe('network down')
    }
  })

  it('reports an unreadable body as an error rather than a blank section', async () => {
    const result = await fetchAlgorithmShard(
      '2opt',
      false,
      stubFetch(200, { schema_version: 99 }),
    )
    expect(result.kind).toBe('error')
  })

  it('returns the parsed shard on success and reads the published URL', async () => {
    let requested = ''
    const ok: FetchLike = async (url) => {
      requested = url
      return { status: 200, ok: true, json: async () => JSON.parse(twoOptRaw) }
    }
    const result = await fetchAlgorithmShard('2opt', true, ok)
    expect(requested).toBe('/bench/v2/algorithms/2opt.json')
    expect(result.kind).toBe('ok')
  })
})

describe('fetchBenchmarkIndex and budgetFor', () => {
  const indexBody = {
    schema_version: 2,
    generated_at: '2026-10-11T11:00:18Z',
    tier: 'a',
    budgets: {
      exact: { seconds: 60, agreed: false },
      local: { seconds: 300, agreed: false },
    },
    solvers: [
      { id: '2opt', name: '2-opt', family: 'local', configs: ['budget'] },
    ],
    problems: [{ id: 'eil76', cities: 76 }],
  }

  it('reads the per-family budgets the manifest publishes', () => {
    const index = parseBenchmarkIndex(indexBody)
    expect(index.tier).toBe('a')
    expect(budgetFor(index, 'local')).toEqual({ seconds: 300, agreed: false })
    expect(budgetFor(index, 'meta')).toBeNull()
  })

  it('returns null when the manifest cannot be read', async () => {
    expect(await fetchBenchmarkIndex(false, stubFetch(404, null))).toBeNull()
    expect(
      await fetchBenchmarkIndex(false, stubFetch(200, { schema_version: 3 })),
    ).toBeNull()
    expect(budgetFor(null, 'local')).toBeNull()
  })
})

describe('buildRows', () => {
  it('orders rows by instance size', () => {
    expect(buildRows(twoOpt).map((row) => row.dataset)).toEqual([
      'gr17',
      'berlin52',
      'eil76',
    ])
  })

  it('keeps two configs of one solver as separate rows', () => {
    const rows = buildRows(branchBound)
    expect(rows.map((row) => `${row.dataset}/${row.config}`)).toEqual([
      'burma14/budget',
      'burma14/exact',
      'gr17/budget',
      'gr17/exact',
    ])
    // A beam search and an exact run must not collapse into one entry.
    expect(
      rows.find((row) => row.dataset === 'gr17' && row.config === 'budget')?.gap
        ?.median,
    ).toBe(4.8921)
    expect(
      rows.find((row) => row.dataset === 'gr17' && row.config === 'exact')?.gap
        ?.median,
    ).toBe(0)
  })

  it('takes the display name and distance type from the problem collection', () => {
    const rows = buildRows(twoOpt, {
      eil76: { name: 'Eil 76', edgeWeightType: 'EUC_2D', cities: 76 },
    })
    const eil76 = rows.find((row) => row.dataset === 'eil76')!
    expect(eil76.name).toBe('Eil 76')
    expect(eil76.edgeWeightType).toBe('EUC_2D')
    // A dataset with no problem doc still renders, under its own id.
    expect(rows.find((row) => row.dataset === 'gr17')?.name).toBe('gr17')
  })

  it('takes determinism from bench/solvers.json', () => {
    // 2opt is measured deterministic; aco is measured varying. The page must not be able
    // to choose this per call site, or one of them will get it wrong.
    expect(buildRows(twoOpt).every((row) => row.deterministic)).toBe(true)
    expect(buildRows(synthetic).every((row) => !row.deterministic)).toBe(true)
  })

  it('marks a cell where nothing finished as having no result', () => {
    const row = buildRows(synthetic).find((row) => row.dataset === 'a280')!
    expect(row.noResult).toBe(true)
    expect(row.gap).toBeNull()
    expect(row.tour).toBeNull()
    expect(row.wall).toBeNull()
  })
})

describe('chooseColumns', () => {
  it('shows a gap column when a row has a gap and the budget is known', () => {
    expect(chooseColumns(buildRows(twoOpt), budget.seconds)).toEqual({
      gap: true,
      optimal: true,
      spread: false,
    })
  })

  it('drops the gap column when the budget is unknown', () => {
    // A gap without the allowance it was measured under is uninterpretable, so the
    // column goes rather than the budget (decision 4).
    expect(chooseColumns(buildRows(twoOpt), null).gap).toBe(false)
  })

  it('drops the gap column when no row has one', () => {
    const noOptimum = buildRows(twoOpt).filter((row) => row.optimal === null)
    expect(noOptimum.length).toBeGreaterThan(0)
    expect(chooseColumns(noOptimum, budget.seconds).gap).toBe(false)
    expect(chooseColumns(noOptimum, budget.seconds).optimal).toBe(false)
  })

  it('asks for a spread only for a non-deterministic solver', () => {
    expect(chooseColumns(buildRows(synthetic), budget.seconds).spread).toBe(
      true,
    )
    expect(chooseColumns(buildRows(twoOpt), budget.seconds).spread).toBe(false)
  })
})

describe('summarise', () => {
  it('summarises one solver across its instances', () => {
    const summary = summarise(buildRows(twoOpt), budget)
    expect(summary.instances).toBe(3)
    expect(summary.configs).toEqual(['budget'])
    expect(summary.deterministic).toBe(true)
    expect(summary.instancesWithOptimum).toBe(2)
    expect(summary.medianGapPct).toBeCloseTo(7.6236, 4)
    expect(summary.budgetSeconds).toBe(300)
    expect(summary.budgetAgreed).toBe(false)
    expect(summary.totalTimeouts).toBe(0)
  })

  it('carries the aggregates the table summary row renders', () => {
    const summary = summarise(buildRows(twoOpt), budget)
    // Best/worst medians, so a varying solver's summary row is a spread too.
    expect(summary.medianGapBestPct).toBeCloseTo(7.6236, 4)
    expect(summary.medianGapWorstPct).toBeCloseTo(7.6236, 4)
    expect(summary.medianWallS).toBeGreaterThan(0)
    expect(summary.medianPeakRssKb).toBeGreaterThan(0)
    // Counts are totals, not medians.
    expect(summary.totalRuns).toBe(15)
    expect(summary.totalErrors).toBe(0)
  })

  it('counts timeouts without letting them into the gaps', () => {
    const rows = buildRows(synthetic)
    const summary = summarise(rows, null)
    expect(summary.totalTimeouts).toBe(5)
    // Only eil76 has a gap; the killed cell contributes nothing.
    expect(summary.medianGapPct).toBe(3.5)
    expect(summary.budgetSeconds).toBeNull()
  })
})

describe('observations', () => {
  // Plain rows, so the trend rules can be driven past the thresholds the real fixtures do
  // not reach.
  function rowWithGap(cities: number, gapMedian: number): BenchmarkRow {
    return {
      ...buildRows(twoOpt)[0],
      dataset: `d${cities}`,
      name: `d${cities}`,
      cities,
      gap: { median: gapMedian, best: gapMedian, worst: gapMedian },
    }
  }

  it('says nothing it cannot support', () => {
    // Two instances with a gap is below the threshold for a size trend, and nothing else
    // fires for this fixture beyond the budget remark.
    const rows = buildRows(twoOpt)
    const notes = observations(rows, summarise(rows, budget))
    expect(
      notes.some((note) => /smaller instances|larger instances/.test(note)),
    ).toBe(false)
  })

  it('reads a size trend only when the halves are far enough apart', () => {
    const rows = [
      rowWithGap(10, 1),
      rowWithGap(20, 2),
      rowWithGap(30, 10),
      rowWithGap(40, 20),
    ]
    const growing = observations(rows, summarise(rows, budget))
    expect(growing.join(' ')).toMatch(/closest to the optimum on the smaller/)

    const flat = [10, 20, 30, 40].map((cities) => rowWithGap(cities, 5))
    expect(observations(flat, summarise(flat, budget)).join(' ')).toMatch(
      /broadly flat/,
    )
  })

  it('reports lost runs first, because they are missing data', () => {
    const rows = buildRows(synthetic)
    const notes = observations(rows, summarise(rows, null))
    expect(notes[0]).toMatch(/exhausted the budget and produced no tour/)
    expect(notes[1]).toMatch(/failed outright/)
    expect(notes.length).toBeLessThanOrEqual(3)
  })

  it('notes varying tours', () => {
    const rows = buildRows(synthetic)
    const summary = summarise(rows, { seconds: 1, agreed: true })
    expect(observations(rows, summary).join(' ')).toMatch(
      /tours vary between runs/,
    )
  })

  it('notes memory character', () => {
    const rows = buildRows(twoOpt)
    expect(observations(rows, summarise(rows, budget)).join(' ')).toMatch(
      /Memory stays in the tens of megabytes/,
    )
  })
})

describe('chartSeries', () => {
  it('plots gaps when the shard has any, and counts what it left out', () => {
    const series = chartSeries(buildRows(twoOpt))
    expect(series.axis).toBe('gap')
    expect(series.points.map((point) => point.dataset)).toEqual([
      'gr17',
      'eil76',
    ])
    // berlin52 has no optimum, so it is omitted rather than plotted as a zero gap.
    expect(series.omitted.noOptimum).toBe(1)
    expect(series.points.every((point) => Number.isFinite(point.y))).toBe(true)
  })

  it('falls back to tour length, and says so, when no instance has an optimum', () => {
    const noOptimum = buildRows(synthetic).filter((row) => row.optimal === null)
    const series = chartSeries(noOptimum)
    expect(series.axis).toBe('tour')
    expect(series.axisLabel).toMatch(/no published optimum/)
    expect(series.points.map((point) => point.dataset)).toEqual(['berlin52'])
    expect(series.omitted.noResult).toBe(1)
  })

  it('never plots a killed cell', () => {
    const series = chartSeries(buildRows(synthetic))
    expect(series.points.map((point) => point.dataset)).not.toContain('a280')
    expect(series.omitted.noResult).toBe(1)
  })

  it('uses instance size for x', () => {
    const series = chartSeries(buildRows(twoOpt))
    expect(series.points.find((point) => point.dataset === 'gr17')?.x).toBe(17)
  })
})

describe('timeoutNote', () => {
  it('renders a killed cell as counts, never as a value', () => {
    const row = buildRows(synthetic).find((row) => row.dataset === 'a280')!
    expect(timeoutNote(row)).toBe('5 timeouts, 1 error')
  })

  it('renders nothing for a cell that finished', () => {
    expect(timeoutNote(buildRows(twoOpt)[0])).toBe(DASH)
  })

  it('singularises one lost run', () => {
    const row = { ...buildRows(twoOpt)[0], timeouts: 1, errors: 0 }
    expect(timeoutNote(row)).toBe('1 timeout')
  })

  it('describes the summary row with the same function as a data row', () => {
    // The aggregate row and a data row must not describe the same losses differently.
    expect(lostRunsNote(5, 1)).toBe('5 timeouts, 1 error')
    expect(lostRunsNote(0, 0)).toBe(DASH)
  })
})

describe('provenance', () => {
  it('names the commit the numbers came from', () => {
    const lines = provenance(twoOpt.measurement)
    const commit = lines.find((line) => line.label === 'measured at')!
    expect(commit.value).toBe('e4588d6f (jsonl)')
    expect(lines.find((line) => line.label === 'teeline')?.value).toBe('1.0.12')
  })

  it('flags a publish-time commit as not being the measured binary', () => {
    const lines = provenance(synthetic.measurement)
    expect(lines.find((line) => line.label === 'measured at')?.value).toMatch(
      /publish-time, not the measured binary/,
    )
  })

  it('does not read an unrecorded dirty flag as clean', () => {
    expect(formatDirty(null)).toBe('dirty state not recorded')
    expect(formatDirty(true)).toBe('dirty tree')
    expect(formatDirty(false)).toBe('clean tree')
    expect(
      provenance(synthetic.measurement).find((line) => line.label === 'tree')
        ?.value,
    ).toBe('dirty state not recorded')
  })

  it('does not invent a version or an environment', () => {
    const lines = provenance(synthetic.measurement)
    expect(lines.find((line) => line.label === 'teeline')?.value).toBe(
      'not recorded',
    )
    expect(lines.find((line) => line.label === 'environment')?.value).toBe(
      'not recorded',
    )
  })
})

describe('problem shards and the landing chart', () => {
  const berlin = parseProblemShard(JSON.parse(berlinRaw))
  const syntheticProblem = parseProblemShard(JSON.parse(problemSyntheticRaw))

  it('parses a leaderboard and keeps an unknown optimum as null', () => {
    expect(berlin.dataset).toBe('berlin52')
    expect(berlin.cities).toBe(52)
    expect(berlin.optimalCost).toBeNull()
    expect(berlin.leaderboard[0].solver).toBe('lk')
    expect(berlin.leaderboard[0].gapPct).toBeNull()
    expect(berlin.leaderboard[0].tourCost?.median).toBeGreaterThan(0)
  })

  it('treats one solver publishing two configs as two entries', () => {
    const configs = syntheticProblem.leaderboard
      .filter((entry) => entry.solver === 'branch_bound')
      .map((entry) => entry.config)
      .sort()
    expect(configs).toEqual(['budget', 'exact'])
  })

  it('refuses a schema version it does not know', () => {
    expect(() =>
      parseProblemShard({ ...JSON.parse(berlinRaw), schema_version: 99 }),
    ).toThrow(/unsupported benchmark schema_version 99/)
  })

  it('treats a 404 as absence, like the algorithm shards', async () => {
    const result = await fetchProblemShard('nope', false, stubFetch(404, null))
    expect(result).toEqual({ kind: 'absent' })
  })

  it('plots tour length and says so when no optimum is published', () => {
    const series = solverSeries(berlin)
    expect(series.axis).toBe('tour')
    expect(series.axisLabel).toMatch(/no published optimum/)
    // Serialised straight to JSON, so the fixture's null optimum is the real case.
    expect(JSON.parse(berlinRaw).optimal_cost).toBeNull()
    expect(series.points).toHaveLength(8)
    expect(series.points.every((point) => Number.isFinite(point.wallS))).toBe(
      true,
    )
  })

  it('plots gaps, never a mix, when an optimum is published', () => {
    const series = solverSeries(syntheticProblem)
    expect(series.axis).toBe('gap')
    // The timed-out solver has no result, so it is counted rather than plotted.
    expect(series.points.map((point) => point.solver)).not.toContain('gsa')
    expect(series.omitted.noResult).toBe(1)
  })

  it('carries each solver\u2019s family, and null for one the index does not know', () => {
    const series = solverSeries(syntheticProblem)
    const byName = new Map(series.points.map((point) => [point.solver, point]))
    expect(byName.get('lk')?.family).toBe('local')
    expect(byName.get('shuffle')?.family).toBe('utility')
    expect(byName.get('mystery_solver')?.family).toBeNull()
  })

  it('labels only the best point in each family', () => {
    const series = solverSeries(syntheticProblem)
    const labelled = series.labelled.map((entry) => entry.solver)
    // One per family present: exact, local, meta, utility — plus the unknown-solver group.
    expect(labelled).toHaveLength(5)
    expect(labelled).toContain('branch_bound')
    // Two solvers in `local`, only the better one earns a label.
    expect(labelled).toContain('lk')
    expect(labelled).not.toContain('2opt')
  })
})

describe('formatting', () => {
  it('formats gaps and tour costs', () => {
    expect(formatGapPct(15.2472)).toBe('15.25%')
    expect(formatGapPct(0)).toBe('0.00%')
    expect(formatGapPct(null)).toBe(DASH)
    expect(formatTour(8384.1885)).toBe('8384.19')
    expect(formatTour(null)).toBe(DASH)
  })

  it('scales wall time to the magnitude', () => {
    expect(formatSeconds(0.0129)).toBe('0.013 s')
    expect(formatSeconds(27.8289)).toBe('27.83 s')
    expect(formatSeconds(90)).toBe('1.5 min')
    expect(formatSeconds(null)).toBe(DASH)
  })
})

describe('URLs', () => {
  it('proxies in dev and reads R2 in production', () => {
    expect(benchmarkBaseUrl(true)).toBe('/bench/v2')
    expect(benchmarkBaseUrl(false)).toBe(
      'https://static.tspsolver.com/bench/v2',
    )
    expect(algorithmShardUrl('2opt', true)).toBe(
      '/bench/v2/algorithms/2opt.json',
    )
    expect(algorithmShardUrl('a/b', false)).toBe(
      'https://static.tspsolver.com/bench/v2/algorithms/a%2Fb.json',
    )
  })
})

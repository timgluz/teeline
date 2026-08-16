import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  SCENARIOS, makeInitState, stepOnce, popcount,
} from "./bhk-algo"
import type { Phase, Scenario, SimState } from "./bhk-algo"

const DEFAULT_SCENARIO = SCENARIOS.grid_6
const SPEEDS = [600, 420, 280, 180, 100, 50, 25]
const SPEED_LABELS = ["1x", "2x", "3x", "4x", "5x", "6x", "7x"]

const INF = Infinity

// ---------------------------------------------------------------
// DpTable — rows = end city, columns = subset bitmask. Base cells
// pre-filled; the next cell to compute is ringed; click a cell to
// inspect its computation. Read-back cells turn gold.
// ---------------------------------------------------------------
function DpTable({ sim, selected, onSelect }: {
  sim: SimState
  selected: { mask: number; row: number } | null
  onSelect: (mask: number, row: number) => void
}) {
  const m = sim.m
  const full = (1 << m) - 1
  const masks = Array.from({ length: full }, (_, i) => i + 1)

  // cells of the optimal route (mask, row) — gold in readback/done
  const routeCells = useMemo(() => {
    const cells = new Set<string>()
    if (!sim.route) return cells
    let mask = full
    for (let k = sim.route.length - 1; k >= 1; k--) {
      const city = sim.route[k]
      const row = city - 1
      if (mask & (1 << row)) cells.add(`${mask}:${row}`)
      mask &= ~(1 << row)
    }
    return cells
  }, [sim.route, full])

  const nextCell = sim.fillPtr < sim.fillOrder.length ? sim.fillOrder[sim.fillPtr] : null

  return (
    <div className="bhk-table-scroll">
      <table className="bhk-table">
        <thead>
          <tr>
            <th className="bhk-th bhk-th-row">end \ subset</th>
            {masks.map((mask) => (
              <th key={mask} className="bhk-th">{mask.toString(2).padStart(m, '0')}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {Array.from({ length: m }, (_, row) => (
            <tr key={row}>
              <th className="bhk-th bhk-th-row">{row + 1}</th>
              {masks.map((mask) => {
                const val = sim.table[row][mask]
                const isSel = selected?.mask === mask && selected?.row === row
                const isNext = nextCell?.mask === mask && nextCell?.row === row
                const isRoute = routeCells.has(`${mask}:${row}`)
                const isBase = popcount(mask) === 1 && (mask & (1 << row)) !== 0
                const cls = [
                  'bhk-cell',
                  val === INF ? 'bhk-cell-empty' : '',
                  isBase ? 'bhk-cell-base' : '',
                  isRoute ? 'bhk-cell-route' : '',
                  isNext ? 'bhk-cell-next' : '',
                  isSel ? 'bhk-cell-sel' : '',
                ].join(' ').trim()
                return (
                  <td key={mask} className={cls} onClick={() => onSelect(mask, row)}>
                    {val === INF ? '·' : val.toFixed(0)}
                  </td>
                )
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

// ---------------------------------------------------------------
// CityMap — current subset (forward) or revealed route (read-back)
// ---------------------------------------------------------------
function CityMap({ sim }: { sim: SimState }) {
  const m = sim.m
  const n = sim.n
  const inReadback = sim.phase === 'readback' || sim.phase === 'done'

  let subset = new Set<number>([0])
  if (inReadback) {
    subset = new Set([0, ...sim.readback])
  } else if (sim.fillPtr < sim.fillOrder.length) {
    const { mask } = sim.fillOrder[sim.fillPtr]
    subset = new Set([0])
    for (let i = 0; i < m; i++) if (mask & (1 << i)) subset.add(i + 1)
  }

  const routeEdges: Array<[number, number]> = []
  if (inReadback && sim.readback.length >= 2) {
    const pts = [0, ...sim.readback]
    for (let k = 0; k < pts.length - 1; k++) routeEdges.push([pts[k], pts[k + 1]])
    if (sim.phase === 'done') routeEdges.push([pts[pts.length - 1], 0])
  }

  return (
    <svg viewBox="0 0 300 300" className="bhk-map" role="img" aria-label="Current subset on the map">
      <rect x={0} y={0} width={300} height={300} className="bhk-bg" />
      {routeEdges.map(([a, b]) => (
        <line key={`${a}-${b}`} className="bhk-map-edge"
          x1={sim.cities[a][0]} y1={sim.cities[a][1]}
          x2={sim.cities[b][0]} y2={sim.cities[b][1]} />
      ))}
      {Array.from({ length: n }, (_, i) => i).map((i) => {
        const inSub = subset.has(i)
        let cls = 'bhk-city-dim'
        if (i === 0) cls = 'bhk-city-start'
        else if (inSub) cls = 'bhk-city'
        return (
          <g key={i}>
            <circle className={cls} cx={sim.cities[i][0]} cy={sim.cities[i][1]} r={7} />
            <text className="bhk-label" x={sim.cities[i][0]} y={sim.cities[i][1] + 16}>{i}</text>
          </g>
        )
      })}
    </svg>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function BhkExplainer() {
  const [sim, setSim] = useState<SimState>(() => makeInitState(DEFAULT_SCENARIO))
  const [running, setRunning] = useState(false)
  const [speedIdx, setSpeedIdx] = useState(2)
  const [selected, setSelected] = useState<{ mask: number; row: number } | null>(null)

  const simRef = useRef(sim)
  const historyRef = useRef<SimState[]>([])
  const scenarioRef = useRef<Scenario>(DEFAULT_SCENARIO)

  const commit = useCallback((next: SimState) => {
    simRef.current = next
    setSim(next)
  }, [])

  const reinit = useCallback((scenario: Scenario) => {
    scenarioRef.current = scenario
    historyRef.current = []
    setSelected(null)
    commit(makeInitState(scenario))
    setRunning(false)
  }, [commit])

  const stepForward = useCallback(() => {
    const cur = simRef.current
    if (cur.phase === 'done') return
    historyRef.current.push(structuredClone(cur))
    const next = stepOnce(cur)
    simRef.current = next
    setSim(next)
    if (next.phase === 'done') setRunning(false)
  }, [])

  const stepBack = useCallback(() => {
    const h = historyRef.current
    if (h.length === 0 || running) return
    commit(h.pop()!)
  }, [running, commit])

  useEffect(() => {
    if (!running) return
    const id = setInterval(stepForward, SPEEDS[speedIdx])
    return () => clearInterval(id)
  }, [running, speedIdx, stepForward])

  // jump to a phase (forward start / read-back start / done)
  const jumpTo = useCallback((target: Phase) => {
    let s = makeInitState(scenarioRef.current)
    let guard = 600
    while (s.phase !== target && s.phase !== 'done' && guard-- > 0) s = stepOnce(s)
    if (s.phase !== target && target !== 'done') s = stepOnce(s)
    historyRef.current = []
    commit(s)
    setRunning(false)
  }, [commit])

  const s = sim
  const full = (1 << s.m) - 1
  const currentMask = s.fillPtr < s.fillOrder.length ? s.fillOrder[s.fillPtr].mask : full
  const selectedCell = selected && selected.mask >= 1 && selected.mask <= full && selected.row >= 0 && selected.row < s.m
    ? { value: s.table[selected.row][selected.mask], mask: selected.mask, row: selected.row }
    : null

  // cell info text
  let cellInfo = 'click a cell to see how its value was computed'
  if (selectedCell) {
    const { value, mask, row } = selectedCell
    if (value === INF) {
      cellInfo = `dp[${mask.toString(2).padStart(s.m, '0')}][${row + 1}] — not computed (city ${row + 1} not in subset)`
    } else if (popcount(mask) === 1) {
      cellInfo = `base — dp[{${row + 1}}][${row + 1}] = d(0, ${row + 1}) = ${value.toFixed(1)}`
    } else {
      const rest = mask & ~(1 << row)
      const via = s.pred[row][mask]
      cellInfo = `dp[${mask.toString(2).padStart(s.m, '0')}][${row + 1}] = dp[${rest.toString(2).padStart(s.m, '0')}][${via + 1}] + d(${via + 1}, ${row + 1}) = ${value.toFixed(1)}`
    }
  }

  let chipText = s.lastEvent ?? 'Bellman-Held-Karp — the DP table fills subset by subset'
  let chipClass = "bhk-chip bhk-chip-idle"
  if (s.phase === 'done') chipClass = "bhk-chip bhk-chip-done"
  else if (s.phase === 'readback') chipClass = "bhk-chip bhk-chip-readback"
  else if (chipText.includes('dp[')) chipClass = "bhk-chip bhk-chip-cell"

  return (
    <div className="bhk-root">
      <style>{CSS}</style>

      <header className="bhk-header">
        <div className="bhk-eyebrow">teeline · algorithms/bhk</div>
        <h2 className="bhk-title">Bellman-Held-Karp — exact dynamic programming</h2>
        <p className="bhk-sub">
          BHK fills a table of <strong>subset costs</strong>: <code>dp[mask][i]</code> is the cheapest
          path from city 0 that visits exactly the cities in <code>mask</code> and ends at city{' '}
          <code>i</code>. Every cell is built from one smaller subset plus one edge — then the optimal
          route is <strong>read back</strong> through the recorded predecessors.
        </p>
      </header>

      <div className="bhk-viz-row">
        <div className="bhk-side">
          <div className="bhk-section-label">DP table — rows end city, columns subset</div>
          <DpTable sim={s} selected={selected} onSelect={(mask, row) => setSelected({ mask, row })} />
          <div className="bhk-section-label" style={{ marginTop: 6 }}>Subset on the map</div>
          <CityMap sim={s} />
        </div>
        <div className="bhk-panel">
          <div className="bhk-section-label">Cell info</div>
          <div className="bhk-cellinfo">{cellInfo}</div>

          <div className="bhk-section-label" style={{ marginTop: 10 }}>Optimal</div>
          <div className="bhk-best">
            <span className="bhk-mono">{s.optCost === null ? '— (after the table fills)' : s.route!.join(' → ') + ' = ' + s.optCost.toFixed(1)}</span>
          </div>

          <div className="bhk-section-label" style={{ marginTop: 10 }}>Stats</div>
          <div className="bhk-statgrid">
            <div><div className="bhk-statlabel">subset size</div><div className="bhk-mono">{popcount(currentMask)}</div></div>
            <div><div className="bhk-statlabel">bits set</div><div className="bhk-mono">{currentMask.toString(2).padStart(s.m, '0')}</div></div>
            <div><div className="bhk-statlabel">phase</div><div className="bhk-mono">{s.phase}</div></div>
            <div><div className="bhk-statlabel">step</div><div className="bhk-mono">{s.step}</div></div>
          </div>

          <div className="bhk-section-label" style={{ marginTop: 10 }}>Mode</div>
          <div className="bhk-mode-row">
            <button className={`bhk-mode-btn ${s.phase === 'forward' ? 'bhk-mode-cur' : ''}`} onClick={() => jumpTo('forward')} disabled={running}>Forward</button>
            <button className={`bhk-mode-btn ${s.phase === 'readback' ? 'bhk-mode-cur' : ''}`} onClick={() => jumpTo('readback')} disabled={running}>Read-back</button>
            <button className={`bhk-mode-btn ${s.phase === 'done' ? 'bhk-mode-cur' : ''}`} onClick={() => jumpTo('done')} disabled={running}>Done</button>
          </div>
        </div>
      </div>

      <div className="bhk-legend">
        <span><span className="bhk-swatch bhk-swatch-base" /> base cell</span>
        <span><span className="bhk-swatch bhk-swatch-filled" /> filled</span>
        <span><span className="bhk-swatch bhk-swatch-next" /> next cell</span>
        <span><span className="bhk-swatch bhk-swatch-route" /> optimal route</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="bhk-config">
        <div className="bhk-config-row">
          <span className="bhk-label">Speed</span>
          <div className="bhk-speed-btns">
            {SPEED_LABELS.map((l, i) => (
              <button key={l} className={`bhk-speed-btn ${i === speedIdx ? 'bhk-speed-btn-sel' : ''}`}
                onClick={() => setSpeedIdx(i)} disabled={running}>{l}</button>
            ))}
          </div>
        </div>
      </div>

      <div className="bhk-controls">
        <button className="bhk-btn" onClick={stepBack} disabled={running || s.step === 0}>⏴ Back</button>
        <button className="bhk-btn" onClick={stepForward} disabled={running || s.phase === 'done'}>⏵ Step</button>
        <button className="bhk-btn" onClick={() => setRunning(!running)} disabled={s.phase === 'done'}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="bhk-btn" onClick={() => reinit(scenarioRef.current)} disabled={running}>↺ Reset</button>
      </div>

      <div className="bhk-scenarios">
        <div className="bhk-section-label">Scenarios</div>
        <div className="bhk-scenario-row">
          {Object.entries(SCENARIOS).map(([key, sc]) => (
            <button key={key} className="bhk-scenario-btn" title={sc.desc}
              onClick={() => reinit(sc)} disabled={running}>
              {sc.label}
            </button>
          ))}
        </div>
      </div>

      <footer className="bhk-footer">
        <span className="bhk-mono">cities: {s.n}</span>
        <span className="bhk-mono">{'dp[mask][i] = min_j dp[mask∖{i}][j] + d(j, i)'}</span>
      </footer>
    </div>
  )
}

const CSS = `
.bhk-root {
  --accent: #0d9488;
  --bg: #ffffff;
  --panel: #f6f8fa;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: #656d76;
  font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 20px;
  max-width: 940px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.bhk-root * { box-sizing: border-box; }
.bhk-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.8em;
}

.bhk-header { margin-bottom: 2px; }
.bhk-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.bhk-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.bhk-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }
.bhk-sub code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  background: var(--panel); padding: 1px 4px; border-radius: 4px;
}

.bhk-viz-row { display: flex; gap: 12px; align-items: stretch; }
.bhk-side { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
.bhk-panel { width: 260px; flex-shrink: 0; display: flex; flex-direction: column; }

.bhk-section-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }

.bhk-table-scroll { overflow: auto; max-height: 320px; border: 1px solid var(--line); border-radius: 8px; }
.bhk-table { border-collapse: collapse; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.72rem; }
.bhk-th {
  position: sticky; top: 0; background: var(--panel); color: var(--muted);
  font-weight: 600; padding: 3px 5px; border-bottom: 1px solid var(--line); font-size: 0.66rem;
}
.bhk-th-row { left: 0; text-align: left; }
.bhk-cell {
  padding: 3px 5px; text-align: center; min-width: 40px;
  border-right: 1px solid #eef1f4; cursor: pointer;
}
.bhk-cell:hover { background: #f0fdf4; }
.bhk-cell-empty { color: #cbd5e1; }
.bhk-cell-base { background: #fef9c3; }
.bhk-cell-route { background: #fef3c7; outline: 1px solid #f59e0b; }
.bhk-cell-next { outline: 2px solid var(--accent); }
.bhk-cell-sel { background: #ccfbf1; }

.bhk-map {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.bhk-bg { fill: var(--panel); }
.bhk-map-edge { stroke: #0d9488; stroke-width: 2.5; stroke-linecap: round; }
.bhk-city { fill: #1f2937; stroke: #fff; stroke-width: 1.5; }
.bhk-city-start { fill: #0d9488; stroke: #fff; stroke-width: 1.5; }
.bhk-city-dim { fill: #d1d5db; stroke: #fff; stroke-width: 1.5; }
.bhk-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: #374151; text-anchor: middle;
  paint-order: stroke fill; stroke: white; stroke-width: 3;
  pointer-events: none; user-select: none;
}

.bhk-cellinfo {
  font-size: 0.76rem; padding: 6px 8px; background: var(--panel);
  border-radius: 6px; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  min-height: 48px;
}
.bhk-best { font-size: 0.76rem; padding: 4px 8px; background: #fefce8; border-radius: 6px; }

.bhk-statgrid { display: flex; flex-wrap: wrap; gap: 10px 16px; font-size: 0.8rem; }
.bhk-statlabel { color: var(--muted); font-size: 0.7em; text-transform: uppercase; letter-spacing: 0.06em; }

.bhk-mode-row { display: flex; gap: 6px; }
.bhk-mode-btn {
  flex: 1; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.76rem; padding: 4px 6px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text); cursor: pointer;
}
.bhk-mode-btn:hover:not(:disabled) { background: #f0fdf4; }
.bhk-mode-btn:disabled { opacity: 0.5; cursor: default; }
.bhk-mode-cur { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.bhk-legend {
  display: flex; flex-wrap: wrap; gap: 6px 14px; font-size: 0.78rem; color: var(--muted);
}
.bhk-swatch {
  display: inline-block; width: 14px; height: 3px; border-radius: 2px;
  margin-right: 4px; vertical-align: middle;
}
.bhk-swatch-base { background: #eab308; }
.bhk-swatch-filled { background: #16a34a; }
.bhk-swatch-next { background: #0d9488; }
.bhk-swatch-route { background: #f59e0b; }

.bhk-chip {
  font-size: 0.82rem; padding: 6px 12px; border-radius: 8px;
  font-weight: 500; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.bhk-chip-idle { background: #f1f5f9; color: var(--muted); }
.bhk-chip-cell { background: #ccfbf1; color: #115e59; }
.bhk-chip-readback { background: #fef3c7; color: #92400e; }
.bhk-chip-done { background: #dcfce7; color: #166534; }

.bhk-config {
  display: flex; flex-direction: column; gap: 8px;
  padding: 12px; background: var(--panel); border-radius: 8px;
}
.bhk-config-row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.bhk-label { font-size: 0.82rem; font-weight: 500; color: var(--text); }
.bhk-speed-btns { display: flex; gap: 4px; }
.bhk-speed-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.bhk-speed-btn:hover:not(:disabled) { background: #f0fdf4; }
.bhk-speed-btn:disabled { opacity: 0.4; cursor: default; }
.bhk-speed-btn-sel { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.bhk-controls { display: flex; gap: 8px; }
.bhk-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem; padding: 6px 14px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.bhk-btn:hover:not(:disabled) { background: #f0fdf4; }
.bhk-btn:disabled { opacity: 0.4; cursor: default; }

.bhk-scenarios { display: flex; flex-direction: column; gap: 6px; }
.bhk-scenario-row { display: flex; flex-wrap: wrap; gap: 6px; }
.bhk-scenario-btn {
  font-size: 0.8rem; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.bhk-scenario-btn:hover:not(:disabled) { background: #f0fdf4; border-color: var(--accent); }
.bhk-scenario-btn:disabled { opacity: 0.4; cursor: default; }

.bhk-footer {
  display: flex; gap: 12px; justify-content: center;
  padding-top: 8px; border-top: 1px solid var(--line);
  color: var(--muted); font-size: 0.78rem;
}
`

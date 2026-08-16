import { useState, useRef, useEffect, useCallback, useMemo } from "preact/hooks"
import {
  SCENARIOS, makeInitState, stepOnce,
} from "./branch-bound-algo"
import type { Scenario, SimState, BnBNode } from "./branch-bound-algo"

const DEFAULT_SCENARIO = SCENARIOS.small_grid
const SPEEDS = [600, 420, 280, 180, 100, 50, 25]
const SPEED_LABELS = ["1x", "2x", "3x", "4x", "5x", "6x", "7x"]

// ---------------------------------------------------------------
// SearchTree — horizontal tree: root on the left, children to the
// right. Pruned nodes red with a strike, best leaf gold, the active
// path green; clicking a node pins it in the info panel.
// ---------------------------------------------------------------
function SearchTree({ sim, selectedId, onSelect }: {
  sim: SimState
  selectedId: number | null
  onSelect: (id: number) => void
}) {
  const X = 78
  const Y = 16

  const { nodes, stack } = sim
  const onStack = useMemo(() => new Set(stack), [stack])
  const maxDepth = useMemo(() => nodes.reduce((m, nd) => Math.max(m, nd.depth), 1), [nodes])
  const width = maxDepth * X + 70
  const height = Math.max(40, nodes.length * Y + 24)

  return (
    <div className="bb-tree-scroll">
      <svg viewBox={`0 0 ${width} ${height}`} className="bb-tree" role="img" aria-label="Branch and bound search tree">
        {/* edges */}
        {nodes.map((nd) => {
          if (nd.parent === null) return null
          const p = nodes[nd.parent]
          const x1 = p.depth * X
          const y1 = p.id * Y + Y / 2
          const x2 = nd.depth * X - 6
          const y2 = nd.id * Y + Y / 2
          const cls = `bb-edge ${onStack.has(nd.id) ? 'bb-edge-active' : ''}`
          return <line key={`e${nd.id}`} className={cls} x1={x1} y1={y1} x2={x2} y2={y2} />
        })}
        {/* nodes */}
        {nodes.map((nd) => {
          const x = nd.depth * X
          const y = nd.id * Y
          const cls = [
            'bb-node',
            nd.status === 'pruned' ? 'bb-node-pruned' : '',
            nd.status === 'best' ? 'bb-node-best' : '',
            nd.status === 'leaf' ? 'bb-node-leaf' : '',
            onStack.has(nd.id) && nd.status === 'open' ? 'bb-node-open' : '',
            nd.id === sim.current ? 'bb-node-current' : '',
            selectedId === nd.id ? 'bb-node-selected' : '',
          ].join(' ').trim()
          const label = nd.path[nd.path.length - 1]
          return (
            <g key={nd.id} className={cls} onClick={() => onSelect(nd.id)}>
              <rect x={x - 24} y={y} width={48} height={Y - 3} rx={3} />
              <text x={x} y={y + 11} className="bb-node-city">{label}</text>
              <text x={x + 6} y={y + 11} className="bb-node-lb">{nd.lb.toFixed(0)}</text>
              {nd.status === 'pruned' && <text x={x + 22} y={y + 8} className="bb-node-x">✕</text>}
              <title>{`path ${nd.path.join('→')} · bound ${nd.lb.toFixed(0)} · ${nd.status}`}</title>
            </g>
          )
        })}
      </svg>
    </div>
  )
}

// ---------------------------------------------------------------
// CityMap — partial tour of the pinned/active node, unvisited gray
// ---------------------------------------------------------------
function CityMap({ sim, node }: { sim: SimState; node: BnBNode | null }) {
  const path = node ? node.path : sim.bestTour ?? []
  const unvisited = node ? new Set(node.unvisited) : new Set<number>()
  const n = sim.n
  const edges: Array<[number, number]> = []
  for (let k = 0; k < path.length - 1; k++) edges.push([path[k], path[k + 1]])

  return (
    <svg viewBox="0 0 300 300" className="bb-map" role="img" aria-label="Partial tour at the selected node">
      <rect x={0} y={0} width={300} height={300} className="bb-bg" />
      {edges.map(([a, b]) => (
        <line key={`${a}-${b}`} className="bb-map-edge"
          x1={sim.cities[a][0]} y1={sim.cities[a][1]}
          x2={sim.cities[b][0]} y2={sim.cities[b][1]} />
      ))}
      {Array.from({ length: n }, (_, i) => i).map((i) => {
        const inPath = path.includes(i)
        let cls = 'bb-city-gray'
        if (inPath) cls = 'bb-city'
        else if (unvisited.has(i)) cls = 'bb-city-unvisited'
        return (
          <g key={i}>
            <circle className={cls} cx={sim.cities[i][0]} cy={sim.cities[i][1]} r={7} />
            <text className="bb-label" x={sim.cities[i][0]} y={sim.cities[i][1] + 16}>{i}</text>
          </g>
        )
      })}
    </svg>
  )
}

// ---------------------------------------------------------------
// Root component
// ---------------------------------------------------------------
export default function BranchBoundExplainer() {
  const [sim, setSim] = useState<SimState>(() => makeInitState(DEFAULT_SCENARIO))
  const [running, setRunning] = useState(false)
  const [speedIdx, setSpeedIdx] = useState(2)
  const [selectedId, setSelectedId] = useState<number | null>(null)

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
    setSelectedId(null)
    commit(makeInitState(scenario))
    setRunning(false)
  }, [commit])

  const stepForward = useCallback(() => {
    const cur = simRef.current
    if (cur.phase === 'done') return
    historyRef.current.push(structuredClone(cur))
    const next = stepOnce(cur)
    commit(next)
    if (next.phase === 'done') setRunning(false)
  }, [commit])

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

  const s = sim
  const selectedNode = selectedId !== null ? s.nodes.find((nd) => nd.id === selectedId) ?? null : null
  const activeNode = s.current !== null ? s.nodes[s.current] : null
  const infoNode = selectedNode ?? activeNode

  // Status chip
  let chipText = s.lastEvent ?? 'Branch & Bound — the search tree grows as we step'
  let chipClass = "bb-chip bb-chip-idle"
  if (s.phase === 'done') {
    chipText = `Done — optimal tour ${s.bestTour?.join('→') ?? '—'} costs ${s.bestCost?.toFixed(0) ?? '—'} (${s.nodes.length} nodes, ${s.pruned} pruned)`
    chipClass = "bb-chip bb-chip-done"
  } else if (chipText.startsWith('New best')) {
    chipClass = "bb-chip bb-chip-best"
  } else if (chipText.startsWith('Pruned')) {
    chipClass = "bb-chip bb-chip-pruned"
  } else if (chipText.startsWith('Expanded')) {
    chipClass = "bb-chip bb-chip-open"
  } else if (chipText.startsWith('Leaf')) {
    chipClass = "bb-chip bb-chip-leaf"
  }

  return (
    <div className="bb-root">
      <style>{CSS}</style>

      <header className="bb-header">
        <div className="bb-eyebrow">teeline · algorithms/branch_bound</div>
        <h2 className="bb-title">Branch &amp; Bound — exact search with pruning</h2>
        <p className="bb-sub">
          B&amp;B explores the tree of partial tours. At every node the <strong>lower bound</strong>
          ({'partial cost + MST(start ∪ remaining)'}) is compared with the <strong>best complete
          tour</strong> found so far — any branch whose bound cannot beat it is <strong>pruned</strong>.
          The bound is valid, so the surviving leaf is the exact optimum.
        </p>
      </header>

      <div className="bb-viz-row">
        <div className="bb-side">
          <div className="bb-section-label">Search tree</div>
          <SearchTree sim={s} selectedId={selectedId} onSelect={setSelectedId} />
          <div className="bb-section-label bb-mt6">Tour at selected node</div>
          <CityMap sim={s} node={infoNode} />
        </div>
        <div className="bb-panel">
          <div className="bb-section-label">Node info</div>
          <div className="bb-nodeinfo">
            {infoNode ? (
              <>
                <div className="bb-nodeinfo-row"><span>path</span><span className="bb-mono">{infoNode.path.join(' → ')}</span></div>
                <div className="bb-nodeinfo-row"><span>bound</span><span className="bb-mono">{infoNode.lb.toFixed(1)}</span></div>
                <div className="bb-nodeinfo-row"><span>status</span><span className="bb-mono">{infoNode.status}</span></div>
                <div className="bb-nodeinfo-note">
                  bound = partial {infoNode.cost.toFixed(1)} + MST(start ∪ {infoNode.unvisited.join(',')})
                </div>
                {infoNode.status === 'pruned' && selectedId === infoNode.id && (
                  <div className="bb-nodeinfo-why">Pruned because {infoNode.lb.toFixed(1)} ≥ best {s.bestCost?.toFixed(1) ?? '—'} — no tour below this branch can win.</div>
                )}
              </>
            ) : (
              <div className="bb-nodeinfo-row">click a tree node to inspect it</div>
            )}
          </div>

          <div className="bb-section-label bb-mt10">Best tour</div>
          <div className="bb-best">
            <span className="bb-mono">{s.bestCost === null ? '— (none yet)' : s.bestTour!.join(' → ') + ' = ' + s.bestCost.toFixed(1)}</span>
          </div>

          <div className="bb-section-label bb-mt10">Stats</div>
          <div className="bb-statgrid">
            <div><div className="bb-statlabel">nodes</div><div className="bb-mono">{s.nodes.length}</div></div>
            <div><div className="bb-statlabel">explored</div><div className="bb-mono">{s.explored}</div></div>
            <div><div className="bb-statlabel">pruned</div><div className="bb-mono">{s.pruned}</div></div>
            <div><div className="bb-statlabel">leaves</div><div className="bb-mono">{s.leaves}</div></div>
            <div><div className="bb-statlabel">best</div><div className="bb-mono">{s.bestCost === null ? '—' : s.bestCost.toFixed(0)}</div></div>
            <div><div className="bb-statlabel">step</div><div className="bb-mono">{s.step}</div></div>
          </div>
        </div>
      </div>

      <div className="bb-legend">
        <span><span className="bb-swatch bb-swatch-open" /> open</span>
        <span><span className="bb-swatch bb-swatch-active" /> active path</span>
        <span><span className="bb-swatch bb-swatch-pruned" /> pruned</span>
        <span><span className="bb-swatch bb-swatch-leaf" /> leaf</span>
        <span><span className="bb-swatch bb-swatch-best" /> new best</span>
      </div>

      <div className={chipClass}>{chipText}</div>

      <div className="bb-config">
        <div className="bb-config-row">
          <span className="bb-label">Speed</span>
          <div className="bb-speed-btns">
            {SPEED_LABELS.map((l, i) => (
              <button key={l} className={`bb-speed-btn ${i === speedIdx ? 'bb-speed-btn-sel' : ''}`}
                onClick={() => setSpeedIdx(i)} disabled={running}>{l}</button>
            ))}
          </div>
        </div>
      </div>

      <div className="bb-controls">
        <button className="bb-btn" onClick={stepBack} disabled={running || s.step === 0}>⏴ Back</button>
        <button className="bb-btn" onClick={stepForward} disabled={running || s.phase === 'done'}>⏵ Step</button>
        <button className="bb-btn" onClick={() => setRunning(!running)} disabled={s.phase === 'done'}>
          {running ? "⏸ Pause" : "▶ Run"}
        </button>
        <button className="bb-btn" onClick={() => reinit(scenarioRef.current)} disabled={running}>↺ Reset</button>
      </div>

      <div className="bb-scenarios">
        <div className="bb-section-label">Scenarios</div>
        <div className="bb-scenario-row">
          {Object.entries(SCENARIOS).map(([key, sc]) => (
            <button key={key} className="bb-scenario-btn" title={sc.desc}
              onClick={() => reinit(sc)} disabled={running}>
              {sc.label}
            </button>
          ))}
        </div>
      </div>

      <footer className="bb-footer">
        <span className="bb-mono">cities: {s.n}</span>
        <span className="bb-mono">bound = partial + MST(start ∪ remaining) · prune when bound ≥ best</span>
      </footer>
    </div>
  )
}

const CSS = `
.bb-root {
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
  max-width: 900px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.bb-root * { box-sizing: border-box; }
.bb-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.8em;
}

.bb-header { margin-bottom: 2px; }
.bb-eyebrow {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.75rem; letter-spacing: 0.08em; text-transform: uppercase;
  color: var(--accent); margin-bottom: 6px;
}
.bb-title { font-size: 1.25rem; font-weight: 650; margin: 0 0 6px; line-height: 1.3; }
.bb-sub { margin: 0; color: var(--muted); font-size: 0.88rem; line-height: 1.55; }

.bb-viz-row { display: flex; gap: 12px; align-items: stretch; }
.bb-side { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
.bb-panel { width: 250px; flex-shrink: 0; display: flex; flex-direction: column; }

.bb-section-label { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }
.bb-mt6 { margin-top: 6px; }
.bb-mt10 { margin-top: 10px; }

.bb-tree-scroll {
  overflow: auto; max-height: 300px;
  border: 1px solid var(--line); border-radius: 8px; background: var(--panel);
}
.bb-tree { display: block; }
.bb-edge { stroke: #cbd5e1; stroke-width: 1.5; }
.bb-edge-active { stroke: #0d9488; }
.bb-node { cursor: pointer; }
.bb-node rect { fill: #e2e8f0; stroke: #94a3b8; stroke-width: 1; }
.bb-node-open rect { fill: #ccfbf1; stroke: var(--accent); }
.bb-node-current rect { stroke: #0d9488; stroke-width: 2.5; }
.bb-node-pruned rect { fill: #fee2e2; stroke: #ef4444; }
.bb-node-best rect { fill: #fef3c7; stroke: #f59e0b; stroke-width: 2; }
.bb-node-leaf rect { fill: #dbeafe; stroke: #60a5fa; }
.bb-node-selected rect { stroke: #1f2328; stroke-width: 2; }
.bb-node-city {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; font-weight: 700; fill: var(--text); text-anchor: middle;
}
.bb-node-lb { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 8px; fill: var(--muted); }
.bb-node-x { font-size: 9px; fill: #ef4444; font-weight: 700; }
.bb-node-pruned .bb-node-city { text-decoration: line-through; }

.bb-map {
  width: 100%; display: block; border-radius: 8px;
  border: 1px solid var(--line);
}
.bb-bg { fill: var(--panel); }
.bb-map-edge { stroke: #0d9488; stroke-width: 2.5; stroke-linecap: round; }
.bb-city { fill: #1f2937; stroke: #fff; stroke-width: 1.5; }
.bb-city-unvisited { fill: #9ca3af; stroke: #fff; stroke-width: 1.5; }
.bb-city-gray { fill: #1f2937; stroke: #fff; stroke-width: 1.5; opacity: 0.5; }
.bb-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 9px; fill: #374151; text-anchor: middle;
  paint-order: stroke fill; stroke: white; stroke-width: 3;
  pointer-events: none; user-select: none;
}

.bb-nodeinfo { display: flex; flex-direction: column; gap: 4px; }
.bb-nodeinfo-row {
  display: flex; justify-content: space-between; gap: 8px;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 6px; background: var(--panel);
}
.bb-nodeinfo-note { font-size: 0.72rem; color: var(--muted); padding: 0 4px; }
.bb-nodeinfo-why {
  font-size: 0.74rem; color: #991b1b; background: #fee2e2;
  border-radius: 6px; padding: 6px 8px;
}
.bb-best { font-size: 0.78rem; padding: 4px 8px; background: #fefce8; border-radius: 6px; }

.bb-statgrid { display: flex; flex-wrap: wrap; gap: 10px 16px; font-size: 0.8rem; }
.bb-statlabel { color: var(--muted); font-size: 0.7em; text-transform: uppercase; letter-spacing: 0.06em; }

.bb-legend {
  display: flex; flex-wrap: wrap; gap: 6px 14px; font-size: 0.78rem; color: var(--muted);
}
.bb-swatch {
  display: inline-block; width: 12px; height: 12px; border-radius: 3px;
  margin-right: 4px; vertical-align: middle; border: 1px solid #94a3b8;
}
.bb-swatch-open { background: #ccfbf1; }
.bb-swatch-active { background: #0d9488; }
.bb-swatch-pruned { background: #fee2e2; border-color: #ef4444; }
.bb-swatch-leaf { background: #dbeafe; border-color: #60a5fa; }
.bb-swatch-best { background: #fef3c7; border-color: #f59e0b; }

.bb-chip {
  font-size: 0.82rem; padding: 6px 12px; border-radius: 8px;
  font-weight: 500; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.bb-chip-idle { background: #f1f5f9; color: var(--muted); }
.bb-chip-open { background: #ccfbf1; color: #115e59; }
.bb-chip-pruned { background: #fee2e2; color: #991b1b; }
.bb-chip-best { background: #fef3c7; color: #92400e; }
.bb-chip-leaf { background: #dbeafe; color: #1e40af; }
.bb-chip-done { background: #dcfce7; color: #166534; }

.bb-config {
  display: flex; flex-direction: column; gap: 8px;
  padding: 12px; background: var(--panel); border-radius: 8px;
}
.bb-config-row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.bb-label { font-size: 0.82rem; font-weight: 500; color: var(--text); }
.bb-speed-btns { display: flex; gap: 4px; }
.bb-speed-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem; padding: 3px 8px; border-radius: 5px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.bb-speed-btn:hover:not(:disabled) { background: #f0fdf4; }
.bb-speed-btn:disabled { opacity: 0.4; cursor: default; }
.bb-speed-btn-sel { background: #ccfbf1; border-color: var(--accent); color: #115e59; font-weight: 600; }

.bb-controls { display: flex; gap: 8px; }
.bb-btn {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85rem; padding: 6px 14px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.bb-btn:hover:not(:disabled) { background: #f0fdf4; }
.bb-btn:disabled { opacity: 0.4; cursor: default; }

.bb-scenarios { display: flex; flex-direction: column; gap: 6px; }
.bb-scenario-row { display: flex; flex-wrap: wrap; gap: 6px; }
.bb-scenario-btn {
  font-size: 0.8rem; padding: 5px 12px; border-radius: 6px;
  border: 1px solid var(--line); background: var(--bg); color: var(--text);
  cursor: pointer;
}
.bb-scenario-btn:hover:not(:disabled) { background: #f0fdf4; border-color: var(--accent); }
.bb-scenario-btn:disabled { opacity: 0.4; cursor: default; }

.bb-footer {
  display: flex; gap: 12px; justify-content: center;
  padding-top: 8px; border-top: 1px solid var(--line);
  color: var(--muted); font-size: 0.78rem;
}
`

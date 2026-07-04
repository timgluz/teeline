// Plain JS (no TS syntax) so it can be imported both by vite.config.ts (browser bundle)
// and by scripts/build-docs.mjs (a plain Node script run before Vite even starts).

export const SOLVER_META = {
  'bhk':          { id: 'bhk',          name: 'Bellman-Held-Karp' },
  'branch_bound': { id: 'branch_bound', name: 'Branch & Bound' },
  'nn':           { id: 'nn',           name: 'Nearest Neighbor' },
  'fourier':      { id: 'fourier',      name: 'Fourier' },
  'christofides': { id: 'christofides', name: 'Christofides' },
  '2opt':         { id: '2opt',         name: '2-opt' },
  '3opt':         { id: '3opt',         name: '3-opt' },
  'or_opt':       { id: 'or_opt',       name: 'Or-opt' },
  'lk':           { id: 'lk',           name: 'Lin-Kernighan' },
  'sa':           { id: 'sa',           name: 'Simulated Annealing' },
  'tabu':         { id: 'tabu',         name: 'Tabu Search' },
  'ga':           { id: 'ga',           name: 'Genetic Algorithm' },
  'pso':          { id: 'pso',          name: 'Particle Swarm' },
  'cs':           { id: 'cs',           name: 'Cuckoo Search' },
  'fpa':          { id: 'fpa',          name: 'Flower Pollination' },
  'gsa':          { id: 'gsa',          name: 'Gravitational Search' },
  'som':          { id: 'som',          name: 'Kohonen SOM' },
}

export const SOLVER_GROUPS = [
  { label: 'Exact',         ids: ['bhk', 'branch_bound'] },
  { label: 'Constructive',  ids: ['nn', 'fourier', 'christofides', 'som'] },
  { label: 'Local search',  ids: ['2opt', '3opt', 'or_opt', 'lk'] },
  { label: 'Metaheuristic', ids: ['sa', 'tabu', 'ga', 'pso', 'cs', 'fpa', 'gsa'] },
]

export const PAGED_SOLVERS = new Set([
  'bhk', 'branch_bound', 'nn', 'fourier', 'christofides', 'som',
  '2opt', '3opt', 'or_opt', 'lk', 'sa', 'tabu', 'ga', 'pso', 'cs', 'fpa', 'gsa',
])

export function renderSidebarHtml(currentId) {
  const groups = SOLVER_GROUPS.map(g => {
    const items = g.ids.map(id => {
      const meta = SOLVER_META[id]
      if (!meta) return ''
      const isCurrent = id === currentId
      const inner = isCurrent
        ? `<span aria-current="page">${meta.name}</span>`
        : PAGED_SOLVERS.has(id)
          ? `<a href="/algorithms/${id}/">${meta.name}</a>`
          : `<span>${meta.name}</span>`
      const currentClass = isCurrent ? ' sidebar-item--current' : ''
      return `<li class="sidebar-item${currentClass}">${inner}</li>`
    }).join('')
    return `
      <li class="sidebar-group">
        <span class="sidebar-group-label">${g.label}</span>
        <ul>${items}</ul>
      </li>`
  }).join('')
  return `<ul>${groups}</ul>`
}

export function renderTopbarHtml() {
  const dropdownItems = SOLVER_GROUPS.map(g => {
    const label = `<li class="algo-group-label"><strong>${g.label}</strong></li>`
    const items = g.ids.map(id => {
      const meta = SOLVER_META[id]
      if (!meta) return ''
      const inner = PAGED_SOLVERS.has(id)
        ? `<a href="/algorithms/${id}/">${meta.name}</a>`
        : `<span>${meta.name}</span>`
      return `<li>${inner}</li>`
    }).join('')
    return label + items
  }).join('')

  return `
    <nav class="topbar" aria-label="Site navigation">
      <ul>
        <li><a class="topbar-logo" href="/">teeline</a></li>
      </ul>
      <ul>
        <li>
          <details class="dropdown">
            <summary>Algorithms</summary>
            <ul>
              ${dropdownItems}
            </ul>
          </details>
        </li>
        <li>
          <a href="https://github.com/timgluz/teeline"
             target="_blank"
             rel="noopener noreferrer"
          >GitHub ↗</a>
        </li>
      </ul>
    </nav>`
}

// Plain JS (no TS syntax) — imported by vite.config.ts's transformIndexHtml plugin
// to server-render the homepage's "Explore the algorithms" card grid at build time.
import { SOLVER_GROUPS, SOLVER_META, PAGED_SOLVERS } from './nav-html.mjs'

// Icons are small line-drawings of what each family of algorithm actually does,
// not decoration: a bullseye for guaranteed-optimal, a rising path for building a
// tour step by step, crossed edges for the swap 2-opt et al. repeatedly undo, and
// scattered points for the randomized search metaheuristics perform.
const GROUP_INFO = {
  'Exact': {
    slug: 'exact',
    description: 'Explores the full search space to guarantee the shortest possible tour — only practical for a few dozen cities.',
    icon: '<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="10" cy="10" r="7"/><circle cx="10" cy="10" r="2.25" fill="currentColor" stroke="none"/></svg>',
  },
  'Constructive': {
    slug: 'constructive',
    description: 'Builds a tour in a single pass with a greedy rule or geometric heuristic — fast, but rarely optimal on its own.',
    icon: '<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 15 L9.5 9 L16 5"/><circle cx="4" cy="15" r="1.4" fill="currentColor" stroke="none"/><circle cx="9.5" cy="9" r="1.4" fill="currentColor" stroke="none"/><circle cx="16" cy="5" r="1.4" fill="currentColor" stroke="none"/></svg>',
  },
  'Local search': {
    slug: 'local-search',
    description: 'Starts from a complete tour and keeps swapping edges until no swap can shorten it any further.',
    icon: '<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3.5 4 L16.5 16"/><path d="M16.5 4 L3.5 16"/></svg>',
  },
  'Metaheuristic': {
    slug: 'metaheuristic',
    description: 'Searches broadly with randomness or swarm behavior to escape the local optima that trap local search.',
    icon: '<svg viewBox="0 0 20 20" fill="currentColor" stroke="none"><circle cx="4" cy="5.5" r="1.5"/><circle cx="14.5" cy="4" r="1.2"/><circle cx="16" cy="12.5" r="1.5"/><circle cx="7.5" cy="15.5" r="1.2"/><circle cx="3.5" cy="13" r="1"/><circle cx="10" cy="8.5" r="1.7"/></svg>',
  },
}

export function renderAlgorithmCardsHtml() {
  const cards = SOLVER_GROUPS.map(g => {
    const info = GROUP_INFO[g.label]
    const links = g.ids.map(id => {
      const meta = SOLVER_META[id]
      if (!meta) return ''
      return PAGED_SOLVERS.has(id)
        ? `<li><a href="/algorithms/${id}/">${meta.name}</a></li>`
        : `<li><span>${meta.name}</span></li>`
    }).join('')
    return `
      <article class="card card--${info.slug}">
        <div class="card-icon" aria-hidden="true">${info.icon}</div>
        <h3 class="card-title">${g.label}</h3>
        <p class="card-desc">${info.description}</p>
        <ul class="card-links">${links}</ul>
      </article>`
  }).join('')
  return `<div class="card-grid card-grid--2col">${cards}</div>`
}

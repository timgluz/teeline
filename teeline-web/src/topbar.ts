import { SOLVER_GROUPS, SOLVER_META, PAGED_SOLVERS } from './solver-groups'

export function renderTopbarHtml(): string {
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

export function initTopbar(containerId = 'topbar'): void {
  const el = document.getElementById(containerId)
  if (!el) return
  el.innerHTML = renderTopbarHtml()

  // Close the dropdown when clicking outside the topbar
  document.addEventListener('click', (e) => {
    const details = el.querySelector<HTMLDetailsElement>('details.dropdown')
    if (details && !el.contains(e.target as Node)) {
      details.removeAttribute('open')
    }
  })
}

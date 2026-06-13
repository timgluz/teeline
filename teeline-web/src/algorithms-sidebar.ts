import { SOLVER_GROUPS, SOLVER_META, PAGED_SOLVERS } from './solver-groups'

export function renderSidebarHtml(currentId: string | null): string {
  return SOLVER_GROUPS.map(g => {
    const items = g.ids.map(id => {
      const meta = SOLVER_META[id]
      if (!meta) return ''
      const isCurrent = id === currentId
      const inner = isCurrent
        ? `<span aria-current="page">${meta.name}</span>`
        : PAGED_SOLVERS.has(id)
          ? `<a href="/algorithms/${id}">${meta.name}</a>`
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
}

export function initSidebar(containerId = 'algo-sidebar'): void {
  const el = document.getElementById(containerId)
  if (!el) return
  const match = window.location.pathname.match(/^\/algorithms\/([^/]+)/)
  const currentId = match ? match[1] : null
  el.innerHTML = renderSidebarHtml(currentId)
}

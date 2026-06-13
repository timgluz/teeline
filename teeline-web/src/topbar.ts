import { SOLVER_GROUPS, SOLVER_META, PAGED_SOLVERS } from './solver-groups'

export function renderTopbarHtml(): string {
  const groupsHtml = SOLVER_GROUPS.map(g => {
    const items = g.ids.map(id => {
      const meta = SOLVER_META[id]
      if (!meta) return ''
      const inner = PAGED_SOLVERS.has(id)
        ? `<a href="/algorithms/${id}">${meta.name}</a>`
        : `<span>${meta.name}</span>`
      return `<li>${inner}</li>`
    }).join('')
    return `
      <li class="algo-group">
        <span class="algo-group-label">${g.label}</span>
        <ul>${items}</ul>
      </li>`
  }).join('')

  return `
    <nav class="topbar" aria-label="Site navigation">
      <a class="topbar-logo" href="/">teeline</a>
      <div class="topbar-links">
        <div class="topbar-dropdown">
          <button
            class="algo-menu-toggle"
            aria-haspopup="true"
            aria-expanded="false"
            aria-controls="algo-menu"
          >Algorithms ▾</button>
          <ul class="algo-menu" id="algo-menu" hidden role="menu">
            ${groupsHtml}
          </ul>
        </div>
        <a
          class="topbar-github"
          href="https://github.com/timgluz/teeline"
          target="_blank"
          rel="noopener noreferrer"
        >GitHub ↗</a>
      </div>
    </nav>`
}

export function initTopbar(containerId = 'topbar'): void {
  const el = document.getElementById(containerId)
  if (!el) return
  el.innerHTML = renderTopbarHtml()

  const toggle = el.querySelector<HTMLButtonElement>('.algo-menu-toggle')
  const menu   = el.querySelector<HTMLElement>('.algo-menu')
  if (!toggle || !menu) return

  toggle.addEventListener('click', () => {
    const isOpen = !menu.hidden
    menu.hidden = isOpen
    toggle.setAttribute('aria-expanded', String(!isOpen))
  })

  document.addEventListener('click', (e) => {
    if (!el.contains(e.target as Node)) {
      menu.hidden = true
      toggle.setAttribute('aria-expanded', 'false')
    }
  })
}

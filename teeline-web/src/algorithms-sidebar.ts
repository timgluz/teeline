import { renderSidebarHtml } from './nav-html.mjs'

export { renderSidebarHtml }

export function initSidebar(containerId = 'algo-sidebar'): void {
  const el = document.getElementById(containerId)
  if (!el) return
  const match = window.location.pathname.match(/^\/algorithms\/([^/]+)/)
  const currentId = match ? match[1] : null
  el.innerHTML = renderSidebarHtml(currentId)
}

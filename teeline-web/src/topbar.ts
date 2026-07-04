import { renderTopbarHtml } from './nav-html.mjs'

export { renderTopbarHtml }

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

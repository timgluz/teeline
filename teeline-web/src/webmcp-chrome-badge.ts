export function initWebMCPChromeBadge(): void {
  const badge = document.getElementById('webmcp-chrome-badge')
  if (!badge) return
  const match = navigator.userAgent.match(/Chrome\/(\d+)/)
  if (!match) return
  const major = parseInt(match[1], 10)
  if (major >= 149) {
    badge.textContent = ` ✓ Chrome ${major} detected`
    badge.className = 'pico-color-green-500'
  } else {
    badge.textContent = ` (Chrome ${major} — requires ≥ 149)`
    badge.className = 'pico-color-orange-500'
  }
}

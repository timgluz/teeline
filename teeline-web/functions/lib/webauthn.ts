// WebAuthn configuration + origin policy shared by the auth handlers.
//
// rpID rules (WebAuthn): rpID must be a suffix of the caller's effective
// domain. Production is tspsolver.com; local dev runs on localhost, which is
// its own valid rpID. The expectedOrigin for verification is the request's
// own origin — but only after it passes the allowlist, which doubles as the
// CSRF Origin check on mutating endpoints.
import type { Env } from './env'

export const RP_NAME = 'Teeline'
export const CHALLENGE_TTL_MS = 5 * 60 * 1000

export function rpIdFor(hostname: string): string {
  const h = hostname.toLowerCase()
  if (h === 'tspsolver.com' || h.endsWith('.tspsolver.com')) return 'tspsolver.com'
  return h
}

const LOCAL_DEV_HOSTS = new Set(['localhost', '127.0.0.1', '[::1]'])

export function isAllowedOrigin(origin: string, env: Env): boolean {
  if (origin === 'https://tspsolver.com') return true
  const extra = env.ALLOWED_ORIGINS?.split(',').map((s) => s.trim()).filter(Boolean) ?? []
  if (extra.includes(origin)) return true
  // Local-dev convenience: any http://localhost / 127.0.0.1 origin. Only the
  // user's own machine can produce these, so it's safe to allow by default.
  try {
    const u = new URL(origin)
    if (u.protocol === 'http:' && LOCAL_DEV_HOSTS.has(u.hostname)) return true
  } catch {
    /* not a URL — not allowed */
  }
  return false
}

export function requestOrigin(request: Request): string {
  return new URL(request.url).origin
}

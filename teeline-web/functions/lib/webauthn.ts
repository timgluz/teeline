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

export function isAllowedOrigin(origin: string, env: Env): boolean {
  if (origin === 'https://tspsolver.com') return true
  const extra = env.ALLOWED_ORIGINS?.split(',').map((s) => s.trim()).filter(Boolean) ?? []
  // Local dev origins (http://localhost / 127.0.0.1) are allowed only when
  // explicitly listed in ALLOWED_ORIGINS (e.g. via .dev.vars) — no blanket
  // allowance, so a misconfigured shared/staging environment can't inherit it.
  return extra.includes(origin)
}

export function requestOrigin(request: Request): string {
  return new URL(request.url).origin
}

// WebAuthn configuration + origin policy shared by the auth handlers.
//
// rpID rules (WebAuthn): rpID must be a suffix of the caller's effective
// domain. Production is tspsolver.com; local dev runs on localhost, which is
// its own valid rpID.
//
// Two distinct origins matter:
// - serverOrigin(request) — the request's destination origin. This is what
//   WebAuthn's expectedOrigin must be (the browser records it in
//   clientDataJSON), so it is used by the *complete* handlers for
//   verification.
// - clientOrigin(request) — the Origin/Referer header, i.e. where the
//   request actually came from. This is the CSRF check on mutating
//   endpoints: a cross-site POST carries the attacker's origin (or none),
//   which the allowlist rejects.
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

/** Destination origin of the request — used for WebAuthn expectedOrigin. */
export function serverOrigin(request: Request): string {
  return new URL(request.url).origin
}

/**
 * Origin the request claims to come from (Origin header, falling back to
 * Referer). Used as the CSRF check: returns null when neither header is
 * present, which mutating endpoints must reject.
 */
export function clientOrigin(request: Request): string | null {
  const origin = request.headers.get('Origin')
  if (origin) return origin
  const referer = request.headers.get('Referer')
  if (referer) {
    try {
      return new URL(referer).origin
    } catch {
      return null
    }
  }
  return null
}

/** True when the request's client origin is present and on the allowlist. */
export function isClientOriginAllowed(request: Request, env: Env): boolean {
  const origin = clientOrigin(request)
  return origin !== null && isAllowedOrigin(origin, env)
}

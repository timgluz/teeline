// Fixed-window per-IP rate limiter backed by D1.
//
// The window is derived from the clock, so a request that lands in a new
// window starts a fresh counter. The increment + read run as one D1 batch
// (atomic), so the count we read includes this request's increment — no
// TOCTOU gap that could allow a bypass or cause false rejections.
//
// Failure mode: the limiter is a SECONDARY defense (auth endpoints already
// require WebAuthn/session verification), so a D1 error fails OPEN (request
// allowed) rather than locking everyone out during an outage.
import type { Env } from './env'
import { json } from './http'

export interface RateLimitResult {
  allowed: boolean
  retryAfterSeconds: number
}

export async function checkRateLimit(
  db: D1Database,
  ip: string,
  scope: string,
  limit: number,
  windowMs: number,
  now: number = Date.now(),
): Promise<RateLimitResult> {
  const windowStart = Math.floor(now / windowMs) * windowMs
  const key = `${scope}:${ip}:${windowStart}`

  // Atomic: the SELECT runs in the same transaction as the increment.
  const results = await db.batch([
    db
      .prepare(
        `INSERT INTO rate_limits (key, count, window_start) VALUES (?1, 1, ?2)
         ON CONFLICT(key) DO UPDATE SET count = count + 1`,
      )
      .bind(key, windowStart),
    db.prepare('SELECT count FROM rate_limits WHERE key = ?1').bind(key),
  ])

  const row = (results[1] as { results: { count: number }[] }).results[0] ?? undefined
  if (!row) {
    // INSERT succeeded but the row isn't visible — unexpected; don't mask it.
    console.warn('[rate-limit] INSERT succeeded but SELECT returned no row for key:', key)
  }
  const count = row?.count ?? 1
  return {
    allowed: count <= limit,
    retryAfterSeconds: Math.max(1, Math.ceil((windowStart + windowMs - now) / 1000)),
  }
}

/** Opportunistic cleanup of expired windows — call occasionally, never block. */
export async function cleanupRateLimits(db: D1Database, now: number = Date.now()): Promise<void> {
  await db.prepare('DELETE FROM rate_limits WHERE window_start < ?1').bind(now - 24 * 3600 * 1000).run()
}

/**
 * Enforces a per-IP limit for the calling handler. Returns a 429 Response
 * when the limit is exceeded, otherwise null (proceed). Fails open (null)
 * when the limiter itself errors — see the module comment.
 */
export async function rateLimit(
  request: Request,
  env: Env,
  scope: string,
  limit: number,
  windowMs: number = 60_000,
): Promise<Response | null> {
  const ip = request.headers.get('CF-Connecting-IP')
  if (!ip) {
    // Cloudflare always sets this header at the edge; its absence means the
    // request didn't come through CF — refuse rather than collapse all such
    // requests into one shared bucket (a trivial DoS vector).
    return json({ error: 'Unable to determine client IP' }, 400)
  }

  // ~1% of requests also sweep stale windows, regardless of allow/deny.
  if (Math.random() < 0.01) {
    cleanupRateLimits(env.DB).catch((err) => console.error('[rate-limit] cleanup failed:', err))
  }

  let allowed: boolean
  let retryAfterSeconds: number
  try {
    ;({ allowed, retryAfterSeconds } = await checkRateLimit(env.DB, ip, scope, limit, windowMs))
  } catch (err) {
    console.error('[rate-limit] D1 error, failing open:', err)
    return null
  }

  if (allowed) return null
  return json({ error: 'Too many requests' }, 429, { 'Retry-After': String(retryAfterSeconds) })
}

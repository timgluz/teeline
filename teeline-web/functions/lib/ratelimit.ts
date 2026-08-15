// Fixed-window per-IP rate limiter backed by D1.
//
// The window is derived from the clock, so a request that lands in a new
// window starts a fresh counter. The increment is atomic (INSERT ... ON
// CONFLICT DO UPDATE), which keeps the count correct under concurrency.
// Approximation is fine for a limiter: we read the count right after the
// atomic increment rather than using RETURNING (broader D1 compatibility).
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

  await db
    .prepare(
      `INSERT INTO rate_limits (key, count, window_start) VALUES (?1, 1, ?2)
       ON CONFLICT(key) DO UPDATE SET count = count + 1`,
    )
    .bind(key, windowStart)
    .run()

  const row = await db
    .prepare('SELECT count FROM rate_limits WHERE key = ?1')
    .bind(key)
    .first<{ count: number }>()

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
 * when the limit is exceeded, otherwise null (proceed).
 */
export async function rateLimit(
  request: Request,
  env: Env,
  scope: string,
  limit: number,
  windowMs: number = 60_000,
): Promise<Response | null> {
  const ip = request.headers.get('CF-Connecting-IP') ?? 'unknown'
  const { allowed, retryAfterSeconds } = await checkRateLimit(env.DB, ip, scope, limit, windowMs)
  if (allowed) {
    // ~1% of requests also sweep stale windows.
    if (Math.random() < 0.01) {
      cleanupRateLimits(env.DB).catch(() => {})
    }
    return null
  }
  return json({ error: 'Too many requests' }, 429, { 'Retry-After': String(retryAfterSeconds) })
}

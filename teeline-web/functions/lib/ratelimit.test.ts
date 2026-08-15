// Rate limiter tests against the D1 shim (real SQLite).
import { beforeEach, describe, expect, it } from 'vitest'
import { makeD1, SCHEMA, type D1Like } from './test/sqlite-d1'
import Database from 'better-sqlite3'
import { checkRateLimit, cleanupRateLimits } from './ratelimit'

let db: D1Like

beforeEach(() => {
  // Fresh in-memory DB per test — migrations are not idempotent (0003 is a
  // plain ALTER TABLE ADD COLUMN).
  db = makeD1(new Database(':memory:'))
  db.exec(SCHEMA)
})

const NOW = 1_752_000_000_000

describe('checkRateLimit', () => {
  it('allows up to the limit, then denies within the same window', async () => {
    for (let i = 1; i <= 3; i++) {
      const r = await checkRateLimit(db, '1.2.3.4', 'login-begin', 3, 60_000, NOW)
      expect(r.allowed).toBe(true)
    }
    const denied = await checkRateLimit(db, '1.2.3.4', 'login-begin', 3, 60_000, NOW)
    expect(denied.allowed).toBe(false)
    expect(denied.retryAfterSeconds).toBeGreaterThan(0)
  })

  it('tracks different scopes and IPs independently', async () => {
    await checkRateLimit(db, '1.1.1.1', 'register-begin', 1, 60_000, NOW)
    expect((await checkRateLimit(db, '1.1.1.1', 'login-begin', 1, 60_000, NOW)).allowed).toBe(true)
    expect((await checkRateLimit(db, '2.2.2.2', 'register-begin', 1, 60_000, NOW)).allowed).toBe(true)
    expect((await checkRateLimit(db, '1.1.1.1', 'register-begin', 1, 60_000, NOW)).allowed).toBe(false)
  })

  it('resets the counter in a new window', async () => {
    await checkRateLimit(db, '1.2.3.4', 'keys-verify', 1, 60_000, NOW)
    expect((await checkRateLimit(db, '1.2.3.4', 'keys-verify', 1, 60_000, NOW)).allowed).toBe(false)
    expect((await checkRateLimit(db, '1.2.3.4', 'keys-verify', 1, 60_000, NOW + 60_000)).allowed).toBe(true)
  })

  it('cleanupRateLimits removes expired windows only', async () => {
    await checkRateLimit(db, '1.2.3.4', 'scope', 5, 60_000, NOW - 48 * 3600 * 1000) // window older than the 24h cleanup threshold
    await checkRateLimit(db, '1.2.3.4', 'scope', 5, 60_000, NOW) // current window
    await cleanupRateLimits(db, NOW)
    const rows = await db.prepare('SELECT count FROM rate_limits').bind().all<{ count: number }>()
    expect(rows.results).toHaveLength(1)
  })
})

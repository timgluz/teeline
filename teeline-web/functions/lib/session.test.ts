// Session token + cookie tests (pure WebCrypto — no DB needed).
import { describe, expect, it } from 'vitest'
import {
  SESSION_COOKIE,
  clearSessionCookieHeader,
  createSessionToken,
  readCookie,
  readSessionToken,
  sessionCookieHeader,
  shouldRefreshSession,
} from './session'

const SECRET = 'test-session-secret-0123456789'
const NOW = Date.now()

describe('session tokens', () => {
  it('round-trips a token', async () => {
    const token = await createSessionToken(SECRET, 'u1', NOW)
    const payload = await readSessionToken(SECRET, token)
    expect(payload).toMatchObject({ sub: 'u1', iat: NOW, exp: NOW + 30 * 24 * 3600 * 1000 })
  })

  it('rejects a tampered token', async () => {
    const token = await createSessionToken(SECRET, 'u1', NOW)
    const flipped = token.slice(0, -1) + (token.endsWith('0') ? '1' : '0')
    expect(await readSessionToken(SECRET, flipped)).toBeNull()
  })

  it('rejects a token signed with a different secret', async () => {
    const token = await createSessionToken('other-secret', 'u1', NOW)
    expect(await readSessionToken(SECRET, token)).toBeNull()
  })

  it('rejects an expired token', async () => {
    const past = NOW - 40 * 24 * 3600 * 1000
    const token = await createSessionToken(SECRET, 'u1', past)
    expect(await readSessionToken(SECRET, token)).toBeNull()
  })

  it('rejects malformed tokens', async () => {
    expect(await readSessionToken(SECRET, null)).toBeNull()
    expect(await readSessionToken(SECRET, '')).toBeNull()
    expect(await readSessionToken(SECRET, 'no-dot-here')).toBeNull()
    expect(await readSessionToken(SECRET, 'garbage.signature')).toBeNull()
  })

  it('shouldRefreshSession only near expiry', async () => {
    const fresh = { sub: 'u1', iat: NOW, exp: NOW + 29 * 24 * 3600 * 1000 }
    expect(shouldRefreshSession(fresh, NOW)).toBe(false)
    const old = { sub: 'u1', iat: NOW, exp: NOW + 10 * 24 * 3600 * 1000 }
    expect(shouldRefreshSession(old, NOW)).toBe(true)
  })
})

describe('cookies', () => {
  it('sessionCookieHeader carries the security flags', () => {
    const h = sessionCookieHeader('tok123')
    expect(h).toContain(`${SESSION_COOKIE}=tok123`)
    expect(h).toContain('HttpOnly')
    expect(h).toContain('Secure')
    expect(h).toContain('SameSite=Strict')
    expect(h).toContain('Path=/')
  })

  it('clearSessionCookieHeader zeroes Max-Age', () => {
    const h = clearSessionCookieHeader()
    expect(h).toContain('Max-Age=0')
    expect(h).toContain(SESSION_COOKIE)
  })

  it('readCookie parses multi-cookie headers', () => {
    const header = `other=1; ${SESSION_COOKIE}=abc123; foo=2`
    expect(readCookie(header, SESSION_COOKIE)).toBe('abc123')
    expect(readCookie(header, 'missing')).toBeNull()
    expect(readCookie(null, SESSION_COOKIE)).toBeNull()
  })
})

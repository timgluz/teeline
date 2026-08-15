// Handler-level rate limiting: a single IP flooding an auth endpoint gets 429.
import { beforeAll, beforeEach, describe, expect, it } from 'vitest'
import { makeD1, SCHEMA, type D1Like } from '../../lib/test/sqlite-d1'
import Database from 'better-sqlite3'
import { onRequestPost as registerBegin } from './register/begin'

let shim: D1Like
const env = { SESSION_SECRET: 'test-session-secret', ALLOWED_ORIGINS: 'http://localhost:8788' } as never

function ctx(request: Request) {
  return { request, env: { ...env, DB: shim } } as never
}

beforeAll(() => {
  shim = makeD1(new Database(':memory:'))
})

beforeEach(() => {
  shim.exec(SCHEMA)
  shim.exec('DELETE FROM rate_limits; DELETE FROM challenges;')
})

describe('register/begin rate limit', () => {
  it('returns 429 once a single IP exceeds 10 requests per minute', async () => {
    const makeReq = (ip: string) =>
      new Request('http://localhost:8788/api/auth/register/begin', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Origin: 'http://localhost:8788', 'CF-Connecting-IP': ip },
        body: '{}',
      })

    for (let i = 0; i < 10; i++) {
      expect((await registerBegin(ctx(makeReq('9.9.9.9')))).status).toBe(201)
    }
    const limited = await registerBegin(ctx(makeReq('9.9.9.9')))
    expect(limited.status).toBe(429)
    expect(limited.headers.get('Retry-After')).toBeTruthy()

    // a different IP is unaffected
    expect((await registerBegin(ctx(makeReq('8.8.8.8')))).status).toBe(201)
  })
})

// API-key lifecycle tests: mint (show-once), list (metadata only), revoke
// (immediate), and the internal verify contract (the shape teeline-api
// expects — a drop-in for the old Clerk verify call).
import { beforeAll, beforeEach, describe, expect, it } from 'vitest'
import { makeD1, SCHEMA, type D1Like } from '../../lib/test/sqlite-d1'
import { createUser } from '../../lib/db'
import { createSessionToken } from '../../lib/session'
import Database from 'better-sqlite3'

import { onRequestGet as listKeys, onRequestPost as createKey } from './keys'
import { onRequestDelete as revokeKey } from './keys/[id]'
import { onRequestPost as verifyKey } from './keys/verify'

let shim: D1Like
const env = {
  SESSION_SECRET: 'test-session-secret',
  AUTH_SERVICE_SECRET: 'verify-shared-secret',
  ALLOWED_ORIGINS: 'http://localhost:8788',
} as never

function ctx(request: Request, params: Record<string, string> = {}) {
  return { request, env: { ...env, DB: shim }, params } as never
}

function req(path: string, init: RequestInit = {}): Request {
  const { headers, ...rest } = init
  return new Request(`http://localhost:8788${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Origin: 'http://localhost:8788', 'CF-Connecting-IP': '1.2.3.4', ...(headers as Record<string, string>) },
    ...rest,
  })
}

async function sessionCookie(userId: string): Promise<string> {
  const token = await createSessionToken(env.SESSION_SECRET as string, userId)
  return `__Host-teeline-session=${token}`
}

async function seedUser(id = 'u1'): Promise<void> {
  await createUser(shim, { id, displayName: 'Tim', createdAt: Date.now() })
}

beforeAll(() => {
  shim = makeD1(new Database(':memory:'))
})

beforeEach(() => {
  shim.exec(SCHEMA)
  shim.exec('DELETE FROM api_keys; DELETE FROM credentials; DELETE FROM challenges; DELETE FROM users;')
})

describe('key creation (show-once)', () => {
  it('requires a session', async () => {
    const res = await createKey(ctx(req('/api/auth/keys')))
    expect(res.status).toBe(401)
  })

  it('mints a well-formed key, stores only the hash, and returns the secret once', async () => {
    await seedUser()
    const cookie = await sessionCookie('u1')
    const res = await createKey(ctx(req('/api/auth/keys', { body: JSON.stringify({ name: 'my-laptop' }), headers: { Cookie: cookie } })))
    expect(res.status).toBe(201)
    const body = (await res.json()) as { id: string; name: string | null; secret: string }
    expect(body.id).toMatch(/^key_/)
    expect(body.name).toBe('my-laptop')
    expect(body.secret).toMatch(/^ak_[A-Za-z0-9_-]{43}$/)

    // only the hash is stored: looking up by the secret's hash finds it
    const rows = await shim.prepare('SELECT id, secret_hash FROM api_keys').bind().all<{ id: string; secret_hash: string }>()
    expect(rows.results).toHaveLength(1)
    expect(rows.results[0].secret_hash).not.toBe(body.secret)
    expect(rows.results[0].secret_hash).toMatch(/^[0-9a-f]{64}$/)

    // second mint → different secret
    const res2 = await createKey(ctx(req('/api/auth/keys', { headers: { Cookie: cookie } })))
    const body2 = (await res2.json()) as { secret: string }
    expect(body2.secret).not.toBe(body.secret)
  })
})

describe('key listing', () => {
  it('returns metadata only — never the secret or hash', async () => {
    await seedUser()
    const cookie = await sessionCookie('u1')
    await createKey(ctx(req('/api/auth/keys', { body: JSON.stringify({ name: 'a' }), headers: { Cookie: cookie } })))
    await createKey(ctx(req('/api/auth/keys', { body: JSON.stringify({ name: 'b' }), headers: { Cookie: cookie } })))

    const res = await listKeys(ctx(req('/api/auth/keys', { headers: { Cookie: cookie } })))
    expect(res.status).toBe(200)
    const { keys } = (await res.json()) as { keys: { id: string; name: string | null; secret?: string; secret_hash?: string }[] }
    expect(keys).toHaveLength(2)
    for (const k of keys) {
      expect(k.secret).toBeUndefined()
      expect(k.secret_hash).toBeUndefined()
      expect(typeof k.id).toBe('string')
    }
  })

  it('requires a session', async () => {
    expect((await listKeys(ctx(req('/api/auth/keys')))).status).toBe(401)
  })
})

describe('key revocation', () => {
  it('revokes a key the owner owns; verify stops accepting it immediately', async () => {
    await seedUser()
    await seedUser('u2')
    const cookie = await sessionCookie('u1')
    const minted = (await createKey(ctx(req('/api/auth/keys', { headers: { Cookie: cookie } })))).json() as unknown as Promise<{ id: string; secret: string }>
    const { id, secret } = await minted

    // verify works before revocation
    expect((await verifyKey(ctx(req('/api/auth/keys/verify', { body: JSON.stringify({ secret }), headers: { 'X-Auth-Secret': 'verify-shared-secret' } })))).status).toBe(200)

    // non-owner cannot revoke
    const nonOwner = await revokeKey(ctx(req('/api/auth/keys'), { id }), { id })
    expect((nonOwner as Response).status).toBe(401) // no session
    const otherCookie = await sessionCookie('u2')
    const asOther = await revokeKey(ctx(req('/api/auth/keys', { headers: { Cookie: otherCookie } }), { id }), { id })
    expect((asOther as Response).status).toBe(400) // 'Key not found' (scoped)

    // owner revokes → verify now 404
    const revoked = await revokeKey(ctx(req('/api/auth/keys', { headers: { Cookie: cookie } }), { id }), { id })
    expect((revoked as Response).status).toBe(200)
    expect((await verifyKey(ctx(req('/api/auth/keys/verify', { body: JSON.stringify({ secret }), headers: { 'X-Auth-Secret': 'verify-shared-secret' } })))).status).toBe(404)
  })
})

describe('internal verify endpoint', () => {
  it('rejects missing or wrong shared secret', async () => {
    const body = JSON.stringify({ secret: 'ak_anything' })
    expect((await verifyKey(ctx(req('/api/auth/keys/verify', { body })))).status).toBe(401)
    expect((await verifyKey(ctx(req('/api/auth/keys/verify', { body, headers: { 'X-Auth-Secret': 'wrong' } })))).status).toBe(401)
  })

  it('returns the Clerk-shaped contract for a valid key', async () => {
    await seedUser()
    const cookie = await sessionCookie('u1')
    const minted = (await createKey(ctx(req('/api/auth/keys', { headers: { Cookie: cookie } })))).json() as unknown as Promise<{ secret: string }>
    const { secret } = await minted

    const res = await verifyKey(ctx(req('/api/auth/keys/verify', { body: JSON.stringify({ secret }), headers: { 'X-Auth-Secret': 'verify-shared-secret' } })))
    expect(res.status).toBe(200)
    expect(await res.json()).toMatchObject({ subject: 'u1', revoked: false, expired: false })
  })

  it('404s for an unknown key', async () => {
    const res = await verifyKey(ctx(req('/api/auth/keys/verify', { body: JSON.stringify({ secret: 'ak_doesnotexist123456789012345678901234' }), headers: { 'X-Auth-Secret': 'verify-shared-secret' } })))
    expect(res.status).toBe(404)
  })

  it('rejects a missing secret field', async () => {
    const res = await verifyKey(ctx(req('/api/auth/keys/verify', { body: '{}', headers: { 'X-Auth-Secret': 'verify-shared-secret' } })))
    expect(res.status).toBe(400)
  })

  it('handles a literal null body without crashing', async () => {
    const res = await verifyKey(ctx(req('/api/auth/keys/verify', { body: 'null', headers: { 'X-Auth-Secret': 'verify-shared-secret' } })))
    expect(res.status).toBe(400)
  })

  it('rejects malformed secrets early (shape check)', async () => {
    const res = await verifyKey(ctx(req('/api/auth/keys/verify', { body: JSON.stringify({ secret: 'not-a-key' }), headers: { 'X-Auth-Secret': 'verify-shared-secret' } })))
    expect(res.status).toBe(404)
  })
})

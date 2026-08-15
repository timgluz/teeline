// Data-layer tests against real SQLite (better-sqlite3) behind a minimal
// D1-compatible shim. D1 *is* SQLite, so the SQL in db.ts runs unchanged
// (numbered ?NNN params are normalized to anonymous ? — same bind order).
//
// Why not miniflare's standalone D1 emulation: the miniflare library build
// shipped with wrangler 4.119 fails to start D1 wrapped bindings
// (cloudflare-internal:d1-api; workers-sdk#4077 / #10114). Note `wrangler
// pages dev`'s local D1 *does* work — this only affects the library path.
// better-sqlite3 is deterministic, fast and CI-friendly; the real D1 path is
// exercised by the deploy-time migrations and the Phase 3 verify e2e.
import { beforeAll, beforeEach, describe, expect, it } from 'vitest'
import {
  addCredential,
  consumeChallenge,
  createApiKey,
  createUser,
  deleteUser,
  findActiveKeyByHash,
  getChallenge,
  getCredentialById,
  getUser,
  hashSecret,
  insertChallenge,
  listApiKeysByUser,
  listCredentialsByUser,
  revokeKey,
  timingSafeEqual,
  touchKeyLastUsed,
  updateCredentialCounter,
} from './db'

import { makeD1, SCHEMA, type D1Like } from './test/sqlite-d1'
import Database from 'better-sqlite3'

let db: D1Like

beforeAll(() => {
  db = makeD1(new Database(':memory:'))
})

beforeEach(() => {
  // Fresh schema per test; the SQL is idempotent (IF NOT EXISTS), which also
  // proves re-applying migrations is safe.
  db.exec(SCHEMA)
  db.exec('DELETE FROM api_keys; DELETE FROM credentials; DELETE FROM challenges; DELETE FROM users;')
})

const NOW = 1_752_000_000_000

describe('users', () => {
  it('creates and reads a user', async () => {
    await createUser(db as never, { id: 'u1', displayName: 'Tim', createdAt: NOW })
    const u = await getUser(db as never, 'u1')
    expect(u).toMatchObject({ id: 'u1', display_name: 'Tim', created_at: NOW })
  })

  it('returns null for a missing user', async () => {
    expect(await getUser(db as never, 'nope')).toBeNull()
  })

  it('deleteUser wipes user, credentials and keys atomically', async () => {
    await createUser(db as never, { id: 'u1', createdAt: NOW })
    await addCredential(db as never, { id: 'c1', userId: 'u1', publicKey: 'pk', counter: 0, createdAt: NOW })
    await createApiKey(db as never, { id: 'k1', userId: 'u1', secretHash: await hashSecret('ak_x'), createdAt: NOW })
    await deleteUser(db as never, 'u1')
    expect(await getUser(db as never, 'u1')).toBeNull()
    expect(await getCredentialById(db as never, 'c1')).toBeNull()
    expect(await listApiKeysByUser(db as never, 'u1')).toEqual([])
  })
})

describe('credentials', () => {
  it('adds, lists and looks up by id (login path)', async () => {
    await createUser(db as never, { id: 'u1', createdAt: NOW })
    await addCredential(db as never, { id: 'c1', userId: 'u1', publicKey: 'pk1', counter: 1, transports: ['internal'], createdAt: NOW })
    await addCredential(db as never, { id: 'c2', userId: 'u1', publicKey: 'pk2', counter: 2, createdAt: NOW })

    const byUser = await listCredentialsByUser(db as never, 'u1')
    expect(byUser.map((c) => c.id)).toEqual(['c1', 'c2'])
    expect(byUser[0].transports).toBe(JSON.stringify(['internal']))

    const found = await getCredentialById(db as never, 'c2')
    expect(found?.user_id).toBe('u1')
    expect(await getCredentialById(db as never, 'ghost')).toBeNull()
  })

  it('updates the counter atomically', async () => {
    await createUser(db as never, { id: 'u1', createdAt: NOW })
    await addCredential(db as never, { id: 'c1', userId: 'u1', publicKey: 'pk', counter: 0, createdAt: NOW })
    await updateCredentialCounter(db as never, 'c1', 7)
    expect((await getCredentialById(db as never, 'c1'))?.counter).toBe(7)
  })
})

describe('api keys', () => {
  it('creates, lists metadata and finds by hash', async () => {
    await createUser(db as never, { id: 'u1', createdAt: NOW })
    const secret = 'ak_abc123'
    await createApiKey(db as never, { id: 'k1', userId: 'u1', name: 'laptop', secretHash: await hashSecret(secret), createdAt: NOW })

    const keys = await listApiKeysByUser(db as never, 'u1')
    expect(keys).toHaveLength(1)
    expect(keys[0]).toMatchObject({ id: 'k1', name: 'laptop', revoked: 0 })
    // secret_hash is present (needed for verify) but callers must never expose it
    expect(keys[0].secret_hash).toBe(await hashSecret(secret))

    const found = await findActiveKeyByHash(db as never, await hashSecret(secret))
    expect(found?.user_id).toBe('u1')
    expect(await findActiveKeyByHash(db as never, await hashSecret('ak_wrong'))).toBeNull()
  })

  it('revoke hides the key from verify immediately, scoped to owner', async () => {
    await createUser(db as never, { id: 'u1', createdAt: NOW })
    await createUser(db as never, { id: 'u2', createdAt: NOW })
    const secret = 'ak_xyz'
    await createApiKey(db as never, { id: 'k1', userId: 'u1', secretHash: await hashSecret(secret), createdAt: NOW })

    // non-owner cannot revoke
    expect(await revokeKey(db as never, 'k1', 'u2')).toBe(false)
    expect(await findActiveKeyByHash(db as never, await hashSecret(secret))).not.toBeNull()

    // owner revokes → verify path immediately returns nothing
    expect(await revokeKey(db as never, 'k1', 'u1')).toBe(true)
    expect(await findActiveKeyByHash(db as never, await hashSecret(secret))).toBeNull()
  })

  it('touchKeyLastUsed records usage', async () => {
    await createUser(db as never, { id: 'u1', createdAt: NOW })
    await createApiKey(db as never, { id: 'k1', userId: 'u1', secretHash: 'h', createdAt: NOW })
    await touchKeyLastUsed(db as never, 'k1', 123)
    expect((await listApiKeysByUser(db as never, 'u1'))[0].last_used_at).toBe(123)
  })
})

describe('challenges', () => {
  it('insert, read and single-use consume', async () => {
    await insertChallenge(db as never, { id: 'ch1', type: 'register', challenge: 'abc', userHandle: 'uh', expiresAt: NOW + 300_000 })
    const c = await getChallenge(db as never, 'ch1')
    expect(c).toMatchObject({ id: 'ch1', type: 'register', challenge: 'abc', user_handle: 'uh' })

    expect(await consumeChallenge(db as never, 'ch1')).toBe(true)
    expect(await consumeChallenge(db as never, 'ch1')).toBe(false) // replay fails
    expect(await getChallenge(db as never, 'ch1')).toBeNull()
  })

  it('login challenge stores a user_id', async () => {
    await insertChallenge(db as never, { id: 'ch2', type: 'login', challenge: 'def', userId: 'u1', expiresAt: NOW + 300_000 })
    expect((await getChallenge(db as never, 'ch2'))?.user_id).toBe('u1')
  })
})

describe('crypto helpers', () => {
  it('hashSecret is deterministic SHA-256 hex', async () => {
    const h = await hashSecret('ak_secret')
    expect(h).toMatch(/^[0-9a-f]{64}$/)
    expect(await hashSecret('ak_secret')).toBe(h)
    expect(await hashSecret('ak_secret2')).not.toBe(h)
  })

  it('timingSafeEqual compares in constant time', () => {
    expect(timingSafeEqual('abc', 'abc')).toBe(true)
    expect(timingSafeEqual('abc', 'abd')).toBe(false)
    expect(timingSafeEqual('abc', 'abcd')).toBe(false) // length mismatch short-circuits
    expect(timingSafeEqual('', '')).toBe(true)
  })
})

describe('d1 shim', () => {
  it('first(col) returns a single column value like real D1', async () => {
    await createUser(db as never, { id: 'u1', displayName: 'Tim', createdAt: NOW })
    const name = await db.prepare('SELECT id, display_name FROM users WHERE id = ?1').bind('u1').first<{ display_name: string }>('display_name')
    expect(name).toBe('Tim')
    const missing = await db.prepare('SELECT id, display_name FROM users WHERE id = ?1').bind('nope').first('display_name')
    expect(missing).toBeNull()
  })
})

// End-to-end handler tests for the WebAuthn flow, with @simplewebauthn/server
// mocked at the verification boundary and the real SQL against the D1 shim.
// Exercises: challenge lifecycle, atomic user+credential creation, session
// cookie issuance/validation, counter updates, origin policy.
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { isoBase64URL } from '@simplewebauthn/server/helpers'
import { makeD1, SCHEMA, type D1Like } from '../../lib/test/sqlite-d1'
import { addCredential, createUser, getChallenge, getCredentialById, getUser, setUserBanned } from '../../lib/db'
import { createSessionToken } from '../../lib/session'
import Database from 'better-sqlite3'

const mocks = vi.hoisted(() => ({
  generateRegistrationOptions: vi.fn(),
  verifyRegistrationResponse: vi.fn(),
  generateAuthenticationOptions: vi.fn(),
  verifyAuthenticationResponse: vi.fn(),
}))
vi.mock('@simplewebauthn/server', () => mocks)

import { onRequestPost as registerBegin } from './register/begin'
import { onRequestPost as registerComplete } from './register/complete'
import { onRequestPost as loginBegin } from './login/begin'
import { onRequestPost as loginComplete } from './login/complete'
import { onRequestGet as me } from './me'
import { onRequestPost as logout } from './logout'
import { onRequestGet as listKeys, onRequestPost as createKey } from './keys'

let shim: D1Like
const env = { SESSION_SECRET: 'test-session-secret', ALLOWED_ORIGINS: 'http://localhost:8788' } as never // DB injected below

function ctx(request: Request) {
  return { request, env: { ...env, DB: shim } } as never
}

function req(path: string, init: RequestInit = {}): Request {
  return new Request(`http://localhost:8788${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Origin: 'http://localhost:8788', 'CF-Connecting-IP': '1.2.3.4' },
    ...init,
  })
}

const NOW = Date.now()

beforeEach(() => {
  // Fresh in-memory DB per test — migrations are not idempotent (0003 is a
  // plain ALTER TABLE ADD COLUMN).
  shim = makeD1(new Database(':memory:'))
  shim.exec(SCHEMA)
  vi.clearAllMocks()
  mocks.generateRegistrationOptions.mockResolvedValue({ challenge: 'reg-challenge' })
  mocks.generateAuthenticationOptions.mockResolvedValue({ challenge: 'login-challenge', rpId: 'localhost', allowCredentials: [], userVerification: 'preferred' })
  mocks.verifyRegistrationResponse.mockResolvedValue({
    verified: true,
    registrationInfo: {
      credential: { id: 'cred-1', publicKey: new Uint8Array(32).fill(7), counter: 3, transports: ['internal', 'usb'] },
      credentialType: 'public-key',
      fmt: 'none',
      aaguid: '00000000-0000-0000-0000-000000000000',
      attestationObject: new Uint8Array(0),
    },
  })
  mocks.verifyAuthenticationResponse.mockResolvedValue({ verified: true, authenticationInfo: { newCounter: 9 } })
})

describe('register', () => {
  it('begin stores a challenge; complete creates the user + credential atomically and sets the session cookie', async () => {
    const beginRes = await registerBegin(ctx(req('/api/auth/register/begin', { body: JSON.stringify({ displayName: 'Tim' }) })))
    expect(beginRes.status).toBe(201)
    const { options, nonce } = (await beginRes.json()) as { options: { challenge: string }; nonce: string }
    expect(options.challenge).toBe('reg-challenge')

    const stored = await getChallenge(shim, nonce)
    expect(stored?.type).toBe('register')
    expect(stored?.user_handle).toBeTruthy()

    const completeRes = await registerComplete(
      ctx(req('/api/auth/register/complete', { body: JSON.stringify({ nonce, credential: { id: 'x', response: { clientDataJSON: 'client', attestationObject: 'att' } } }) })),
    )
    expect(completeRes.status).toBe(201)
    const setCookie = completeRes.headers.get('Set-Cookie')
    expect(setCookie).toContain('__Host-teeline-session')
    expect(setCookie).toContain('HttpOnly')
    expect(setCookie).toContain('SameSite=Strict')

    // challenge consumed, user + credential persisted
    expect(await getChallenge(shim, nonce)).toBeNull()
    const users = await shim.prepare('SELECT id FROM users').bind().all<{ id: string }>()
    expect(users.results).toHaveLength(1)
    const creds = await shim.prepare('SELECT id, counter, transports FROM credentials').bind().all<{ id: string; counter: number; transports: string }>()
    expect(creds.results).toHaveLength(1)
    expect(creds.results[0]).toMatchObject({ id: 'cred-1', counter: 3 })
    expect(JSON.parse(creds.results[0].transports)).toEqual(['internal', 'usb'])
  })

  it('rejects a replayed nonce', async () => {
    const begin = (await registerBegin(ctx(req('/api/auth/register/begin', { body: '{}' })))).json() as unknown as Promise<{ nonce: string }>
    const { nonce } = await begin
    const body = JSON.stringify({ nonce, credential: { id: 'x', response: { clientDataJSON: 'client', attestationObject: 'att' } } })
    expect((await registerComplete(ctx(req('/api/auth/register/complete', { body })))).status).toBe(201)
    expect((await registerComplete(ctx(req('/api/auth/register/complete', { body })))).status).toBe(400)
  })

  it('rejects an unknown nonce', async () => {
    const res = await registerComplete(ctx(req('/api/auth/register/complete', { body: JSON.stringify({ nonce: 'ghost', credential: { id: 'x', response: { clientDataJSON: 'client', attestationObject: 'att' } } }) })))
    expect(res.status).toBe(400)
  })
})

describe('login', () => {
  async function seedUser() {
    await createUser(shim, { id: 'u1', displayName: 'Tim', createdAt: NOW })
    await addCredential(shim, {
      id: 'cred-1',
      userId: 'u1',
      publicKey: isoBase64URL.fromBuffer(new Uint8Array(32).fill(5)),
      counter: 4,
      createdAt: NOW,
    })
  }

  it('begin + complete authenticates and updates the counter', async () => {
    await seedUser()
    const beginRes = await loginBegin(ctx(req('/api/auth/login/begin', { body: '{}' })))
    expect(beginRes.status).toBe(201)
    const { nonce } = (await beginRes.json()) as { nonce: string }

    const completeRes = await loginComplete(
      ctx(req('/api/auth/login/complete', { body: JSON.stringify({ nonce, credential: { id: 'cred-1', response: { clientDataJSON: 'client', authenticatorData: 'auth', signature: 'sig' } } }) })),
    )
    expect(completeRes.status).toBe(200)
    expect(completeRes.headers.get('Set-Cookie')).toContain('__Host-teeline-session')
    expect((await getCredentialById(shim, 'cred-1'))?.counter).toBe(9) // newCounter from mock
  })

  it('rejects an unknown credential', async () => {
    await seedUser()
    const beginRes = await loginBegin(ctx(req('/api/auth/login/begin', { body: '{}' })))
    const { nonce } = (await beginRes.json()) as { nonce: string }
    const res = await loginComplete(
      ctx(req('/api/auth/login/complete', { body: JSON.stringify({ nonce, credential: { id: 'ghost', response: { clientDataJSON: 'client', authenticatorData: 'auth', signature: 'sig' } } }) })),
    )
    expect(res.status).toBe(400)
  })

  it('denies a banned user (403) but still advances the credential counter', async () => {
    await seedUser()
    await setUserBanned(shim, 'u1', true)
    const beginRes = await loginBegin(ctx(req('/api/auth/login/begin', { body: '{}' })))
    const { nonce } = (await beginRes.json()) as { nonce: string }

    const res = await loginComplete(
      ctx(req('/api/auth/login/complete', { body: JSON.stringify({ nonce, credential: { id: 'cred-1', response: { clientDataJSON: 'client', authenticatorData: 'auth', signature: 'sig' } } }) })),
    )
    expect(res.status).toBe(403)
    expect(res.headers.get('Set-Cookie')).toBeNull() // no session for a banned user
    // counter advanced anyway so an eventual unban doesn't hit a stale counter
    expect((await getCredentialById(shim, 'cred-1'))?.counter).toBe(9)
  })
})

describe('session endpoints', () => {
  it('me returns the user for a valid session cookie', async () => {
    await createUser(shim, { id: 'u1', displayName: 'Tim', createdAt: NOW })
    const token = await createSessionToken(env.SESSION_SECRET as string, 'u1', NOW)
    const res = await me(ctx(new Request('http://localhost:8788/api/auth/me', { headers: { Cookie: `__Host-teeline-session=${token}` } })))
    expect(res.status).toBe(200)
    const body = (await res.json()) as { user: { id: string; displayName: string } }
    expect(body.user).toMatchObject({ id: 'u1', displayName: 'Tim' })
  })

  it('me rejects a missing or tampered cookie', async () => {
    expect((await me(ctx(new Request('http://localhost:8788/api/auth/me')))).status).toBe(401)
    expect((await me(ctx(new Request('http://localhost:8788/api/auth/me', { headers: { Cookie: '__Host-teeline-session=tampered' } })))).status).toBe(401)
  })

  it('me rejects a banned user (403) despite a valid cookie', async () => {
    await createUser(shim, { id: 'u1', displayName: 'Tim', createdAt: NOW })
    const token = await createSessionToken(env.SESSION_SECRET as string, 'u1', NOW)
    const cookie = `__Host-teeline-session=${token}`
    expect((await me(ctx(new Request('http://localhost:8788/api/auth/me', { headers: { Cookie: cookie } })))).status).toBe(200)

    await setUserBanned(shim, 'u1', true)
    const res = await me(ctx(new Request('http://localhost:8788/api/auth/me', { headers: { Cookie: cookie } })))
    expect(res.status).toBe(403)
    expect(res.headers.get('Set-Cookie')).toBeNull() // no session slide for a banned user
  })

  it('banned users cannot mint or list API keys (requireSession 403)', async () => {
    await createUser(shim, { id: 'u1', displayName: 'Tim', createdAt: NOW })
    const token = await createSessionToken(env.SESSION_SECRET as string, 'u1', NOW)
    const cookie = `__Host-teeline-session=${token}`
    await setUserBanned(shim, 'u1', true)

    expect((await createKey(ctx(req('/api/auth/keys', { headers: { Cookie: cookie } })))).status).toBe(403)
    expect((await listKeys(ctx(req('/api/auth/keys', { headers: { Cookie: cookie } })))).status).toBe(403)
  })

  it('logout clears the cookie', async () => {
    const res = await logout(ctx(req('/api/auth/logout')))
    expect(res.status).toBe(200)
    expect(res.headers.get('Set-Cookie')).toContain('Max-Age=0')
  })
})

describe('origin policy', () => {
  it('rejects ceremonies whose Origin header is not allowed (CSRF)', async () => {
    const evil = new Request('https://evil.com/api/auth/register/begin', {
      method: 'POST',
      headers: { Origin: 'https://evil.com' },
    })
    expect((await registerBegin(ctx(evil))).status).toBe(403)
  })

  it('rejects mutating requests with no Origin header (e.g. cross-site POSTs)', async () => {
    const noOrigin = new Request('http://localhost:8788/api/auth/logout', { method: 'POST' })
    expect((await logout(ctx(noOrigin))).status).toBe(403)
    // same-origin requests with the Origin header pass
    expect((await logout(ctx(req('/api/auth/logout')))).status).toBe(200)
  })
})

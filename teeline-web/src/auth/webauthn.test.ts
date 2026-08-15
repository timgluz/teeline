// Client WebAuthn ceremony helper tests — navigator.credentials + fetch
// stubbed, node env. Verifies the begin→create/get→complete wiring.
import { afterEach, describe, expect, it, vi } from 'vitest'
import { loginWithPasskey, me, registerPasskey } from './webauthn'

afterEach(() => {
  vi.unstubAllGlobals()
})

// fetch stub: route begin/complete based on the URL
function stubAuthFetch() {
  vi.stubGlobal('fetch', vi.fn(async (input: string) => {
    const path = String(input)
    if (path.endsWith('/register/begin')) {
      return new Response(JSON.stringify({ options: { challenge: 'reg-challenge' }, nonce: 'n1' }), { status: 201 })
    }
    if (path.endsWith('/register/complete')) {
      return new Response(JSON.stringify({ id: 'u1', displayName: null, createdAt: 1 }), { status: 201 })
    }
    if (path.endsWith('/login/begin')) {
      return new Response(JSON.stringify({ options: { challenge: 'login-challenge' }, nonce: 'n2' }), { status: 201 })
    }
    if (path.endsWith('/login/complete')) {
      return new Response(JSON.stringify({ id: 'u1', displayName: 'Tim', createdAt: 1 }), { status: 200 })
    }
    if (path.endsWith('/me')) {
      return new Response(JSON.stringify({ error: 'Unauthorized' }), { status: 401 })
    }
    return new Response(JSON.stringify({ error: 'not found' }), { status: 404 })
  }))
}

function stubCredentials() {
  const credential = {
    id: 'cred-1',
    type: 'public-key',
    rawId: 'raw',
    response: { clientDataJSON: 'c', attestationObject: 'a' },
    getClientExtensionResults: () => ({}),
  } as unknown as PublicKeyCredential
  const get = vi.fn(async (_opts: CredentialRequestOptions) => credential)
  const create = vi.fn(async (_opts: CredentialCreationOptions) => credential)
  vi.stubGlobal('navigator', { credentials: { get, create } })
  return { get, create }
}

describe('registerPasskey', () => {
  it('runs begin → credentials.create → complete and returns the user', async () => {
    stubAuthFetch()
    const { create } = stubCredentials()
    const user = await registerPasskey('Tim')
    expect(user).toMatchObject({ id: 'u1' })
    expect(create).toHaveBeenCalledTimes(1)
    const opts = create.mock.calls[0][0] as CredentialCreationOptions
    expect(opts.publicKey).toBeDefined()
  })

  it('throws when the user cancels the ceremony', async () => {
    stubAuthFetch()
    vi.stubGlobal('navigator', { credentials: { get: vi.fn(), create: vi.fn(async () => null) } })
    await expect(registerPasskey()).rejects.toThrow('cancelled')
  })
})

describe('loginWithPasskey', () => {
  it('runs begin → credentials.get → complete and returns the user', async () => {
    stubAuthFetch()
    const { get } = stubCredentials()
    const user = await loginWithPasskey()
    expect(user).toMatchObject({ id: 'u1', displayName: 'Tim' })
    expect(get).toHaveBeenCalledTimes(1)
  })
})

describe('me', () => {
  it('returns null on 401 (not signed in)', async () => {
    stubAuthFetch()
    stubCredentials()
    await expect(me()).resolves.toBeNull()
  })
})

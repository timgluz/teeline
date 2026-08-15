// Client WebAuthn ceremony helper tests — navigator.credentials + fetch
// stubbed, node env. Verifies the begin→create/get→complete wiring.
import { afterEach, describe, expect, it, vi } from 'vitest'
import { loginWithPasskey, me, registerPasskey } from './webauthn'

const mocks = vi.hoisted(() => ({
  startRegistration: vi.fn(),
  startAuthentication: vi.fn(),
}))
vi.mock('@simplewebauthn/browser', () => mocks)

afterEach(() => {
  vi.unstubAllGlobals()
  vi.clearAllMocks()
})

// fetch stub: route begin/complete based on the URL
function stubAuthFetch() {
  vi.stubGlobal('fetch', vi.fn(async (input: string) => {
    const path = String(input)
    if (path.endsWith('/register/begin')) {
      return new Response(JSON.stringify({ options: { challenge: 'reg-challenge' }, nonce: 'n1' }), { status: 201 })
    }
    if (path.endsWith('/register/complete')) {
      return new Response(JSON.stringify({ user: { id: 'u1', displayName: null, createdAt: 1 } }), { status: 201 })
    }
    if (path.endsWith('/login/begin')) {
      return new Response(JSON.stringify({ options: { challenge: 'login-challenge' }, nonce: 'n2' }), { status: 201 })
    }
    if (path.endsWith('/login/complete')) {
      return new Response(JSON.stringify({ user: { id: 'u1', displayName: 'Tim', createdAt: 1 } }), { status: 200 })
    }
    if (path.endsWith('/me')) {
      return new Response(JSON.stringify({ error: 'Unauthorized' }), { status: 401 })
    }
    return new Response(JSON.stringify({ error: 'not found' }), { status: 404 })
  }))
}

describe('registerPasskey', () => {
  it('runs begin → startRegistration → complete and returns the user', async () => {
    stubAuthFetch()
    mocks.startRegistration.mockResolvedValue({ id: 'cred-1', rawId: 'raw', type: 'public-key', response: { clientDataJSON: 'c', attestationObject: 'a' }, clientExtensionResults: {} })
    const user = await registerPasskey('Tim')
    expect(user).toMatchObject({ id: 'u1' })
    expect(mocks.startRegistration).toHaveBeenCalledTimes(1)
    expect(mocks.startRegistration.mock.calls[0][0].optionsJSON.challenge).toBe('reg-challenge')
  })

  it('maps user cancellation to a friendly error', async () => {
    stubAuthFetch()
    mocks.startRegistration.mockRejectedValue(new DOMException('The operation either timed out or was not allowed.', 'NotAllowedError'))
    await expect(registerPasskey()).rejects.toThrow('Passkey creation was cancelled')
  })

  it('maps unexpected failures to a generic error', async () => {
    stubAuthFetch()
    mocks.startRegistration.mockRejectedValue(new Error('boom'))
    await expect(registerPasskey()).rejects.toThrow('please try again')
  })
})

describe('loginWithPasskey', () => {
  it('runs begin → startAuthentication → complete and returns the user', async () => {
    stubAuthFetch()
    mocks.startAuthentication.mockResolvedValue({ id: 'cred-1', rawId: 'raw', type: 'public-key', response: { clientDataJSON: 'c', authenticatorData: 'a', signature: 's' }, clientExtensionResults: {} })
    const user = await loginWithPasskey()
    expect(user).toMatchObject({ id: 'u1', displayName: 'Tim' })
    expect(mocks.startAuthentication).toHaveBeenCalledTimes(1)
    expect(mocks.startAuthentication.mock.calls[0][0].optionsJSON.challenge).toBe('login-challenge')
  })

  it('maps user cancellation to a friendly error', async () => {
    stubAuthFetch()
    mocks.startAuthentication.mockRejectedValue(new DOMException('aborted', 'AbortError'))
    await expect(loginWithPasskey()).rejects.toThrow('Passkey sign-in was cancelled')
  })
})

describe('me', () => {
  it('returns null on 401 (not signed in)', async () => {
    stubAuthFetch()
    await expect(me()).resolves.toBeNull()
  })
})

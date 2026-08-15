// Client-side WebAuthn ceremony helpers.
//
// Uses @simplewebauthn/browser — the client half of the same library as the
// server (Pages Functions). It handles the two fiddly conversions that
// native navigator.credentials does NOT: decoding the server's base64url
// JSON options into WebAuthn-native BufferSources (challenge, user.id,
// allowCredentials ids), and serializing the returned credential back to
// the base64url JSON the server's verify*Response() expects.
import { startAuthentication, startRegistration } from '@simplewebauthn/browser'
import { ApiError, apiFetch } from './api'

export interface User {
  id: string
  displayName: string | null
  createdAt: number
}

/** Map common WebAuthn failures to user-friendly messages. */
async function withCancellation<T>(fn: () => Promise<T>, cancelledMessage: string): Promise<T> {
  try {
    return await fn()
  } catch (err) {
    // User dismissed the prompt or timed out — treat as a graceful cancel.
    if (err instanceof DOMException && (err.name === 'NotAllowedError' || err.name === 'AbortError')) {
      throw new Error(cancelledMessage)
    }
    throw new Error('Passkey operation failed — please try again')
  }
}

/** First-time registration: create a passkey on this origin → session. */
export async function registerPasskey(displayName?: string): Promise<User> {
  const { options, nonce } = await apiFetch<{
    options: Parameters<typeof startRegistration>[0]['optionsJSON']
    nonce: string
  }>('/api/auth/register/begin', {
    method: 'POST',
    body: JSON.stringify({ displayName }),
  })

  const response = await withCancellation(
    () => startRegistration({ optionsJSON: options }),
    'Passkey creation was cancelled',
  )

  const { user } = await apiFetch<{ user: User }>('/api/auth/register/complete', {
    method: 'POST',
    body: JSON.stringify({ nonce, credential: response }),
  })
  return user
}

/** Sign in with an existing passkey (discoverable credentials — no username). */
export async function loginWithPasskey(): Promise<User> {
  const { options, nonce } = await apiFetch<{
    options: Parameters<typeof startAuthentication>[0]['optionsJSON']
    nonce: string
  }>('/api/auth/login/begin', {
    method: 'POST',
    body: JSON.stringify({}),
  })

  const response = await withCancellation(
    () => startAuthentication({ optionsJSON: options }),
    'Passkey sign-in was cancelled',
  )

  const { user } = await apiFetch<{ user: User }>('/api/auth/login/complete', {
    method: 'POST',
    body: JSON.stringify({ nonce, credential: response }),
  })
  return user
}

/** Current user, or null when not signed in. */
export async function me(): Promise<User | null> {
  try {
    const { user } = await apiFetch<{ user: User }>('/api/auth/me')
    return user
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) return null
    throw err
  }
}

export async function logout(): Promise<void> {
  await apiFetch<void>('/api/auth/logout', { method: 'POST' })
}

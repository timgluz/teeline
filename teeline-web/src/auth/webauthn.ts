// Client-side WebAuthn ceremony helpers — native navigator.credentials, no
// library needed. The server (Cloudflare Pages Functions) stores the
// challenge and verifies the attestation/assertion.
import { ApiError, apiFetch } from './api'

export interface User {
  id: string
  displayName: string | null
  createdAt: number
}

/** Serialize a PublicKeyCredential to the JSON shape SimpleWebAuthn expects. */
function credentialToJSON(cred: PublicKeyCredential): unknown {
  return JSON.parse(JSON.stringify(cred))
}

/** First-time registration: create a passkey on this origin → session. */
export async function registerPasskey(displayName?: string): Promise<User> {
  const { options, nonce } = await apiFetch<{
    options: PublicKeyCredentialCreationOptions
    nonce: string
  }>('/api/auth/register/begin', {
    method: 'POST',
    body: JSON.stringify({ displayName }),
  })

  const credential = await navigator.credentials.create({ publicKey: options })
  if (!credential) throw new Error('Passkey creation was cancelled')
  const publicKeyCredential = credential as PublicKeyCredential // WebAuthn create/get with publicKey options returns a PublicKeyCredential

  return apiFetch<User>('/api/auth/register/complete', {
    method: 'POST',
    body: JSON.stringify({ nonce, credential: credentialToJSON(publicKeyCredential) }),
  })
}

/** Sign in with an existing passkey (discoverable credentials — no username). */
export async function loginWithPasskey(): Promise<User> {
  const { options, nonce } = await apiFetch<{
    options: PublicKeyCredentialRequestOptions
    nonce: string
  }>('/api/auth/login/begin', {
    method: 'POST',
    body: '{}',
  })

  const credential = await navigator.credentials.get({ publicKey: options })
  if (!credential) throw new Error('Passkey sign-in was cancelled')
  const publicKeyCredential = credential as PublicKeyCredential

  return apiFetch<User>('/api/auth/login/complete', {
    method: 'POST',
    body: JSON.stringify({ nonce, credential: credentialToJSON(publicKeyCredential) }),
  })
}

/** Current user, or null when not signed in. */
export async function me(): Promise<User | null> {
  try {
    return await apiFetch<User>('/api/auth/me')
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) return null
    throw err
  }
}

export async function logout(): Promise<void> {
  await apiFetch<void>('/api/auth/logout', { method: 'POST' })
}

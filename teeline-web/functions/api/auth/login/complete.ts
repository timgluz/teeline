// POST /api/auth/login/complete — finish an authentication ceremony.
// Verifies the assertion against the stored challenge, atomically updates
// the credential counter (anti-clone bookkeeping), and sets the session
// cookie.
import { verifyAuthenticationResponse } from '@simplewebauthn/server'
import { isoBase64URL } from '@simplewebauthn/server/helpers'
import type { Env } from '../../../lib/env'
import { consumeChallenge, getChallenge, getCredentialById, getUser, updateCredentialCounter } from '../../../lib/db'
import { isAllowedOrigin, requestOrigin, rpIdFor } from '../../../lib/webauthn'
import { badRequest, forbidden, json, serverError } from '../../../lib/http'
import { createSessionToken, sessionCookieHeader } from '../../../lib/session'

type AuthenticationResponse = Parameters<typeof verifyAuthenticationResponse>[0]['response']

// Minimal runtime shape check before handing the payload to the verifier
// (defense-in-depth; the library validates the details).
function isAuthenticationCredential(v: unknown): v is AuthenticationResponse {
  if (typeof v !== 'object' || v === null) return false
  const c = v as { id?: unknown; response?: unknown }
  const r = c.response as { clientDataJSON?: unknown; authenticatorData?: unknown; signature?: unknown } | undefined
  return (
    typeof c.id === 'string' &&
    typeof r?.clientDataJSON === 'string' &&
    typeof r.authenticatorData === 'string' &&
    typeof r.signature === 'string'
  )
}

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  const origin = requestOrigin(request)
  if (!isAllowedOrigin(origin, env)) return forbidden()
  if (!env.SESSION_SECRET) return serverError('Auth service not configured')

  let body: { nonce?: unknown; credential?: unknown }
  try {
    body = (await request.json()) as { nonce?: unknown; credential?: unknown }
  } catch {
    return badRequest('Invalid JSON body')
  }
  if (typeof body.nonce !== 'string' || !isAuthenticationCredential(body.credential)) {
    return badRequest('nonce and credential are required')
  }
  const credentialId = (body.credential as { id: string }).id

  const row = await getChallenge(env.DB, body.nonce)
  if (!row || row.type !== 'login' || row.expires_at < Date.now()) {
    return badRequest('Unknown or expired challenge — start again')
  }
  if (!(await consumeChallenge(env.DB, row.id))) {
    return badRequest('Challenge already used — start again')
  }

  // Discoverable credentials send only the credential id; look it up across
  // all users to find the owner.
  const credential = await getCredentialById(env.DB, credentialId)
  if (!credential) return badRequest('Unknown credential')

  const rpID = rpIdFor(new URL(request.url).hostname)
  // transports type derived from the verifier's expectation (avoids a direct
  // dep on @simplewebauthn/typescript-types for one cast)
  type Transport = NonNullable<Parameters<typeof verifyAuthenticationResponse>[0]['credential']['transports']>[number]
  function safeParseTransports(raw: string): Transport[] | undefined {
    try {
      return JSON.parse(raw) as Transport[]
    } catch {
      return undefined // corrupt stored value — treat as absent
    }
  }
  let verification
  try {
    verification = await verifyAuthenticationResponse({
      response: body.credential,
      expectedChallenge: row.challenge,
      expectedOrigin: origin,
      expectedRPID: rpID,
      credential: {
        id: credential.id,
        publicKey: isoBase64URL.toBuffer(credential.public_key),
        counter: credential.counter,
        transports: credential.transports ? safeParseTransports(credential.transports) : undefined,
      },
    })
  } catch (err) {
    console.error('Authentication verification failed:', err)
    return badRequest('Authentication verification failed')
  }
  if (!verification.verified) return badRequest('Authentication not verified')

  // Resolve the user FIRST, then persist the new counter: updating the
  // counter for a credential whose user was deleted would permanently brick
  // it (stored counter > authenticator counter on the next attempt).
  try {
    const user = await getUser(env.DB, credential.user_id)
    if (!user) return serverError('User not found for verified credential')

    // Counter rollback is the clone-detection signal; SimpleWebAuthn already
    // rejected a lower counter, so persisting the new one is safe and atomic.
    await updateCredentialCounter(env.DB, credential.id, verification.authenticationInfo.newCounter)

    const token = await createSessionToken(env.SESSION_SECRET, user.id)
    return json(
      { user: { id: user.id, displayName: user.display_name, createdAt: user.created_at } },
      200,
      { 'Set-Cookie': sessionCookieHeader(token) },
    )
  } catch (err) {
    console.error('Login post-verification failed:', err)
    return serverError('Login failed — please try again')
  }
}

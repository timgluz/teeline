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

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  const origin = requestOrigin(request)
  if (!isAllowedOrigin(origin, env)) return forbidden()
  if (!env.SESSION_SECRET) return serverError('Auth service not configured (SESSION_SECRET missing)')

  let body: { nonce?: unknown; credential?: { id?: unknown } }
  try {
    body = (await request.json()) as { nonce?: unknown; credential?: { id?: unknown } }
  } catch {
    return badRequest('Invalid JSON body')
  }
  const credentialId = body.credential?.id
  if (typeof body.nonce !== 'string' || typeof credentialId !== 'string') {
    return badRequest('nonce and credential.id are required')
  }

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
  let verification
  try {
    verification = await verifyAuthenticationResponse({
      response: body.credential as AuthenticationResponse,
      expectedChallenge: row.challenge,
      expectedOrigin: origin,
      expectedRPID: rpID,
      credential: {
        id: credential.id,
        publicKey: isoBase64URL.toBuffer(credential.public_key),
        counter: credential.counter,
        transports: credential.transports ? (JSON.parse(credential.transports) as Transport[]) : undefined,
      },
    })
  } catch (err) {
    return badRequest(`Authentication verification failed: ${String(err)}`)
  }
  if (!verification.verified) return badRequest('Authentication not verified')

  // Counter rollback is the clone-detection signal; SimpleWebAuthn already
  // rejected a lower counter, so persisting the new one is safe and atomic.
  await updateCredentialCounter(env.DB, credential.id, verification.authenticationInfo.newCounter)

  const user = await getUser(env.DB, credential.user_id)
  if (!user) return serverError('User not found for verified credential')

  const token = await createSessionToken(env.SESSION_SECRET, user.id)
  return json(
    { user: { id: user.id, displayName: user.display_name, createdAt: user.created_at } },
    200,
    { 'Set-Cookie': sessionCookieHeader(token) },
  )
}

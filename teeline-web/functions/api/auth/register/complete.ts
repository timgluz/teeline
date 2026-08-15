// POST /api/auth/register/complete — finish a registration ceremony.
// Verifies the attestation response against the stored challenge, creates
// the user + credential atomically, and sets the session cookie.
import { verifyRegistrationResponse } from '@simplewebauthn/server'
import { isoBase64URL } from '@simplewebauthn/server/helpers'
import type { Env } from '../../../lib/env'
import { consumeChallenge, createUserWithCredential, getChallenge } from '../../../lib/db'
import { isAllowedOrigin, requestOrigin, rpIdFor } from '../../../lib/webauthn'
import { badRequest, forbidden, json, serverError } from '../../../lib/http'
import { createSessionToken, sessionCookieHeader } from '../../../lib/session'

type RegistrationResponse = Parameters<typeof verifyRegistrationResponse>[0]['response']

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  const origin = requestOrigin(request)
  if (!isAllowedOrigin(origin, env)) return forbidden()
  if (!env.SESSION_SECRET) return serverError('Auth service not configured (SESSION_SECRET missing)')

  let body: { nonce?: unknown; credential?: unknown }
  try {
    body = (await request.json()) as { nonce?: unknown; credential?: unknown }
  } catch {
    return badRequest('Invalid JSON body')
  }
  if (typeof body.nonce !== 'string' || !body.credential || typeof body.credential !== 'object') {
    return badRequest('nonce and credential are required')
  }

  const row = await getChallenge(env.DB, body.nonce)
  if (!row || row.type !== 'register' || row.expires_at < Date.now()) {
    return badRequest('Unknown or expired challenge — start again')
  }
  if (!row.user_handle) return serverError('Challenge missing user handle')
  if (!(await consumeChallenge(env.DB, row.id))) {
    return badRequest('Challenge already used — start again')
  }

  const rpID = rpIdFor(new URL(request.url).hostname)
  let verification
  try {
    verification = await verifyRegistrationResponse({
      response: body.credential as RegistrationResponse,
      expectedChallenge: row.challenge,
      expectedOrigin: origin,
      expectedRPID: rpID,
    })
  } catch (err) {
    return badRequest(`Registration verification failed: ${String(err)}`)
  }
  if (!verification.verified || !verification.registrationInfo) {
    return badRequest('Registration not verified')
  }

  const { credential } = verification.registrationInfo
  const { counter } = credential
  const userId = row.user_handle
  const now = Date.now()

  await createUserWithCredential(
    env.DB,
    { id: userId, displayName: undefined, createdAt: now },
    {
      id: credential.id, // already base64url (SimpleWebAuthn v13)
      publicKey: isoBase64URL.fromBuffer(credential.publicKey),
      counter,
      transports: credential.transports ?? [],
      createdAt: now,
    },
  )

  const token = await createSessionToken(env.SESSION_SECRET, userId, now)
  return json(
    { user: { id: userId, displayName: null, createdAt: now } },
    201,
    { 'Set-Cookie': sessionCookieHeader(token) },
  )
}

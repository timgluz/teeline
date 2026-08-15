// POST /api/auth/register/complete — finish a registration ceremony.
// Verifies the attestation response against the stored challenge, creates
// the user + credential atomically, and sets the session cookie.
import { verifyRegistrationResponse } from '@simplewebauthn/server'
import { isoBase64URL } from '@simplewebauthn/server/helpers'
import type { Env } from '../../../lib/env'
import { consumeChallenge, createUserWithCredential, getChallenge } from '../../../lib/db'
import { isClientOriginAllowed, rpIdFor, serverOrigin } from '../../../lib/webauthn'
import { badRequest, forbidden, json, serverError } from '../../../lib/http'
import { rateLimit } from '../../../lib/ratelimit'
import { createSessionToken, sessionCookieHeader } from '../../../lib/session'

type RegistrationResponse = Parameters<typeof verifyRegistrationResponse>[0]['response']

// D1/SQLite unique-constraint failures surface as an error whose message
// contains the constraint name — distinguish "already registered" from a
// transient failure so the client is told to log in, not to retry.
function isConstraintViolation(err: unknown): boolean {
  const msg = err instanceof Error ? err.message : String(err)
  return msg.includes('UNIQUE constraint failed')
}

// Minimal runtime shape check before handing the payload to the verifier
// (defense-in-depth; the library validates the details).
function isRegistrationCredential(v: unknown): v is RegistrationResponse {
  if (typeof v !== 'object' || v === null) return false
  const c = v as { response?: unknown }
  const r = c.response as { clientDataJSON?: unknown; attestationObject?: unknown } | undefined
  return typeof r?.clientDataJSON === 'string' && typeof r.attestationObject === 'string'
}

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  if (!isClientOriginAllowed(request, env)) return forbidden() // CSRF: client origin must be on the allowlist
  const limited = await rateLimit(request, env, 'register-complete', 10)
  if (limited) return limited
  if (!env.SESSION_SECRET) return serverError('Auth service not configured')

  let body: { nonce?: unknown; credential?: unknown }
  try {
    body = (await request.json()) as { nonce?: unknown; credential?: unknown }
  } catch {
    return badRequest('Invalid JSON body')
  }
  if (typeof body.nonce !== 'string' || !isRegistrationCredential(body.credential)) {
    return badRequest('nonce and credential are required')
  }

  let row
  try {
    row = await getChallenge(env.DB, body.nonce)
  } catch (err) {
    console.error('Failed to fetch challenge:', err)
    return serverError('Failed to complete registration — please try again')
  }
  if (!row || row.type !== 'register' || row.expires_at < Date.now()) {
    return badRequest('Unknown or expired challenge — start again')
  }
  if (!row.user_handle) return serverError('Challenge missing user handle')
  let consumed
  try {
    consumed = await consumeChallenge(env.DB, row.id)
  } catch (err) {
    console.error('Failed to consume challenge:', err)
    return serverError('Failed to complete registration — please try again')
  }
  if (!consumed) {
    return badRequest('Challenge already used — start again')
  }

  const rpID = rpIdFor(new URL(request.url).hostname)
  let verification
  try {
    verification = await verifyRegistrationResponse({
      response: body.credential,
      expectedChallenge: row.challenge,
      expectedOrigin: serverOrigin(request),
      expectedRPID: rpID,
    })
  } catch (err) {
    console.error('Registration verification failed:', err)
    return badRequest('Registration verification failed')
  }
  if (!verification.verified || !verification.registrationInfo) {
    return badRequest('Registration not verified')
  }

  const { credential } = verification.registrationInfo
  const { counter } = credential
  const userId = row.user_handle
  const now = Date.now()

  try {
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
  } catch (err) {
    // Challenge is already consumed; surface a generic error rather than a
    // bare 500 (e.g. duplicate credential id, transient DB failure).
    console.error('Failed to create user with credential:', err)
    if (isConstraintViolation(err)) {
      return badRequest('This passkey is already registered — please log in instead')
    }
    return serverError('Failed to complete registration — please try again')
  }

  let token
  try {
    token = await createSessionToken(env.SESSION_SECRET, userId, now)
  } catch (err) {
    // User + credential are persisted; only session minting failed. Tell the
    // client to log in rather than returning 500 (which would trigger a
    // duplicate registration attempt).
    console.error('Failed to create session token after registration:', err)
    return json(
      { error: 'Registration succeeded but the session could not be established — please log in.', user: { id: userId }, session_established: false },
      201,
    )
  }
  return json(
    { user: { id: userId, displayName: null, createdAt: now } },
    201,
    { 'Set-Cookie': sessionCookieHeader(token) },
  )
}

// POST /api/auth/register/begin — start a WebAuthn registration ceremony.
// Creates a challenge row (TTL, single-use) and returns the options the
// browser passes to navigator.credentials.create(). Registration is open:
// the first passkey on this origin becomes an account.
import { generateRegistrationOptions } from '@simplewebauthn/server'
import type { Env } from '../../../lib/env'
import { insertChallenge } from '../../../lib/db'
import { CHALLENGE_TTL_MS, RP_NAME, isAllowedOrigin, requestOrigin, rpIdFor } from '../../../lib/webauthn'
import { forbidden, json, serverError } from '../../../lib/http'

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  const origin = requestOrigin(request)
  if (!isAllowedOrigin(origin, env)) return forbidden()
  if (!env.SESSION_SECRET) return serverError('Auth service not configured')

  // Optional friendly name; defaults to a generic label. A null/primitive
  // body is valid JSON but not an object — guard before property access.
  let displayName = 'Teeline user'
  try {
    const body = (await request.json()) as { displayName?: unknown } | null
    if (
      typeof body === 'object' &&
      body !== null &&
      typeof body.displayName === 'string' &&
      body.displayName.trim()
    ) {
      // Truncate by code points, not UTF-16 units (avoid splitting surrogates).
      displayName = Array.from(body.displayName.trim()).slice(0, 64).join('')
    }
  } catch {
    /* body optional */
  }

  const rpID = rpIdFor(new URL(request.url).hostname)
  const userHandle = crypto.randomUUID()
  const nonce = crypto.randomUUID()

  let options
  try {
    options = await generateRegistrationOptions({
      rpName: RP_NAME,
      rpID,
      userName: userHandle, // stable unique id; the authenticator shows displayName
      userDisplayName: displayName,
      userID: new Uint8Array(new TextEncoder().encode(userHandle)),
      attestationType: 'none',
      authenticatorSelection: {
        residentKey: 'required',
        userVerification: 'preferred',
      },
      timeout: CHALLENGE_TTL_MS,
    })

    await insertChallenge(env.DB, {
      id: nonce,
      type: 'register',
      challenge: options.challenge,
      userHandle,
      expiresAt: Date.now() + CHALLENGE_TTL_MS,
    })
  } catch (err) {
    console.error('Failed to start registration:', err)
    return serverError('Failed to start registration — please try again')
  }

  return json({ options, nonce }, 201)
}

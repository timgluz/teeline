// POST /api/auth/login/begin — start a WebAuthn authentication ceremony.
// Uses discoverable (resident) credentials, so no username is needed — the
// authenticator lists passkeys for this rpID and the client picks one.
import { generateAuthenticationOptions } from '@simplewebauthn/server'
import type { Env } from '../../../lib/env'
import { insertChallenge } from '../../../lib/db'
import { CHALLENGE_TTL_MS, isAllowedOrigin, requestOrigin, rpIdFor } from '../../../lib/webauthn'
import { forbidden, json, serverError } from '../../../lib/http'

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  const origin = requestOrigin(request)
  if (!isAllowedOrigin(origin, env)) return forbidden()
  // Fail fast before the user interacts with the authenticator: login/complete
  // needs SESSION_SECRET to mint the session token.
  if (!env.SESSION_SECRET) return serverError('Auth service not configured')

  const rpID = rpIdFor(new URL(request.url).hostname)
  const nonce = crypto.randomUUID()

  const options = await generateAuthenticationOptions({
    rpID,
    allowCredentials: [], // discoverable credentials only
    userVerification: 'preferred',
    timeout: CHALLENGE_TTL_MS,
  })

  await insertChallenge(env.DB, {
    id: nonce,
    type: 'login',
    challenge: options.challenge,
    expiresAt: Date.now() + CHALLENGE_TTL_MS,
  })

  return json({ options, nonce }, 201)
}

// POST /api/auth/keys/verify — INTERNAL. Called by teeline-api (Fly.io) to
// check an API key on every authenticated request. Protected by the shared
// AUTH_SERVICE_SECRET; mirrors the old Clerk verify contract
// ({subject, revoked, expired}) so the Rust ApiKeyVerifier is a drop-in swap.
//
//   curl -X POST https://tspsolver.com/api/auth/keys/verify \
//     -H "X-Auth-Secret: <shared>" -H "Content-Type: application/json" \
//     -d '{"secret":"ak_..."}'
//   → 200 {subject, revoked:false, expired:false} | 404 unknown/revoked | 401 bad secret
import type { Env } from '../../../lib/env'
import { findActiveKeyByHash, timingSafeEqual, touchKeyLastUsed } from '../../../lib/db'
import { badRequest, json, unauthorized } from '../../../lib/http'
import { hashSecret, isKeySecretShape } from '../../../lib/keys'

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  const presented = request.headers.get('X-Auth-Secret')
  if (!env.AUTH_SERVICE_SECRET || !presented || !timingSafeEqual(presented, env.AUTH_SERVICE_SECRET)) {
    return unauthorized()
  }

  let body: { secret?: unknown }
  try {
    body = (await request.json()) as { secret?: unknown }
  } catch {
    return badRequest('Invalid JSON body')
  }
  if (!body || typeof body.secret !== 'string') return badRequest('secret is required')
  // Early reject of malformed secrets before hashing + DB lookup.
  if (!isKeySecretShape(body.secret)) return json({ error: 'Unknown or revoked key' }, 404)

  try {
    const hash = await hashSecret(body.secret)
    const key = await findActiveKeyByHash(env.DB, hash)
    if (!key) return json({ error: 'Unknown or revoked key' }, 404)

    // Best-effort usage tracking — must never fail the verification.
    try {
      await touchKeyLastUsed(env.DB, key.id)
    } catch (err) {
      console.warn('Failed to record key usage:', err)
    }

    return json({ subject: key.user_id, revoked: false, expired: false })
  } catch (err) {
    console.error('Key verification failed:', err)
    return json({ error: 'Verification failed' }, 500)
  }
}

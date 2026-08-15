// DELETE /api/auth/keys/:id — revoke a key (session required, scoped to the
// key's owner). Revocation is a soft flag, effective immediately.
import type { Env } from '../../../lib/env'
import { revokeKey } from '../../../lib/db'
import { badRequest, forbidden, json, serverError, unauthorized } from '../../../lib/http'
import { getSessionUser } from '../../../lib/session'
import { isClientOriginAllowed } from '../../../lib/webauthn'

export const onRequestDelete: PagesFunction<Env> = async ({ request, env, params }) => {
  if (!isClientOriginAllowed(request, env)) return forbidden() // CSRF
  const session = await getSessionUser(env, request.headers)
  if (!session) return unauthorized()

  const id = params.id
  if (!id || typeof id !== 'string') return badRequest('Key id required')

  try {
    const revoked = await revokeKey(env.DB, id, session.sub)
    if (!revoked) return badRequest('Key not found')
    return json({ ok: true })
  } catch (err) {
    console.error('Failed to revoke API key:', err)
    return serverError('Failed to revoke API key — please try again')
  }
}

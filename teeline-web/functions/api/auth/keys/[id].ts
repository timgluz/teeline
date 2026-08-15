// DELETE /api/auth/keys/:id — revoke a key (session required, scoped to the
// key's owner). Revocation is a soft flag, effective immediately.
import type { Env } from '../../../lib/env'
import { revokeKey } from '../../../lib/db'
import { badRequest, json, serverError } from '../../../lib/http'
import { requireSession } from '../../../lib/auth'

export const onRequestDelete: PagesFunction<Env> = async ({ request, env, params }) => {
  const auth = await requireSession(request, env)
  if (auth instanceof Response) return auth

  const id = params.id
  if (!id || typeof id !== 'string') return badRequest('Key id required')

  try {
    const revoked = await revokeKey(env.DB, id, auth.userId)
    if (!revoked) return badRequest('Key not found')
    return json({ ok: true })
  } catch (err) {
    console.error('Failed to revoke API key:', err)
    return serverError('Failed to revoke API key — please try again')
  }
}

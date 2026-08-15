// GET /api/auth/me — returns the signed-in user (or 401). Also slides the
// session cookie forward when it's past the halfway point of its lifetime.
import type { Env } from '../../lib/env'
import { getUser } from '../../lib/db'
import { json, unauthorized } from '../../lib/http'
import { createSessionToken, getSessionUser, sessionCookieHeader, shouldRefreshSession } from '../../lib/session'

export const onRequestGet: PagesFunction<Env> = async ({ request, env }) => {
  const session = await getSessionUser(env, request.headers)
  if (!session) return unauthorized()

  const user = await getUser(env.DB, session.sub)
  if (!user) return unauthorized() // account deleted → session is void

  const headers: Record<string, string> = {}
  if (shouldRefreshSession(session) && env.SESSION_SECRET) {
    headers['Set-Cookie'] = sessionCookieHeader(await createSessionToken(env.SESSION_SECRET, user.id))
  }
  return json({ user: { id: user.id, displayName: user.display_name, createdAt: user.created_at } }, 200, headers)
}

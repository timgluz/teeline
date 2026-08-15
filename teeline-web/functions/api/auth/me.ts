// GET /api/auth/me — returns the signed-in user (or 401). Also slides the
// session cookie forward when it's past the halfway point of its lifetime.
import type { Env } from '../../lib/env'
import { getUser } from '../../lib/db'
import { json, forbidden, serverError, unauthorized } from '../../lib/http'
import { createSessionToken, getSessionUser, sessionCookieHeader, shouldRefreshSession } from '../../lib/session'

export const onRequestGet: PagesFunction<Env> = async ({ request, env }) => {
  const session = await getSessionUser(env, request.headers)
  if (!session) return unauthorized()

  let user
  try {
    user = await getUser(env.DB, session.sub)
  } catch (err) {
    console.error('Failed to fetch user:', err)
    return serverError('Failed to load user — please try again')
  }
  if (!user) return unauthorized() // account deleted → session is void
  if (user.banned) return forbidden('Account is banned') // banned account → deny, no session slide

  const headers: Record<string, string> = {}
  if (shouldRefreshSession(session) && env.SESSION_SECRET) {
    try {
      headers['Set-Cookie'] = sessionCookieHeader(await createSessionToken(env.SESSION_SECRET, user.id))
    } catch (err) {
      // Sliding refresh is best-effort: the existing token stays valid until
      // exp, so a mint failure must not break the response.
      console.error('Session refresh failed:', err)
    }
  }
  return json({ user: { id: user.id, displayName: user.display_name, createdAt: user.created_at } }, 200, headers)
}

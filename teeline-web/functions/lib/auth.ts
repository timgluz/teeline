// Shared session authorization for authenticated endpoints:
// 1. CSRF — the client origin (Origin/Referer header) must be on the allowlist
// 2. session — valid HMAC cookie
// 3. user existence — a deleted account voids its session
// Returns the userId or an error Response.
import type { Env } from './env'
import { getUser } from './db'
import { forbidden, unauthorized } from './http'
import { getSessionUser } from './session'
import { isClientOriginAllowed } from './webauthn'

export async function requireSession(
  request: Request,
  env: Env,
): Promise<Response | { userId: string }> {
  if (!isClientOriginAllowed(request, env)) return forbidden() // CSRF
  const session = await getSessionUser(env, request.headers)
  if (!session) return unauthorized()
  const user = await getUser(env.DB, session.sub)
  if (!user) return unauthorized() // deleted account → session void
  return { userId: user.id }
}

// POST /api/auth/logout — clears the session cookie. Idempotent.
import type { Env } from '../../lib/env'
import { forbidden, json } from '../../lib/http'
import { clearSessionCookieHeader } from '../../lib/session'
import { isAllowedOrigin, requestOrigin } from '../../lib/webauthn'

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  // Same origin check as every other mutating endpoint (defense-in-depth on
  // top of SameSite=Strict).
  const origin = requestOrigin(request)
  if (!isAllowedOrigin(origin, env)) return forbidden()
  return json({ ok: true }, 200, { 'Set-Cookie': clearSessionCookieHeader() })
}

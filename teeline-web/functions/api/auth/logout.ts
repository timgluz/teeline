// POST /api/auth/logout — clears the session cookie. Idempotent.
import type { Env } from '../../lib/env'
import { json } from '../../lib/http'
import { clearSessionCookieHeader } from '../../lib/session'

export const onRequestPost: PagesFunction<Env> = async () => {
  return json({ ok: true }, 200, { 'Set-Cookie': clearSessionCookieHeader() })
}

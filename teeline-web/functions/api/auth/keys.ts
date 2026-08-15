// POST /api/auth/keys — mint a new API key (session required). The plaintext
// secret is returned exactly once; only its SHA-256 hash is stored.
// GET  /api/auth/keys — list key metadata (never the secret/hash).
import type { Env } from '../../lib/env'
import { createApiKey, listApiKeysByUser } from '../../lib/db'
import { json, serverError } from '../../lib/http'
import { generateKeySecret, hashSecret, newKeyId } from '../../lib/keys'
import { requireSession } from '../../lib/auth'

export const onRequestPost: PagesFunction<Env> = async ({ request, env }) => {
  const auth = await requireSession(request, env)
  if (auth instanceof Response) return auth

  // Optional human-readable label.
  let name: string | undefined
  try {
    const body = (await request.json()) as { name?: unknown } | null
    if (typeof body === 'object' && body !== null && typeof body.name === 'string' && body.name.trim()) {
      name = Array.from(body.name.trim()).slice(0, 64).join('')
    }
  } catch (err) {
    console.warn('keys: optional body parse failed:', err)
  }

  try {
    const secret = generateKeySecret()
    const id = newKeyId()
    const now = Date.now()
    await createApiKey(env.DB, {
      id,
      userId: auth.userId,
      name,
      secretHash: await hashSecret(secret),
      createdAt: now,
    })
    // `secret` is the only time the plaintext exists — the client must save
    // it now (the UI shows it once until the page is refreshed).
    return json({ id, name: name ?? null, secret, createdAt: now }, 201)
  } catch (err) {
    console.error('Failed to create API key:', err)
    return serverError('Failed to create API key — please try again')
  }
}

export const onRequestGet: PagesFunction<Env> = async ({ request, env }) => {
  const auth = await requireSession(request, env)
  if (auth instanceof Response) return auth

  try {
    const keys = await listApiKeysByUser(env.DB, auth.userId)
    return json({
      keys: keys.map((k) => ({
        id: k.id,
        name: k.name,
        createdAt: k.created_at,
        lastUsedAt: k.last_used_at,
        revoked: k.revoked === 1,
      })),
    })
  } catch (err) {
    console.error('Failed to list API keys:', err)
    return serverError('Failed to list API keys — please try again')
  }
}

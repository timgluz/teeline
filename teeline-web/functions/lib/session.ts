// Stateless signed session cookie.
//
// Token format: base64url(JSON {sub, iat, exp}) + "." + hex(HMAC-SHA256(token)).
// Cookie: __Host-teeline-session (HttpOnly; Secure; SameSite=Strict; 30 d).
// The __Host- prefix forces Secure + Path=/ + no Domain, which makes the
// cookie immune to subdomain-injected variants.
import { timingSafeEqual } from './db'
import type { Env } from './env'

export const SESSION_COOKIE = '__Host-teeline-session'
export const SESSION_TTL_MS = 30 * 24 * 60 * 60 * 1000 // 30 days
const SLIDING_AFTER_MS = 15 * 24 * 60 * 60 * 1000 // re-issue past 15 days

export interface SessionPayload {
  sub: string
  iat: number
  exp: number
}

function b64urlEncode(bytes: Uint8Array): string {
  let bin = ''
  for (const b of bytes) bin += String.fromCharCode(b)
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

function b64urlDecode(s: string): Uint8Array {
  const b64 = s.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (s.length % 4)) % 4)
  const bin = atob(b64)
  const out = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i)
  return out
}

async function hmacSha256Hex(secret: string, data: string): Promise<string> {
  const key = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign'],
  )
  const sig = await crypto.subtle.sign('HMAC', key, new TextEncoder().encode(data))
  return [...new Uint8Array(sig)].map((b) => b.toString(16).padStart(2, '0')).join('')
}

export async function createSessionToken(secret: string, sub: string, now: number = Date.now()): Promise<string> {
  const payload: SessionPayload = { sub, iat: now, exp: now + SESSION_TTL_MS }
  const body = b64urlEncode(new TextEncoder().encode(JSON.stringify(payload)))
  const sig = await hmacSha256Hex(secret, body)
  return `${body}.${sig}`
}

export async function readSessionToken(secret: string, token: string | null): Promise<SessionPayload | null> {
  if (!token) return null
  const dot = token.indexOf('.')
  if (dot <= 0) return null
  const body = token.slice(0, dot)
  const sig = token.slice(dot + 1)
  const expected = await hmacSha256Hex(secret, body)
  if (!timingSafeEqual(sig, expected)) return null
  try {
    const payload = JSON.parse(new TextDecoder().decode(b64urlDecode(body))) as SessionPayload
    if (typeof payload.sub !== 'string' || typeof payload.exp !== 'number') return null
    if (payload.exp <= Date.now()) return null
    return payload
  } catch {
    return null
  }
}

export function shouldRefreshSession(payload: SessionPayload, now: number = Date.now()): boolean {
  return payload.exp - now < SLIDING_AFTER_MS
}

// ---- Cookie helpers ------------------------------------------------------

export function sessionCookieHeader(token: string, maxAgeSeconds: number = SESSION_TTL_MS / 1000): string {
  return `${SESSION_COOKIE}=${token}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=${maxAgeSeconds}`
}

export function clearSessionCookieHeader(): string {
  return `${SESSION_COOKIE}=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0`
}

export function readCookie(headerValue: string | null, name: string): string | null {
  if (!headerValue) return null
  for (const part of headerValue.split(';')) {
    const eq = part.indexOf('=')
    if (eq === -1) continue
    if (part.slice(0, eq).trim() === name) return part.slice(eq + 1).trim()
  }
  return null
}

// Resolves the current session from the request's Cookie header.
export async function getSessionUser(env: Env, headers: Headers): Promise<SessionPayload | null> {
  if (!env.SESSION_SECRET) return null
  return readSessionToken(env.SESSION_SECRET, readCookie(headers.get('Cookie'), SESSION_COOKIE))
}

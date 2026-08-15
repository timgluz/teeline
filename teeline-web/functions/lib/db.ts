// Data access layer for the WebAuthn auth service (Cloudflare D1).
//
// Every function takes the D1 handle explicitly (no module globals), which
// keeps the layer unit-testable against miniflare's local D1 emulation and
// the real binding identical in production.
//
// Schema: migrations/0001_init.sql
//   users        — one row per passkey account (id = WebAuthn userHandle)
//   credentials  — WebAuthn public keys; counter updated atomically
//   api_keys     — SHA-256 hashes only; plaintext shown exactly once
//   challenges   — WebAuthn ceremony state; TTL + single-use

export interface UserRow {
  id: string
  display_name: string | null
  created_at: number
  // Operator-set ban flag: banned users cannot log in and their API keys
  // stop verifying (findActiveKeyByHash joins users and filters banned).
  banned: number
}

export interface CredentialRow {
  id: string
  user_id: string
  public_key: string
  counter: number
  transports: string | null
  created_at: number
}

export interface ApiKeyRow {
  id: string
  user_id: string
  name: string | null
  secret_hash: string
  created_at: number
  last_used_at: number | null
  revoked: number
}

export interface ChallengeRow {
  id: string
  type: 'register' | 'login'
  challenge: string
  user_id: string | null
  user_handle: string | null
  expires_at: number
}

const USER_COLS = 'id, display_name, created_at'
// SELECT list includes the ban flag (INSERTs omit it — the column defaults to 0).
const USER_SELECT_COLS = 'id, display_name, created_at, banned'
const CRED_COLS = 'id, user_id, public_key, counter, transports, created_at'
const KEY_COLS = 'id, user_id, name, secret_hash, created_at, last_used_at, revoked'
// Same columns, qualified for the api_keys↔users JOIN in findActiveKeyByHash.
const KEY_COLS_ALIASED = 'k.id, k.user_id, k.name, k.secret_hash, k.created_at, k.last_used_at, k.revoked'

// ---- Users ---------------------------------------------------------------

export async function createUser(
  db: D1Database,
  user: { id: string; displayName?: string; createdAt: number },
): Promise<void> {
  await db
    .prepare(`INSERT INTO users (${USER_COLS}) VALUES (?1, ?2, ?3)`)
    .bind(user.id, user.displayName ?? null, user.createdAt)
    .run()
}

export async function getUser(db: D1Database, id: string): Promise<UserRow | null> {
  return (
    (await db.prepare(`SELECT ${USER_SELECT_COLS} FROM users WHERE id = ?1`).bind(id).first<UserRow>()) ??
    null
  )
}

// Operator hook: set or clear the ban flag. Returns false when the user
// doesn't exist. Banned users can't log in and their keys stop verifying.
export async function setUserBanned(db: D1Database, id: string, banned: boolean): Promise<boolean> {
  const res = await db.prepare('UPDATE users SET banned = ?1 WHERE id = ?2').bind(banned ? 1 : 0, id).run()
  return res.meta.changes > 0
}

// Operator reset: passkey loss = account loss. Wipes user + credentials + keys
// atomically (D1 batch runs in one transaction).
export async function deleteUser(db: D1Database, id: string): Promise<void> {
  await db.batch([
    db.prepare('DELETE FROM api_keys WHERE user_id = ?1').bind(id),
    db.prepare('DELETE FROM credentials WHERE user_id = ?1').bind(id),
    db.prepare('DELETE FROM users WHERE id = ?1').bind(id),
  ])
}

// ---- Credentials ---------------------------------------------------------

export async function addCredential(
  db: D1Database,
  c: {
    id: string
    userId: string
    publicKey: string
    counter: number
    transports?: string[]
    createdAt: number
  },
): Promise<void> {
  await db
    .prepare(`INSERT INTO credentials (${CRED_COLS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)`)
    .bind(c.id, c.userId, c.publicKey, c.counter, c.transports ? JSON.stringify(c.transports) : null, c.createdAt)
    .run()
}

// Registration: create the user and its first credential in one transaction
// (D1 batch is atomic) so a failure can't leave a half-created account.
export async function createUserWithCredential(
  db: D1Database,
  user: { id: string; displayName?: string; createdAt: number },
  credential: { id: string; publicKey: string; counter: number; transports?: string[]; createdAt: number },
): Promise<void> {
  await db.batch([
    db
      .prepare(`INSERT INTO users (${USER_COLS}) VALUES (?1, ?2, ?3)`)
      .bind(user.id, user.displayName ?? null, user.createdAt),
    db
      .prepare(`INSERT INTO credentials (${CRED_COLS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)`)
      .bind(credential.id, user.id, credential.publicKey, credential.counter, credential.transports ? JSON.stringify(credential.transports) : null, credential.createdAt),
  ])
}

export async function listCredentialsByUser(db: D1Database, userId: string): Promise<CredentialRow[]> {
  return (
    await db
      .prepare(`SELECT ${CRED_COLS} FROM credentials WHERE user_id = ?1 ORDER BY created_at`)
      .bind(userId)
      .all<CredentialRow>()
  ).results
}

// Discoverable-credential login: the client sends only the credential id, so
// we look the credential up across all users.
export async function getCredentialById(db: D1Database, id: string): Promise<CredentialRow | null> {
  return (
    (await db.prepare(`SELECT ${CRED_COLS} FROM credentials WHERE id = ?1`).bind(id).first<CredentialRow>()) ??
    null
  )
}

// Single-statement update = atomic; prevents lost counter increments under
// concurrent assertions (the counter is the anti-clone bookkeeping).
export async function updateCredentialCounter(db: D1Database, id: string, counter: number): Promise<void> {
  await db.prepare('UPDATE credentials SET counter = ?1 WHERE id = ?2').bind(counter, id).run()
}

// ---- API keys ------------------------------------------------------------

export async function createApiKey(
  db: D1Database,
  k: { id: string; userId: string; name?: string; secretHash: string; createdAt: number },
): Promise<void> {
  await db
    .prepare(`INSERT INTO api_keys (${KEY_COLS}) VALUES (?1, ?2, ?3, ?4, ?5, NULL, 0)`)
    .bind(k.id, k.userId, k.name ?? null, k.secretHash, k.createdAt)
    .run()
}

// Metadata only — the secret itself is never stored, so it can never be listed.
export async function listApiKeysByUser(db: D1Database, userId: string): Promise<ApiKeyRow[]> {
  return (
    await db
      .prepare(`SELECT ${KEY_COLS} FROM api_keys WHERE user_id = ?1 ORDER BY created_at DESC`)
      .bind(userId)
      .all<ApiKeyRow>()
  ).results
}

// Verify path: hash lookup, revoked keys are invisible (revocation is a soft
// flag, effective immediately — no cache to invalidate). Also invisible: keys
// owned by a banned user — banning an account kills its existing keys in the
// same query, no per-key bookkeeping.
export async function findActiveKeyByHash(db: D1Database, secretHash: string): Promise<ApiKeyRow | null> {
  return (
    (await db
      .prepare(
        `SELECT ${KEY_COLS_ALIASED}
         FROM api_keys k JOIN users u ON u.id = k.user_id
         WHERE k.secret_hash = ?1 AND k.revoked = 0 AND u.banned = 0`,
      )
      .bind(secretHash)
      .first<ApiKeyRow>()) ?? null
  )
}

// Scoped to the key's owner so one user can't revoke another's key.
export async function revokeKey(db: D1Database, id: string, userId: string): Promise<boolean> {
  const res = await db
    .prepare('UPDATE api_keys SET revoked = 1 WHERE id = ?1 AND user_id = ?2')
    .bind(id, userId)
    .run()
  return res.meta.changes > 0
}

export async function touchKeyLastUsed(db: D1Database, id: string, at: number = Date.now()): Promise<void> {
  await db.prepare('UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2').bind(at, id).run()
}

// ---- Challenges ----------------------------------------------------------

export async function insertChallenge(
  db: D1Database,
  c: {
    id: string
    type: 'register' | 'login'
    challenge: string
    userId?: string
    userHandle?: string
    expiresAt: number
  },
): Promise<void> {
  await db
    .prepare('INSERT INTO challenges (id, type, challenge, user_id, user_handle, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)')
    .bind(c.id, c.type, c.challenge, c.userId ?? null, c.userHandle ?? null, c.expiresAt)
    .run()
}

export async function getChallenge(db: D1Database, id: string): Promise<ChallengeRow | null> {
  return (
    (await db
      .prepare('SELECT id, type, challenge, user_id, user_handle, expires_at FROM challenges WHERE id = ?1')
      .bind(id)
      .first<ChallengeRow>()) ?? null
  )
}

// Single-use: DELETE is the consume step; returns false on a second attempt.
export async function consumeChallenge(db: D1Database, id: string): Promise<boolean> {
  const res = await db.prepare('DELETE FROM challenges WHERE id = ?1').bind(id).run()
  return res.meta.changes > 0
}

// ---- Crypto helpers ------------------------------------------------------

// SHA-256 hex of a secret. API keys are stored hashed; the plaintext exists
// only in the one HTTPS response that mints it.
export async function hashSecret(secret: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(secret))
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('')
}

// Constant-time comparison for equal-length strings (hash comparison — we
// never compare raw secrets, only their digests).
export function timingSafeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false
  let out = 0
  for (let i = 0; i < a.length; i++) out |= a.charCodeAt(i) ^ b.charCodeAt(i)
  return out === 0
}

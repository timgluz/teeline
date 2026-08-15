// API-key minting helpers.
//
// Key format: ak_ + base64url(32 CSPRNG bytes) ≈ 46 chars. The plaintext is
// returned to the caller exactly once; only its SHA-256 hash is stored (see
// db.ts). `ak_` prefix is kept for continuity with the old Clerk keys.
import { hashSecret } from './db'

export function newKeyId(): string {
  return `key_${crypto.randomUUID()}`
}

export function generateKeySecret(): string {
  const bytes = new Uint8Array(32)
  crypto.getRandomValues(bytes)
  let bin = ''
  for (const b of bytes) bin += String.fromCharCode(b)
  const b64 = btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
  return `ak_${b64}`
}

export function isKeySecretShape(v: unknown): v is string {
  return typeof v === 'string' && v.startsWith('ak_') && v.length >= 10
}

export { hashSecret }

-- WebAuthn auth schema (design: decisions/webauthn-auth-design)
-- users: one row per passkey account (userHandle = users.id)
CREATE TABLE IF NOT EXISTS users (
  id           TEXT PRIMARY KEY,          -- uuid v4 (WebAuthn userHandle)
  display_name TEXT,                      -- optional, set at registration
  created_at   INTEGER NOT NULL
);

-- credentials: WebAuthn public keys; counter must be updated atomically
-- (anti-clone detection — SimpleWebAuthn validates, we persist the new value)
CREATE TABLE IF NOT EXISTS credentials (
  id         TEXT PRIMARY KEY,            -- base64url credentialId
  user_id    TEXT NOT NULL REFERENCES users(id),
  public_key TEXT NOT NULL,               -- base64url COSE public key
  counter    INTEGER NOT NULL DEFAULT 0,
  transports TEXT,                        -- JSON array
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_credentials_user ON credentials(user_id);

-- api_keys: only SHA-256 hashes at rest; plaintext shown exactly once
CREATE TABLE IF NOT EXISTS api_keys (
  id           TEXT PRIMARY KEY,          -- "key_" + uuid (management id)
  user_id      TEXT NOT NULL REFERENCES users(id),
  name         TEXT,                      -- optional label ("my-laptop")
  secret_hash  TEXT NOT NULL,             -- SHA-256(secret), hex
  created_at   INTEGER NOT NULL,
  last_used_at INTEGER,
  revoked      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_keys_user ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_keys_hash ON api_keys(secret_hash);

-- challenges: WebAuthn ceremony state; TTL 300 s, single-use (DELETE on consume)
CREATE TABLE IF NOT EXISTS challenges (
  id          TEXT PRIMARY KEY,           -- random nonce returned to client
  type        TEXT NOT NULL,              -- 'register' | 'login'
  challenge   TEXT NOT NULL,
  user_id     TEXT,                       -- NULL for register (no user yet)
  user_handle TEXT,                       -- register only
  expires_at  INTEGER NOT NULL
);

---
id: 01a004bf-067a-7af0-ae9c-668dc0b8f347
slug: decisions/webauthn-auth-design
title: "WebAuthn Auth: Design & Implementation Plan"
type: spec
status: active
priority: medium
---

# Teeline Auth: WebAuthn (passkeys) + self-serve API keys — Design & Implementation Plan

**Status:** Final design (supersedes `docs/webauthn-auth-investigation.md`)
**Date:** 2026-08-15

---

## 0. Locked-in decisions

| Decision | Choice |
|---|---|
| Auth hosting | **Pages Functions on tspsolver.com** (same origin as `/api-key/`, `rpID = tspsolver.com`, no CORS) |
| Registration | **Open** — anyone can create a passkey account and mint keys (matches today's Clerk self-serve) |
| Repo layout | **Inside `teeline-web`** — `teeline-web/functions/` (server) + `teeline-web/src/` (client) |
| Sessions | **Signed stateless cookie, 30-day sliding** (`HttpOnly; Secure; SameSite=Strict`, `__Host-` prefix) |
| Key generation | **Session-only** — no re-prompt of the passkey |
| Recovery | **None** — passkey loss = account loss; operator can wipe a user via `wrangler d1 execute` |
| Backend | `teeline-api` stays on **Fly.io**; verifies keys by calling our Worker (same contract as Clerk today) |
| Break-glass `API_KEY` | Kept, **behind an explicit feature toggle** (`TEELINE_AUTH_MODE`), used by local-dev & CI |
| Migration | **Hard cut, no dual-verify** (no users; Clerk never exposes raw keys anyway) |
| Gotchas | Accepted (zero users today) |

---

## 1. Target architecture

```
Browser — https://tspsolver.com/api-key/   (Astro page + Preact island)
   │  navigator.credentials.create() / .get()          (WebAuthn ceremony, same origin)
   ▼
Cloudflare Pages Functions  (teeline-web/functions/api/auth/*)
   ├─ POST /register/begin · /register/complete        (SimpleWebAuthn verify)
   ├─ POST /login/begin    · /login/complete           (SimpleWebAuthn verify, discoverable creds)
   ├─ GET  /me · POST /logout
   ├─ POST /keys            → mint ak_…, store SHA-256 hash, return plaintext ONCE
   ├─ GET  /keys            → metadata only
   ├─ DELETE /keys/:id      → revoke (immediate, transactional)
   └─ POST /keys/verify     → internal (X-Auth-Secret), {subject, revoked, expired}
          │
          ▼
Cloudflare D1 (SQLite)  — users · credentials · api_keys · challenges
          │
          ▼ (HTTPS, shared secret)
teeline-api (Fly.io, Rust/axum — unchanged surface)
   ApiKeyVerifier::verify(key) → POST {AUTH_SERVICE_URL}/api/auth/keys/verify
```

**Why D1 instead of Durable Objects** (refines the investigation): D1 gives the same per-record atomicity (transactions for counter updates and revocation) and a credential→user lookup (`WHERE id = ?`) for discoverable-credential login with zero coordination code — simpler to build, test and inspect than a per-user DO plus a credential-index registry. D1's write serialization is a non-issue at this scale; the DO-per-user design remains the documented scale-out path if traffic ever demands it.

---

## 2. Data model (D1)

```sql
CREATE TABLE users (
  id           TEXT PRIMARY KEY,          -- uuid v4 (WebAuthn userHandle)
  display_name TEXT,                      -- optional, set at registration
  created_at   INTEGER NOT NULL
);

CREATE TABLE credentials (
  id         TEXT PRIMARY KEY,            -- base64url credentialId
  user_id    TEXT NOT NULL REFERENCES users(id),
  public_key TEXT NOT NULL,               -- base64url COSE public key
  counter    INTEGER NOT NULL DEFAULT 0,  -- anti-clone; atomic update
  transports TEXT,                        -- JSON array
  created_at INTEGER NOT NULL
);
CREATE INDEX idx_credentials_user ON credentials(user_id);

CREATE TABLE api_keys (
  id           TEXT PRIMARY KEY,          -- "key_" + uuid (management id)
  user_id      TEXT NOT NULL REFERENCES users(id),
  name         TEXT,                      -- optional label ("my-laptop")
  secret_hash  TEXT NOT NULL,             -- SHA-256(secret), hex
  created_at   INTEGER NOT NULL,
  last_used_at INTEGER,
  revoked      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_keys_user ON api_keys(user_id);
CREATE INDEX idx_keys_hash ON api_keys(secret_hash);

CREATE TABLE challenges (
  id          TEXT PRIMARY KEY,           -- random nonce returned to client
  type        TEXT NOT NULL,              -- 'register' | 'login'
  challenge   TEXT NOT NULL,
  user_id     TEXT,                       -- NULL for register (no user yet)
  user_handle TEXT,                       -- register only
  expires_at  INTEGER NOT NULL            -- TTL 300s; single-use (DELETE on consume)
);
```

Migrations live in `teeline-web/migrations/0001_init.sql` (wrangler D1 migrations).

---

## 3. Auth service endpoints (Pages Functions)

All under `/api/auth/`, same origin `https://tspsolver.com`, `rpID = "tspsolver.com"`, `expectedOrigin = https://tspsolver.com` (dev: `http://localhost:*` via env `ALLOWED_ORIGINS`).

| Endpoint | Auth | Body | Response / effect |
|---|---|---|---|
| `POST /register/begin` | – | `{displayName?}` | `{options: {challenge, rp, user{id,name,displayName}, pubKeyCredParams, authenticatorSelection{residentKey:'required', userVerification:'preferred'}, attestation:'none'}, nonce}` — challenge row stored in D1 (TTL 300s) |
| `POST /register/complete` | nonce | `{nonce, credential}` | `verifyRegistrationResponse` → create user + credential in one transaction → set session cookie → `{user}` |
| `POST /login/begin` | – | `{}` | `{options: {challenge, rpId, allowCredentials: [], userVerification:'preferred'}, nonce}` (discoverable credentials — no username) |
| `POST /login/complete` | nonce | `{nonce, credential}` | `verifyAuthenticationResponse` (credential lookup by id across all users) → **atomic counter update** → set session cookie → `{user}` |
| `GET /me` | session | – | `{user}` or 401 |
| `POST /logout` | session | – | clears cookie |
| `POST /keys` | session | `{name?}` | mint `ak_`+base64url(32B) → store SHA-256 + metadata → `{id, name, secret, createdAt}` — **secret returned exactly once** |
| `GET /keys` | session | – | `[{id, name, createdAt, lastUsedAt, revoked}]` — never the secret |
| `DELETE /keys/:id` | session | – | soft revoke (transactional, immediate) |
| `POST /keys/verify` | `X-Auth-Secret` | `{secret}` | constant-time SHA-256 compare → `{subject, revoked, expired}` (mirrors today's Clerk contract so the Rust side barely changes) |

Notes:
- Sessions are **stateless signed cookies**: HMAC-SHA256 over `{sub, iat, exp}` with `SESSION_SECRET`; sliding renewal when >15 days old; `__Host-teeline-session`; `SameSite=Strict` + mutating endpoints also require `X-Requested-With` header (CSRF).
- Registration is open: first passkey on the origin creates the account. Multiple passkeys per account is a documented follow-up (synced passkeys make one credential work across devices in practice).
- Rate limiting on auth endpoints (in-Worker, e.g. per-IP token bucket): register/begin+complete, login/begin+complete, keys/verify.

## 4. API key format & lifecycle

- `ak_` + base64url(32 random bytes) ≈ 46 chars — keeps the existing prefix so docs/tools/`passes_shape_check` keep working.
- At rest: **SHA-256 only**; plaintext exists only in the one HTTPS response (never logged).
- Verify: hash presented secret, constant-time compare against `secret_hash`, join to `users` for subject, honor `revoked`.
- `expired` is always `false` in v1 (no expiry feature) — field kept for contract parity with Clerk.
- Show-once UX: the key is held in the Preact island's state only (deliberately **not** `sessionStorage`); a page refresh destroys it; reminder banner + "I've saved it" acknowledgement.

## 5. teeline-api changes (Fly.io, Rust)

- Replace `ClerkVerifier` with `ServiceVerifier` in `teeline-api/src/clerk.rs` (same `ApiKeyVerifier` trait, same `VerifyResponse` shape + `decide()` logic):
  - `POST {AUTH_SERVICE_URL}/api/auth/keys/verify` with `X-Auth-Secret: {AUTH_SERVICE_SECRET}` header, 3 s timeout, `ak_` shape check retained.
- Env contract (`teeline-api/src/main.rs`):
  - `TEELINE_AUTH_MODE` — explicit toggle:
    - `breakglass` → only static `API_KEY` (used by **local-dev & CI**)
    - `service` → `ServiceVerifier` (+ optional break-glass `API_KEY`)
    - `disabled` / unset with no other config → auth middleware off (back-compat with the no-auth MVP behavior)
  - `AUTH_SERVICE_URL`, `AUTH_SERVICE_SECRET` (service mode); `API_KEY` (breakglass/service modes).
  - `CLERK_SECRET_KEY` removed.
- `require_auth` middleware, `token_matches` (constant-time), `/api/v1/health` exemption, tower-governor rate limit: **unchanged**.

## 6. Web UI (`/api-key/`)

- Keep the Astro page (SEO copy, curl usage examples) but mount a **Preact island `<ApiKeyManager />`** replacing the "go to Clerk" instructions.
- Island states:
  1. **Anonymous** → "Sign in with passkey" button (creates passkey on first visit; `navigator.credentials.create` / `.get` — native, no client lib).
  2. **Signed in** → "Generate API key" + keys list (name, created, last used, revoke button).
  3. **Key generated** → show-once box + "Copy it into your password manager now — it won't be shown again" + "I've saved it" (dismisses to the list).
- New client modules: `teeline-web/src/auth/webauthn.ts` (ceremony helpers), `teeline-web/src/auth/api.ts` (fetch wrapper incl. CSRF header).
- Nav/docs copy updated to describe the new flow.

## 7. Files touched (new / changed)

```
new  teeline-web/functions/api/auth/register.ts      (begin+complete)
new  teeline-web/functions/api/auth/login.ts         (begin+complete)
new  teeline-web/functions/api/auth/keys.ts          (POST/GET/DELETE)
new  teeline-web/functions/api/auth/verify.ts        (internal)
new  teeline-web/functions/api/auth/me.ts, logout.ts
new  teeline-web/functions/lib/webauthn.ts           (SimpleWebAuthn wrappers, origin/rpID config)
new  teeline-web/functions/lib/session.ts            (sign/verify cookie)
new  teeline-web/functions/lib/db.ts                 (D1 access, SQL)
new  teeline-web/migrations/0001_init.sql
new  teeline-web/src/auth/webauthn.ts, api.ts
new  teeline-web/src/components/ApiKeyManager.tsx    (Preact island)
new  teeline-web/src/api-key-manager.test.ts(x)      (vitest, miniflare)
new  teeline-web/e2e/auth.spec.ts                    (Playwright + WebAuthn virtual authenticator)
chg teeline-web/wrangler.toml                        (d1 binding, migrations, vars)
chg teeline-web/package.json                         (+ @simplewebauthn/server)
chg teeline-web/src/pages/api-key/index.astro        (mount island, new copy)
chg teeline-api/src/clerk.rs                         (ClerkVerifier → ServiceVerifier)
chg teeline-api/src/main.rs                          (env/toggle wiring)
chg teeline-api/tests/api_tests.rs                   (mode-aware test setup)
chg .github/workflows/deploy-web.yml                 (d1 migrations apply on deploy)
chg .github/workflows/teeline-web.yml                (functions build gate)
chg .github/workflows/rust.yml                       (if needed for new envs)
del Clerk decommission: accounts.tspsolver.com DNS, CLERK_SECRET_KEY, docs copy
```

## 8. Deployment & CI/CD

- `wrangler.toml`: `d1_databases` binding (`teeline-auth`), `SESSION_SECRET`/`AUTH_SERVICE_SECRET`/`ALLOWED_ORIGINS` as Pages project secrets/vars.
- `deploy-web.yml`: existing `wrangler pages deploy dist/` picks up `functions/` automatically; add `wrangler d1 migrations apply teeline-auth --remote` before/after deploy.
- `teeline-web.yml`: add a build gate proving the Functions bundle compiles (`wrangler deploy --dry-run` or `wrangler pages functions build`) + pin/verify the SimpleWebAuthn version (edge-build regressions have happened — [8.3.4](https://github.com/MasterKale/SimpleWebAuthn/issues/471)).
- Fly.io: set `AUTH_SERVICE_URL` + `AUTH_SERVICE_SECRET`, unset `CLERK_SECRET_KEY`, `TEELINE_AUTH_MODE=service`.
- Local-dev & CI: `TEELINE_AUTH_MODE=breakglass` + `API_KEY` (Rust tests already use a static test token + `NullVerifier`).

## 9. Clerk decommission (hard cut, no users)

1. Flip `TEELINE_AUTH_MODE` to `service` on Fly; verify live; keep `CLERK_SECRET_KEY` unset.
2. Delete `accounts.tspsolver.com` DNS record; remove Clerk billing.
3. Delete `ClerkVerifier` code + `CLERK_SECRET_KEY` refs.
4. Update `/api-key/` page, docs, release notes: existing `ak_` keys are invalid — regenerate via the new flow.

## 10. Testing

- **Unit (vitest + miniflare):** challenge TTL/single-use, session sign/verify, key hash/constant-time compare, register/login complete handlers with mocked SimpleWebAuthn, verify endpoint (valid/revoked/unknown).
- **Rust:** `ServiceVerifier` unit tests (shape check, `decide`), handler test against a local stub HTTP server; existing auth tests unchanged (`breakglass` mode).
- **E2E (Playwright, Chromium):** CDP **WebAuthn virtual authenticator** — full flow: register → generate key → copy shown-once → refresh wipes it → sign-in again → list/revoke → API `verify` against a stub; plus curl-style `X-Api-Key` request to a local teeline-api instance.

## 11. Implementation plan (phases)

| Phase | Scope | Deliverable | Est. |
|---|---|---|---|
| **0 — Scaffold** | `functions/` dir + D1 binding + migrations runner; add `@simplewebauthn/server` (pin known-good version); `wrangler dev` works locally; build-gate in CI | Empty service that deploys | 0.5 d |
| **1 — Data layer** | `migrations/0001_init.sql`, `functions/lib/db.ts` (users/credentials/api_keys/challenges CRUD, transactions, constant-time hash) | Unit-tested data access | 0.5 d |
| **2 — WebAuthn** | register/login begin+complete (SimpleWebAuthn), session cookie, `/me`, `/logout`, origin config | Sign in/up with passkey end-to-end | 1–1.5 d |
| **3 — API keys** | `/keys` POST/GET/DELETE, show-once response, `/keys/verify` internal | Key lifecycle + verify contract | 0.5–1 d |
| **4 — Web UI** | `<ApiKeyManager />` island + client auth modules, page copy | Full user flow on tspsolver.com | 1 d |
| **5 — teeline-api** | `ServiceVerifier`, `TEELINE_AUTH_MODE` toggle, env/docs | Fly.io verifies via Worker | 0.5 d |
| **6 — Deploy & e2e** | deploy-web.yml (d1 migrations), secrets, Playwright WebAuthn e2e | Green deploy + e2e | 1 d |
| **7 — Decommission** | remove Clerk code/DNS/keys, docs, changelog | Clerk fully gone | 0.5 d |
| **8 — Hardening** | auth rate limits, audit logging, session-secret rotation doc, security review | Ship-ready | 0.5–1 d |

**Total ≈ 6–8 focused dev-days** (~1.5–2 calendar weeks with review).

## 12. Risks & open items

- **SimpleWebAuthn version/build compat** — pin + CI build gate (Phase 0).
- **Passkey loss = account loss** — accepted; operator reset = `wrangler d1 execute` delete user row(s); document.
- **D1 write serialization** — fine at this scale; DO-per-user is the documented scale path.
- **Playwright virtual authenticator is Chromium-only** — acceptable (e2e covers the flow; manual smoke on Safari/Firefox).
- **Verify endpoint abuse** — rate-limit + shared secret + shape check.
- **Session secret rotation** — document procedure; cookie invalidation on rotation is acceptable (re-login).
- **Open registration + abuse** — no users today; revisit quotas when it matters.

Follow-ups (post-v1): multiple passkeys per account, API-key expiry, per-key usage quotas, audit log, optional `allowed_origins` for the API.

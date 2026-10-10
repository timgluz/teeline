---
id: 01a004be-9c4d-7290-b46c-3542a7931570
slug: decisions/adr-007-webauthn-auth
title: "ADR-007: Replace Clerk with WebAuthn (passkeys) + self-serve API keys"
type: adr
status: active
priority: medium
tags: [adr, auth, webauthn, api-keys, cloudflare]
---

# ADR-007: Replace Clerk with WebAuthn (passkeys) + self-serve API keys

## Status

Accepted (2026-08-15). Implementation plan + full design: `decisions/webauthn-auth-design`.

## Context

- `teeline-api` (Rust/axum, Fly.io) authenticates every `/api/v1/*` request (except `/health`)
  via the `ApiKeyVerifier` trait. Today that is `ClerkVerifier`, which POSTs the presented
  `ak_…` key to Clerk's Backend API (`/v1/api_keys/verify`, `CLERK_SECRET_KEY`), accepting a
  `{subject, revoked, expired}` response; a static break-glass `API_KEY` is also accepted.
- `teeline-web`'s `/api-key/` page is static and pushes users to Clerk-hosted
  `accounts.tspsolver.com` (sign-in via GitHub/GitLab/Google/email; key creation in Clerk's UI).
- Clerk is a paid third-party IdP. The platform currently has **zero users**.
- Goal: drop Clerk. Identity = WebAuthn passkey (no password, no third-party IdP). User signs
  in on tspsolver.com, clicks **Generate API key**, gets `ak_…` shown exactly once (until page
  refresh), and is reminded to store it in a password manager. Auth state lives in Cloudflare.

## Decision

1. **Hosting:** auth service = Cloudflare **Pages Functions on tspsolver.com** (same origin as
   the page ⇒ `rpID = tspsolver.com`, no CORS). Code lives in `teeline-web/functions/`.
2. **Storage:** Cloudflare **D1** (SQLite): `users`, `credentials` (atomic WebAuthn counter),
   `api_keys` (SHA-256 hashes only), `challenges` (TTL 300 s, single-use). Durable-Object
   per-user kept as the documented scale-out path, not the v1 choice.
3. **Library:** `@simplewebauthn/server` (official Cloudflare Workers support since v8.0.0
   ESM build); client uses native `navigator.credentials`.
4. **Session:** stateless signed cookie (HMAC-SHA256, `__Host-`, `HttpOnly; Secure;
   SameSite=Strict`), 30-day sliding; CSRF defense = SameSite=Strict + `X-Requested-With`.
5. **API keys:** `ak_` + base64url(32 CSPRNG bytes); SHA-256 at rest (constant-time compare);
   plaintext returned **exactly once**; show-once UI (island state only — refresh wipes it);
   revocable; `expired` field kept for contract parity with Clerk.
6. **Registration:** open — anyone with a passkey can create an account and mint keys
   (matches today's Clerk self-serve).
7. **teeline-api stays on Fly.io:** `ClerkVerifier` → `ServiceVerifier` calling
   `POST {AUTH_SERVICE_URL}/api/auth/keys/verify` with `X-Auth-Secret` — same
   `{subject, revoked, expired}` contract, so the middleware is untouched.
8. **Break-glass `API_KEY`** kept behind an explicit `TEELINE_AUTH_MODE` toggle:
   `breakglass` (local-dev & CI) / `service` (prod) / `disabled`.
9. **Cut-over:** hard cut, no dual-verify (zero users; Clerk never exposes raw key secrets,
   so migration is impossible anyway).

## Consequences

- Passkey loss = account loss (accepted; operator reset via `wrangler d1 execute`).
- Existing `ak_` keys are invalid after cut-over — users regenerate via the new flow.
- No third-party IdP dependency or billing; auth becomes self-hosted on existing Cloudflare.
- SimpleWebAuthn version must be pinned + a CI build gate added (edge-build regressions have
  happened, e.g. 8.3.4).
- The `/api-key/` page becomes interactive (Preact island) instead of static instructions.

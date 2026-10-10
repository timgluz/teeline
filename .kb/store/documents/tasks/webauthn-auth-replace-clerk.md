---
id: 01a004bf-5da8-7722-9400-a265afff5984
slug: tasks/webauthn-auth-replace-clerk
title: "Replace Clerk with WebAuthn + self-serve API keys"
type: task
status: completed
priority: high
tags: [auth, webauthn, api-keys, cloudflare, d1, 4077/, 455/]
---

# Replace Clerk with WebAuthn + self-serve API keys

Design + decision: `decisions/adr-007-webauthn-auth`, `decisions/webauthn-auth-design`.
Status: planned — not started. Zero users today; gotchas accepted (passkey loss = account loss).

## Phases

- [x] **Phase 0 — Scaffold (done, commit cbf27dc):** `teeline-web/functions/` + D1 binding +
      migrations runner; `@simplewebauthn/server` pinned 13.3.2 (bundles + runs on Workers,
      verified); `wrangler pages dev` works locally; CI build gate added (teeline-web.yml).
      Also landed early: remote D1 created + migrations applied; deploy-web.yml applies
      migrations before Pages deploy (commit fd29cd3).
- [x] **Phase 1 — Data layer (done, commit a542477 / PR #458):** `migrations/0001_init.sql`
      (Phase 0); `functions/lib/db.ts` (transactions, atomic counter, scoped revocation,
      SHA-256 hash + constant-time compare, single-use challenges); `functions/tsconfig.json`
      + workers-types typecheck; 12 unit tests via better-sqlite3 D1 shim (miniflare D1
      emulation broken with wrangler 4.119's workerd — workers-sdk#4077/#10114).
- [x] **Phase 2 — WebAuthn (done, commit f443fa4 / PR #459):** register/login begin+complete
      (SimpleWebAuthn), HMAC session cookie (__Host-, 30 d sliding), `/me`, `/logout`, rpID/origin
      policy; 25 new tests (295 total); runtime smoke OK (me→401, fail-closed without secret).
- [x] **Phase 3 — API keys (done, commits f8c166f+79a9525 / PR #460):** `/keys` POST/GET/DELETE
      (show-once `ak_` mint, SHA-256 at rest, owner-scoped revoke), `/keys/verify` internal
      (X-Auth-Secret; Clerk-shaped contract), shared `requireSession` lib; 12 tests; OCR-reviewed.
- [x] **Phase 4 — Web UI (done, commit 7281cab / PR #461):** Preact `<ApiKeyManager />` island +
      `src/auth/{webauthn,api}.ts`, show-once key UX, dev proxy /api/auth → wrangler pages dev.
- [x] **Phase 5 — teeline-api (done, PR #462 merged 935ea0b):** `ServiceVerifier`
      (replaces `ClerkVerifier`), `TEELINE_AUTH_MODE` toggle (breakglass/service/disabled +
      inference), AUTH_SERVICE_URL/SECRET, README; 24 OCR findings applied; 12 test suites green.
      API deployed to Fly (breakglass mode until Fly secrets set).
- [x] **Phase 6 — e2e (done, commit 4fc4df1 / PR #463):** Playwright WebAuthn virtual-authenticator
      e2e (register→mint→show-once→refresh-wipes→login→revoke→verify) via a dedicated config +
      web-e2e.yml CI. Deploy/migrations/secrets were landed in Phases 0-5. Also fixed a
      production-breaking client bug the e2e caught (helpers returned the { user } envelope, not the
      User object — live registration would crash on user.id.slice).
- [x] **Phase 7 — Decommission (COMPLETE 2026-08-15):** repo side DONE (sweep clean — all remaining `clerk` mentions are
      intentional historical comments; code removed in Phase 5; docs updated in Phase 5/6; Scalar
      link fixed 7f785ff). Remaining are **operator steps** (user-side):
  1. ✅ `fly secrets unset CLERK_SECRET_KEY --app teeline-api` (done 2026-08-15)
  2. ✅ Delete the `accounts.tspsolver.com` DNS record (Cloudflare, tspsolver.com zone) — done
     2026-08-15, hostname no longer resolves
  3. ✅ Clerk application deleted (dashboard.clerk.com) — billing stopped
  - Sweep performed 2026-08-15: only contextual comments remain (verified).
- [x] **Phase 8 — Hardening (done, commits 6c0364b+f4f31bc / PR #464):** per-IP D1-backed rate
      limiting on all auth endpoints (atomic batch, fail-open, 429/Retry-After), audit logs with
      client IP, session-rotation note (README), 7 OCR findings applied. Session-secret rotation
      doc: teeline-web/README (rotate → all sessions invalid, re-login).

Total ≈ 6–8 focused dev-days.

## Definition of done

- Passkey register + sign-in on tspsolver.com/api-key/ (fresh + returning device)
- Generate API key → shown once, wiped on refresh, "save to password manager" reminder
- Revoke works; teeline-api verifies `ak_` keys via the Worker (Fly.io, `service` mode)
- Local-dev & CI use `TEELINE_AUTH_MODE=breakglass`; Rust tests green
- Clerk fully removed; docs updated; e2e green

## Deployment runbook (fresh environment — secrets for Fly.io + Cloudflare)

Recorded so the auth infra can be re-set-up from scratch (see also
`teeline-api/README.md` + `teeline-web/README.md`):

```bash
# Cloudflare (from teeline-web/): create auth D1 + schema + secrets
npx wrangler d1 create teeline-auth                 # paste database_id into wrangler.toml
npx wrangler d1 migrations apply teeline-auth --remote
echo "<AUTH_SERVICE_SECRET>" | npx wrangler pages secret put AUTH_SERVICE_SECRET --project-name teeline-web
echo "<SESSION_SECRET>"      | npx wrangler pages secret put SESSION_SECRET      --project-name teeline-web

# Fly: point the API at the auth service (AUTH_SERVICE_SECRET MUST match Pages)
fly secrets set TEELINE_AUTH_MODE=service AUTH_SERVICE_URL=https://tspsolver.com \
  AUTH_SERVICE_SECRET=<same-value> --app teeline-api
fly secrets unset CLERK_SECRET_KEY --app teeline-api

# Deploy: task web:deploy:release (migrations run first) + task api:release
# Local-dev/CI: TEELINE_AUTH_MODE=breakglass + API_KEY only.
```

Verification runbook (live checks + troubleshooting): `docs/auth-verification-runbook.md`.

## Completion evidence (2026-08-15)

- All 9 phases shipped via PRs #455/#456 (scaffold), #458 (data), #459 (WebAuthn), #460 (keys),
  #461 (UI), #462 (teeline-api), #463 (e2e), #464 (hardening), #465 (ban flag); master at 5b6f9cb.
- Live verification: user registered with a passkey on tspsolver.com and accessed auth-only API
  endpoints (2026-08-15) — end-to-end green.
- Operator follow-up landed: `tasks/banned-flag` (completed).
- Story post material for the blog: `tasks/blog-passkeys-instead-of-clerk`.

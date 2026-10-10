---
id: 01a0067d-f8fa-7343-91c7-d74619ba3dbc
slug: tasks/banned-flag
title: "Account-level banned flag (operator abuse control)"
type: task
status: completed
priority: medium
tags: [auth, webauthn, cloudflare, d1, moderation]
---

# Account-level banned flag (operator abuse control)

Follow-up to the WebAuthn auth migration (`tasks/webauthn-auth-replace-clerk`). Zero users today,
but an abusive user's account can now be blocked without deleting data.

## What landed (PR #465, branch `feat/ban-flag`)

- **Migration `0003_add_banned.sql`:** `users.banned INTEGER NOT NULL DEFAULT 0`.
- **Data layer (`functions/lib/db.ts`):** `banned` in `UserRow` + `USER_SELECT_COLS`;
  `setUserBanned(db, id, banned)` operator helper; `findActiveKeyByHash` now JOINs users and
  filters `u.banned = 0` — banning kills existing keys in one query, no per-key bookkeeping.
- **Enforcement:**
  - `requireSession` (lib/auth.ts) → 403 `Account is banned` for session APIs
    (`/api/auth/me`, `/api/auth/keys` create/list/revoke).
  - `/api/auth/login/complete` → 403, no session cookie; credential counter still advances so an
    eventual unban doesn't hit a stale counter.
  - `/api/auth/keys/verify` → 404 for banned users' keys (doesn't leak ban status; teeline-api
    middleware rejects immediately).
- **Docs:** `docs/auth-verification-runbook.md` §7 — operator `wrangler d1 execute` commands
  (list / ban / unban) + caveats.
- **Tests:** 6 new/updated cases across db/keys/handlers suites; 332 total pass.
  Test harness changed: fresh in-memory DB per test (migrations are no longer idempotent —
  0003 is a plain `ALTER TABLE ADD COLUMN`).

## Known caveat

Flag is **per account**: with open registration, a banned user can create a brand-new account with
a new passkey. A durable ban would additionally deny-list credential IDs (follow-up if needed).

## Operator workflow

```bash
# list users
npx wrangler d1 execute teeline-auth --remote --command "SELECT id, display_name, banned, created_at FROM users"
# ban / unban
npx wrangler d1 execute teeline-auth --remote --command "UPDATE users SET banned = 1 WHERE id = '<user-id>'"
npx wrangler d1 execute teeline-auth --remote --command "UPDATE users SET banned = 0 WHERE id = '<user-id>'"
```

## Completion evidence

- PR #465 squash-merged to master as `5b6f9cb` (2026-08-15).
- deploy-web.yml run `31899189774` green: `wrangler d1 migrations apply teeline-auth --remote`
  (0003 applied) then `wrangler pages deploy dist/` → live at tspsolver.com.
- All CI green on the PR (build, ci, e2e, plan, codecov, socket).
- OCR review: 1 medium finding (fragile column-alias string building) fixed before merge; 1 low on
  pre-existing shim code (out of scope).
- Manual sign-up verification (user, 2026-08-15): registration + auth-only API endpoints work with
  the new auth on the live site.


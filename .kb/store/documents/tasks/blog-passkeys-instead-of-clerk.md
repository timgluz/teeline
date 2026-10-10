---
id: 01a00502-b3f9-74a0-9886-3e9dffa5a2df
slug: tasks/blog-passkeys-instead-of-clerk
title: "Blog: Passkeys instead of Clerk (story post + recipe follow-up)"
type: task
status: active
priority: medium
tags: [blog, auth, webauthn, passkeys, cloudflare, 455/]
---

# Blog: Passkeys instead of Clerk

Story post about replacing Clerk with WebAuthn passkeys + self-serve API keys
(migration: `tasks/webauthn-auth-replace-clerk`, ADR-007).

**Decisions (agreed 2026-08-15):** angle A+C (migration story + UX/product lens), **no code** in
post #1, **~3–4 A4 pages (≈2000–2500 words)**, written **after the migration ships** (Phase 7 /
deploy). Post #2 = technical recipe (B, with code) — planned follow-up.

**Style decisions (confirmed 2026-08-15, Q&A):** first person "I" (matches existing posts);
**generic naming** — refer to "our auth provider", never name the vendor; slug renamed
accordingly (`passkeys-instead-of-a-login-provider`); 2–3 sentence teeline intro recap, then
straight into auth; **draft: true first**, user previews live, then flip to draft: false;
post #2 waits for feedback on #1.

**Where:** `teeline-web/src/content/blog/passkeys-instead-of-a-login-provider.md` (slug:
`passkeys-instead-of-a-login-provider` — renamed from `passkeys-instead-of-clerk` 2026-08-15 to
match the generic-naming decision; frontmatter: title, description, pubDate at publish, tags
[auth, webauthn, passkeys, cloudflare], draft: true until publish).

## Outline — post #1 (story, no code)

1. **Hook** — old flow: `/api-key/` sent users to *another website* (accounts.tspsolver.com) to
   sign in with GitHub/Google/email before getting a key; the whole identity stack was someone
   else's product.
2. **Why Clerk was right at first** — OAuth buttons for free, zero auth code; "use an IdP until
   you can't."
3. **The three pushes** —
   - tspsolver is **not monetized**; Clerk's passkey features sit behind a **paywall** that makes
     no sense for a non-monetized project
   - OAuth provider integration is a **maintenance matrix** (GitHub × GitLab × Google × email),
     not a feature
   - the actual job is "a developer wants an **API key**", not "an account" — identity should be
     one tap
4. **The idea** — the **passkey is the account**; browser/OS/password manager runs the ceremony,
   we store one public key. Greenfield: zero users ⇒ free to make big trade-offs.
5. **The shape** (conceptual, no code) — tap passkey → session → "Generate API key" → shown once
   → "save it in your password manager". Passkey authenticates *you*; API key authorizes *your
   scripts*; same password manager holds both.
6. **Trade-offs (honest ledger)** — passkey loss = account loss (no recovery); hard cut (Clerk
   never reveals raw keys ⇒ migration impossible anyway); edge-runtime/library constraints
   (deferred details); why consistent storage matters (counter checks, instant revocation);
   break-glass toggle for dev/CI.
7. **Gained vs lost** — IdP independence, no OAuth matrix, one-tap identity, self-hosted on
   existing Cloudflare infra ↔ managed user management, recovery story, Clerk UI.
8. **Nuance + pointer** — when we'd still use an IdP (teams, monetization, compliance); "full
   technical recipe is a follow-up post."

## Post #2 — technical recipe (follow-up, B, with code)

Ceremony flow (register/login begin+complete), D1 schema, challenge TTL/single-use storage,
session cookie, edge-bundling gotchas (SimpleWebAuthn pinned version), D1-vs-Durable-Objects
rationale. Planned after post #1.

## Definition of done (post #1)

- [x] Migration merged & deployed (task `tasks/webauthn-auth-replace-clerk` COMPLETED 2026-08-15;
      user verified live sign-up + auth-only API endpoints)
- [x] Post written ≤3000-word budget → actually ~2075 words, no code blocks
- [x] Frontmatter valid; `astro check` passes; `draft: false` set (PR #467, 2026-08-15)

**Status 2026-08-15:** post published (`draft: false`) in PR #467 (branch
`feat/blog-draft-preview`), which also carries the draft-preview route change (dev shows drafts,
prod excludes). Verified in local build: page present in `dist/blog/`, listed in `/blog/`, in
`rss.xml`. Awaiting user merge → deploy-web.yml publishes live. Post #2 (recipe, with code)
queued after #1 is live + feedback.

## Reflection notes (raw material, 2026-08-15 — post-migration)

Collected from the actual work so the draft can be honest and specific. Facts, not prose.

### Timeline (all on 2026-08-15 — a single-day migration)

- 9 PRs shipped in one day: scaffold (#455/#456) → data layer (#458) → WebAuthn ceremonies (#459)
  → API keys (#460) → Preact UI (#461) → teeline-api verifier (#462) → Playwright e2e (#463) →
  hardening: rate limits + audit logs (#464) → operator ban flag (#465). Squash-merged, deploy
  green each time, user verified live sign-up at the end.
- Why it was fast: greenfield auth (zero users), Cloudflare Pages Functions + D1 were already in
  the stack, and the ceremony libraries did the heavy lifting.

### Concrete details that make the story real

- Old flow: `/api-key/` on tspsolver.com redirected to **accounts.tspsolver.com** (a Clerk-hosted
  page) → GitHub/Google/email OAuth → back. Identity was literally another website.
- The new flow: one tap (passkey) → session → "Generate API key" → secret shown **once** → "save
  it in your password manager". Passkey authenticates the human; the API key authorizes the
  script; the same password manager holds both.
- Passkey = account: we store only the **public key** + a counter; the browser/OS/password
  manager runs the ceremony. "The passkey is the account" — no username/password rows at all.
- API keys: `ak_` prefix, SHA-256 at rest (plaintext exists only in the one minting response),
  instant soft revocation, per-IP rate limits (10/min ceremonies, 600/min verify), audit logs.
- The **ban flag** (post #465, shipped today): operator flips `banned = 1` → login 403, session
  APIs 403, existing keys 404 on verify immediately. No key rotation needed. This is a great
  closing beat: even the abuse story is self-hosted now.
- Infra: Cloudflare Pages Functions (edge) + D1 (SQLite) for storage; sessions are HMAC-signed
  cookies (`__Host-`, HttpOnly, SameSite=Strict, 30-day sliding); teeline-api (Rust/axum on Fly)
  verifies keys via an internal endpoint with a shared secret.
- Honest gotchas encountered: SimpleWebAuthn needed a pinned version to bundle on Workers; D1
  local emulation was broken in the miniflare library path (worked around with a SQLite shim in
  tests); a client-side bug (envelope unwrap) was caught by the Playwright WebAuthn e2e before it
  shipped — the e2e paid for itself.
- Big trade-offs accepted (fine because zero users): **passkey loss = account loss** (no recovery
  flow; operator can delete the account, that's it); **hard cut** — Clerk never reveals raw keys,
  so a migration was impossible anyway (no dual-run); no teams/roles/RBAC.
- "Use an IdP until you can't" — Clerk was the right call when we needed OAuth buttons with zero
  code; it became wrong when the passkey features we wanted sat behind a paywall that makes no
  sense for a non-monetized project, and the OAuth provider list (GitHub × Google × email × …) is
  a maintenance matrix, not a feature.

### Open questions for the draft (to confirm with the user)

1. First person ("I") vs "we" — existing blog posts are first-person. → **ANSWERED: "I"**
2. Mention specific names (Clerk) or generically ("our auth provider")? Existing posts name
   WebMCP, so naming is on-brand. → **ANSWERED: generic naming; never name the vendor**
3. Length target: 2000–2500 words confirmed? Existing longest post is ~2350. → **confirmed**
4. Include a short "what we built" recap of teeline itself or assume the reader knows it?
   → **ANSWERED: 2–3 sentence intro**
5. Date it as 2026-08-15 or wait; `pubDate` at publish. → **ANSWERED: draft: true first, preview,
   then flip; pubDate at publish**
6. Post #2 (recipe with code) — write immediately after, or wait for feedback on #1?
   → **ANSWERED: wait for #1 feedback**

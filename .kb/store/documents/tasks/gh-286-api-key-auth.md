---
id: 019f2c15-c403-7ff0-ae0b-e86edde6ab9e
slug: tasks/gh-286-api-key-auth
title: "GH #286: static bearer-token auth for teeline-api (+ #307 router dedup)"
type: task
status: review
priority: high
tags: [api, auth, mvp]
---

## Overview

Implements GH #286: optional static bearer-token auth for `teeline-api`, gated by
an `API_KEY` env var. Folds in GH #307 (router-assembly dedup) as a prerequisite,
since adding a second conditional layer (auth, alongside the existing rate-limit
layer) would otherwise duplicate wiring logic across `main.rs` and
`lib.rs::build_router()` — exactly the class of bug #307 warns about.

One of the last MVP-blocking tickets for `teeline-api`.

## Goals

- `API_KEY` unset → no auth required (back-compat with current MVP behavior)
- `API_KEY` set → all `/api/v1/*` requests except `GET /api/v1/health` require
  `Authorization: Bearer <token>` or `X-Api-Key: <token>`, else 401 JSON
  `{"error":"Unauthorized"}`
- Originally implemented as a Tower `Layer`/`Service` (mirroring `MetricsLayer`);
  refactored to `axum::middleware::from_fn` + `route_layer` during the code-review
  fix round (see Progress Log 2026-07-04, round 2) — simpler, and `route_layer`
  fixes the 404-vs-401 fallback bug that `.layer()` had
- Documented in the OpenAPI spec via `utoipa::Modify` + `security(...)`
- `build_router` gains a seam (`build_router(state, api)`) so route/layer wiring
  lives in one place instead of being duplicated between `main.rs` and `lib.rs`

## Acceptance Criteria

- [x] `API_KEY` unset: all requests pass through unauthenticated
- [x] `API_KEY` set: missing/wrong token → 401; `GET /api/v1/health` always exempt
- [x] Token comparison is constant-time (`subtle` crate)
- [x] OpenAPI spec documents both `bearer_token` and `api_key` security schemes
- [x] `cargo test -p teeline-api` passes (all 8 `make_app()` call sites updated
      for the new `build_router` signature, plus new auth-specific tests)
- [x] `cargo clippy --workspace -D warnings` passes
- [x] Existing `task test:e2e:api` stays green, unmodified (auth-disabled path)
- [x] New `task test:e2e:auth` suite passes (auth-enabled path, separate server)

## Context

- Plan reviewed by Opus before implementation; full plan at
  `/home/timgluz/.claude/plans/yes-let-s-continue-with-eager-zephyr.md`
- Depends on #276–#285 (all closed)
- Related: #307 (router dedup, folded in here), #308 (CI extraction, unrelated/independent)
- Implementation split into 4 subtasks: (1) router seam, (2) auth middleware core,
  (3) OpenAPI docs, (4) testing & CI — landing as one PR (compile-order dependency:
  `main.rs` in subtask 1 references `AuthLayer` from subtask 2)
- Branch: `feat/286-api-key-auth`

## Progress Log

### 2026-07-04
- Planned with user (open questions, outline, 4-subtask split), reviewed by Opus
  (caught a real blocker: 8 test files call `build_router`, not just
  `api_tests.rs`; plus a missing `S::Error: Send` bound). Plan revised and
  approved. Starting implementation.
- Implemented all 4 subtasks on `feat/286-api-key-auth`:
  1. Router seam (#307): `build_router(state, api)` in lib.rs; main.rs consumes
     it, drops duplicated route wiring + unused `MetricsLayer`/`get` imports.
  2. `AuthLayer`/`AuthService` in middleware.rs (mirrors `MetricsLayer`), `subtle`
     constant-time comparison, `ApiError::Unauthorized` in error.rs.
  3. `utoipa::Modify` `SecurityAddon` (bearer_token + api_key schemes) in
     openapi.rs; `security(...)` added to solvers/parse/solve routes.
  4. Fixed all 8 `make_app()` call sites; added 7 new auth tests to
     `api_tests.rs`; single `tests/hurl/auth/auth.hurl` using hurl `{{api_token}}`
     variable templating instead of duplicating the literal token across files
     (user's suggestion mid-implementation — cleaner than the original 3-file
     plan); new `test:e2e:auth` Taskfile task (defines token once via `vars:`,
     reuses `api:stop` for teardown per Opus's stale-server note); CI step added.
  5. Applied `cargo fmt`; fixed one clippy `collapsible_if` in `extract_token`
     using a let-chain.
- Opened PR #310. Ran `/code-review max --comment` (10 finder agents + verify +
  gap sweep); posted 11 confirmed findings as inline PR comments. Fixed all 11:
  1. **Critical**: `API_KEY=""` silently disabled auth (`.ok()` returns
     `Some("")`, and empty-vs-empty `ct_eq` matched) — fixed via
     `.filter(|t| !t.is_empty())` in `api_key()` plus defense-in-depth in
     `token_matches`. Live curl-reproduced before and after.
  2. **High**: `AuthLayer` via `.layer()` wrapped the router's fallback too, so
     ANY unmatched path anywhere returned 401 instead of 404 — contradicted an
     earlier reviewer's pure-static-reasoning claim; resolved by live curl
     repro, then fixed via `route_layer` (only wraps `path_router`, confirmed
     against vendored axum-0.8.9 source).
  3. Taskfile `{{.API_TOKEN}}` interpolated directly into a quoted `bash -c`
     string was shell-injectable (proven with a `$(touch PWNED)` payload) —
     fixed via `env:` + `$TEST_API_TOKEN` reference.
  4. Case-sensitive `Bearer ` prefix rejected RFC-valid lowercase `bearer` —
     fixed via `split_once(' ')` + `eq_ignore_ascii_case`.
  5. `AUTH_EXEMPT_PATH` was a literal duplicated in reasoning across 3 sites
     with no shared source of truth — addressed as part of the `from_fn`
     refactor (single const, single check site).
  6. Missing `WWW-Authenticate` header on 401 (RFC 7235 §3.1) — added to
     `ApiError::Unauthorized`'s response.
  7. `security(...)` OpenAPI annotations could drift from actual enforcement
     with no test catching it — added
     `all_protected_paths_have_security_requirement_except_health` to
     `tests/openapi.rs`.
  8. Shared `/tmp/teeline-api.pid` across `api:serve`/`test:e2e:api`/
     `test:e2e:auth` is a latent concurrency risk — documented via comment
     (not fixed structurally; tasks never run concurrently today).
  9. **Cleanup**: refactored hand-rolled `AuthLayer`/`AuthService` (~85 lines
     of `Layer`/`Service` impls) to `axum::middleware::from_fn` + `route_layer`
     (~30 lines) — re-verified full behavior parity via 22 tests + live curl
     matrix (health/no-token/bearer/lowercase-bearer/unmatched-path/
     WWW-Authenticate) after the rewrite.
  10. **Cleanup**: the `bash -c 'nohup ... & echo $! > pid; disown'` launch
      line was copy-pasted verbatim 3x in Taskfile.yml — extracted to a shared
      internal task `api:_launch` (vars threaded through as env vars, same
      injection-safe pattern as fix #3), called from `api:serve`,
      `test:e2e:api`, `test:e2e:auth`.
  11. `extract_token` allocated a new `String` per request — changed to
      return `Option<&str>` borrowing from the request's `HeaderMap`.
- Re-verified after all 11 fixes: `cargo test -p teeline-api` (22 tests in
  api_tests.rs + others), `cargo clippy -p teeline -p teeline-cli -p
  teeline-api -- -D warnings` (matches CI's exact invocation) clean,
  `cargo fmt --check` clean, `task test:e2e:api` (7/7 files) and
  `task test:e2e:auth` (1/1 file, 10 requests) both green. Committed as
  6338a3f and pushed to update PR #310.

## Completion Evidence

- `cargo test -p teeline-api`: 24 unit + 18 api_tests (incl. 7 new auth tests) +
  1/2/5/6/6/4 across the other 6 route-specific test files — all passing, 0 failed
- `cargo clippy --workspace -- -D warnings`: clean
- `cargo fmt -- --check`: clean
- Manual smoke test (auth disabled): `/api/v1/solvers` with no header → 200
- Manual smoke test (auth enabled): health open (200 no header), solvers without
  token (401), wrong bearer (401), correct bearer (200), correct X-Api-Key (200),
  401 body exactly `{"error":"Unauthorized"}`
- `openapi.json`: both `bearer_token`/`api_key` schemes present;
  `/api/v1/solvers` requires either; `/api/v1/health` has no security requirement
- `task test:e2e:api`: 7/7 files, 14 requests, all succeeded (unmodified, confirms
  back-compat)
- `task test:e2e:auth`: 1/1 file, 10 requests, all succeeded
- `task check`: exit 0 (core `teeline`/`teeline-cli` unaffected)
- Post-review-fix round: 11/11 findings from `/code-review max` resolved, all
  re-verified with tests + clippy + fmt + live e2e/curl (see Progress Log
  round 2); commit 6338a3f pushed to PR #310

Ready for merge, pending final PR approval.

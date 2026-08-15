# Auth endpoints — manual verification runbook

How to check, from a terminal, that the WebAuthn auth service and the API are working together.
Use this when something "doesn't authenticate" or after re-deploying / re-setting secrets.

Reference layout:

- **Auth service** — Cloudflare Pages Functions on `tspsolver.com/api/auth/*` (project `teeline-web`)
- **Auth database** — Cloudflare D1 `teeline-auth` (schema: users / credentials / api_keys / challenges)
- **API** — `teeline-api` on Fly.io, `https://api.tspsolver.com`; verifies keys by calling
  `POST {AUTH_SERVICE_URL}/api/auth/keys/verify` with the shared `AUTH_SERVICE_SECRET` header

All curl examples assume a terminal where `wrangler` and `fly` are logged in.

---

## 1. Quick health checks (no secrets needed)

```bash
# Signed-out user → the session endpoint must reject
curl -s -o /dev/null -w "%{http_code}\n" https://tspsolver.com/api/auth/me          # expect 401

# Registration ceremony start → must mint a challenge (needs the Origin header — CSRF check)
curl -s -X POST -H "Origin: https://tspsolver.com" -H "Content-Type: application/json" \
  -d '{}' https://tspsolver.com/api/auth/register/begin                              # expect 201 + JSON "options.challenge"

# Login ceremony start
curl -s -X POST -H "Origin: https://tspsolver.com" -H "Content-Type: application/json" \
  -d '{}' https://tspsolver.com/api/auth/login/begin                                 # expect 201

# Internal verify endpoint: wrong shared secret → rejected
curl -s -X POST -H "X-Auth-Secret: wrong" -H "Content-Type: application/json" \
  -d '{"secret":"ak_x"}' https://tspsolver.com/api/auth/keys/verify                  # expect 401 JSON {"error":"Unauthorized"}

# The interactive UI is deployed?
curl -s https://tspsolver.com/api-key/ | grep -o astro-island                       # expect output
```

## 2. End-to-end: seed a known key in D1, call the API with it

This exercises the whole loop — **API → auth service verify → D1 lookup → authorize** — and also
proves the shared secrets match between Fly and Pages (a mismatch shows up as 401, see §3).

```bash
# 1. Pick a secret and compute its SHA-256 (hex, lowercase — what the service stores)
SECRET="ak_manualcheck00000000000000000000000000"
HASH=$(printf '%s' "$SECRET" | sha256sum | cut -d' ' -f1)

# 2. Insert a throwaway user + key into the auth D1 (from teeline-web/)
cd teeline-web
npx wrangler d1 execute teeline-auth --remote --command "
  INSERT INTO users (id, display_name, created_at) VALUES ('manual-check','runbook',1752000000000) ON CONFLICT DO NOTHING;
  INSERT INTO api_keys (id, user_id, name, secret_hash, created_at, revoked) VALUES ('key_manual1','manual-check','runbook','$HASH',1752000000000,0) ON CONFLICT DO NOTHING;"

# 3. Call the API
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $SECRET" https://api.tspsolver.com/api/v1/solvers   # expect 200
curl -s -o /dev/null -w "%{http_code}\n" -H "X-Api-Key: $SECRET"            https://api.tspsolver.com/api/v1/solvers   # expect 200
curl -s -o /dev/null -w "%{http_code}\n" https://api.tspsolver.com/api/v1/solvers                                        # expect 401 (no key)
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer ak_wrongkey00000000000000000000000000000" https://api.tspsolver.com/api/v1/solvers  # expect 401

# 4. Clean up the throwaway rows
npx wrangler d1 execute teeline-auth --remote --command "
  DELETE FROM api_keys WHERE id='key_manual1';
  DELETE FROM users WHERE id='manual-check';"
```

## 3. Interpreting failures

| Symptom | Meaning | Fix |
| --- | --- | --- |
| `me` → **200** (homepage HTML) | `/api/auth/*` isn't hitting a function — stale Pages deployment (deploy-web workflow failing or not run) | Fix the deploy; see §5; the last deploy must be ≥ the latest merged phase |
| `verify` → **405** | Static host answered (function not registered on that path) | Same as above — functions missing from the deployment |
| `verify` → **401** | Function ran, but `X-Auth-Secret` is wrong/missing → **Fly's `AUTH_SERVICE_SECRET` ≠ Pages'** (or one isn't set) | Re-set the same value on both: `fly secrets set … --app teeline-api` and `wrangler pages secret put … --project-name teeline-web` |
| `verify` → **404** | Function ran with a **matching** secret, but the key isn't in D1 (or fails the `ak_` shape check) | Re-check §2 seeding; confirm the row with a SELECT |
| API **401** with a known-good key | API not in service mode (`TEELINE_AUTH_MODE` unset/breakglass), or `AUTH_SERVICE_URL` wrong | `fly secrets list --app teeline-api` + §4 |
| deploy fails `The given account is not valid… [7403]` | GitHub `CLOUDFLARE_ACCOUNT_ID` ≠ account owning `teeline-auth`, or token lacks D1 permission | See `teeline-web/README.md` → Required GitHub Secrets |
| curl without `Origin` header → **403** | Correct — the CSRF Origin check requires the browser's origin | Add `-H "Origin: https://tspsolver.com"` |

## 4. Confirm the API is running the current code (Fly)

```bash
fly releases --app teeline-api     # latest release should be recent (a teeline-api deploy)
fly logs --app teeline-api         # the startup line tells the auth mode:
#   "API auth: verifying keys via auth service"   → service mode (correct for prod)
#   "API auth: break-glass key only"              → TEELINE_AUTH_MODE unset/breakglass
#   "API auth disabled"                           → no credentials at all
#   "API auth enabled static_key=true clerk=true" → OLD binary (pre-Phase-5); deploy again

# With a valid key in hand (§2), a successful request logs:
#   "request authorized via service API key"      (only exists in the new middleware)
```

## 5. Confirm the Pages deployment is current

```bash
cd teeline-web
npx wrangler pages deployment list --project-name teeline-web
# the latest Production row's commit must include the auth phases (functions + /api-key/ UI)

# Watch live requests (optionally while re-running §1/§2):
npx wrangler pages deployment tail --project-name teeline-web <deployment-id>
```

## 6. Gotchas

- **`AUTH_SERVICE_SECRET` must be byte-identical on Fly and Pages** — it is the only thing
  protecting the internal verify endpoint.
- **`SESSION_SECRET`** (Pages only) signs the session cookie; rotating it signs everyone out.
- Curl tests to the auth service need the **`Origin` header** (CSRF), and the session cookie is
  `__Host-`/SameSite=Strict — a full sign-in only works from a real browser.
- Test keys inserted via §2 are **real, working keys** — always clean them up.

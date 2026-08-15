# teeline-api

REST API for solving TSP problems programmatically — submit cities as inline JSON coordinates or a TSPLIB file, pick a solver, get an optimised tour back. Built with Axum, documented with OpenAPI 3.1 + Scalar.

Live at **[api.tspsolver.com](https://api.tspsolver.com)** (Fly.io) — interactive docs at [api.tspsolver.com/docs](https://api.tspsolver.com/docs).

## Authentication

Every endpoint except `GET /api/v1/health` requires a bearer token (`Authorization: Bearer <token>` or `X-Api-Key: <token>`). Get a personal API key via self-serve sign-in — see the [step-by-step guide](https://tspsolver.com/api-key/).

## Local Development

```bash
cargo build -p teeline-api
task api:serve        # runs in the background, writes PID to /tmp/teeline-api.pid
task api:stop
```

Relevant environment variables:

| Variable | Purpose | Default |
| --- | --- | --- |
| `PORT` | Listen port | `8080` |
| `API_KEY` | Static break-glass bearer token. Unset or empty disables it. | unset |
| `TEELINE_AUTH_MODE` | `breakglass` (static key only), `service` (verify via the auth service) or `disabled`. Unset: inferred from the credentials below. | inferred |
| `AUTH_SERVICE_URL` | Base URL of the WebAuthn auth service (e.g. `https://tspsolver.com`). Used in `service` mode. | unset |
| `AUTH_SERVICE_SECRET` | Shared secret for the auth service's internal verify endpoint. Used in `service` mode. | unset |
| `RATE_LIMIT_RPM` | Requests per minute per client. `0` disables rate limiting. | `100` |

## Testing

```bash
cargo test -p teeline-api      # unit + integration tests
task test:e2e:api              # hurl e2e suite, auth disabled
task test:e2e:auth             # hurl e2e suite, static API_KEY enabled
```

## Deployment

Pushes to `master` that touch `teeline-api/**` automatically trigger the [`deploy-api`](../.github/workflows/deploy-api.yml) GitHub Actions workflow, which builds a Docker image (`Dockerfile`) and deploys to [Fly.io](https://fly.io).

### Required GitHub Secrets

| Secret | How to obtain |
| --- | --- |
| `FLY_API_TOKEN` | `fly tokens create deploy --app teeline-api` |

### Required Fly.io Secrets

Set via `fly secrets set <NAME>=<value> --app teeline-api`:

| Secret | Purpose |
| --- | --- |
| `API_KEY` | Static break-glass bearer token |
| `TEELINE_AUTH_MODE` | `service` in production (see Authentication above) |
| `AUTH_SERVICE_URL` | Base URL of the WebAuthn auth service |
| `AUTH_SERVICE_SECRET` | Shared secret for the auth service's verify endpoint |

> **`AUTH_SERVICE_SECRET` must match the Cloudflare Pages project's `AUTH_SERVICE_SECRET`** —
> the auth service only answers the internal verify endpoint to callers presenting this header.
> Set the Pages side with `echo "<value>" | npx wrangler pages secret put AUTH_SERVICE_SECRET
> --project-name teeline-web` (see `teeline-web/README.md` → *Cloudflare Pages Secrets*).

### End-to-end setup checklist (fresh environment)

```bash
# 1. Cloudflare: create the auth D1 + schema + secrets (from teeline-web/)
npx wrangler d1 create teeline-auth                                   # → database_id into wrangler.toml
npx wrangler d1 migrations apply teeline-auth --remote
echo "<AUTH_SERVICE_SECRET>" | npx wrangler pages secret put AUTH_SERVICE_SECRET --project-name teeline-web
echo "<SESSION_SECRET>"      | npx wrangler pages secret put SESSION_SECRET      --project-name teeline-web

# 2. Fly: point the API at the auth service (same AUTH_SERVICE_SECRET)
fly secrets set TEELINE_AUTH_MODE=service AUTH_SERVICE_URL=https://tspsolver.com \
  AUTH_SERVICE_SECRET=<same-value> --app teeline-api
fly secrets unset CLERK_SECRET_KEY --app teeline-api                    # remove the old IdP

# 3. Deploy both (CI on master, or manually):
#    task web:deploy:release   # wrangler pages deploy (migrations run first in deploy-web.yml)
#    task api:release          # flyctl deploy
```

Local-dev / CI keep `TEELINE_AUTH_MODE=breakglass` + `API_KEY` only (no auth-service round-trip).

### Manual Deploy

```bash
task api:release   # flyctl deploy --config fly.toml --remote-only
```

## Architecture

```text
teeline-api/
├── src/
│   ├── main.rs       # entrypoint — reads env, wires up auth/rate-limiting/routes
│   ├── lib.rs        # build_api_router() / build_router() — route wiring
│   ├── middleware.rs # require_auth (static key + service verifier), metrics
│   ├── auth.rs       # ApiKeyVerifier trait + ServiceVerifier
│   ├── openapi.rs    # utoipa OpenAPI spec + Scalar docs UI
│   ├── error.rs      # ApiError → HTTP response mapping
│   ├── metrics.rs    # Prometheus/OpenMetrics state
│   ├── models/       # request/response DTOs
│   ├── routes/       # handler for each endpoint
│   └── services/     # solver + registry service traits
└── tests/
    ├── *.rs          # mock-based integration tests, one file per route
    └── hurl/         # e2e suite run against a real running binary
```

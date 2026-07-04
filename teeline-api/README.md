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
| `CLERK_SECRET_KEY` | Enables self-serve per-user API keys via Clerk. Unset disables it. | unset |
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
| `CLERK_SECRET_KEY` | Enables self-serve API keys (see Authentication above) |

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
│   ├── middleware.rs # require_auth (static key + Clerk verifier), metrics
│   ├── clerk.rs      # ApiKeyVerifier trait + ClerkVerifier
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

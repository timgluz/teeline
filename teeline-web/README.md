# teeline-web

Browser-based TSP solver — upload a `.tsp` file, configure a solver, and download the optimised tour in `.tour`, `.csv`, `.json`, or `.svg` format. Powered by a Rust/WASM solver core running in a Web Worker.

Live at **[tspsolver.com](https://tspsolver.com)** (Cloudflare Pages).

## Local Development

```bash
npm ci
npm run dev      # Astro dev server at http://localhost:4321
npm test         # Vitest unit tests
npm run test:e2e # Playwright e2e tests
npm run build    # astro check + TypeScript check + production build → dist/
```

## Deployment

Pushes to `master` that touch `teeline-web/**` automatically trigger the [`deploy-web`](../.github/workflows/deploy-web.yml) GitHub Actions workflow, which builds and deploys to Cloudflare Pages.

### Required GitHub Secrets

Add these in **GitHub → repo Settings → Secrets and variables → Actions**:

| Secret | How to obtain |
| --- | --- |
| `CLOUDFLARE_API_TOKEN` | [CF Dashboard](https://dash.cloudflare.com) → My Profile → API Tokens → **Create Token** — needs **both** *Cloudflare Pages: Edit* **and** *D1: Edit* permissions (deploy runs `wrangler d1 migrations apply` before the Pages deploy) |
| `CLOUDFLARE_ACCOUNT_ID` | [CF Dashboard](https://dash.cloudflare.com) → select your account → the Account ID appears in the right sidebar. **Must be the account that owns the `teeline-auth` D1 database** (check with `npx wrangler d1 list`; a mismatch fails the deploy with CF error 7403). |

`SENTRY_AUTH_TOKEN` is optional — the build succeeds without it; Sentry source map uploads are simply skipped.

### Cloudflare Pages Secrets — auth service

The WebAuthn auth service (Pages Functions under `functions/api/auth/`) needs two secrets on the
**teeline-web** Pages project (set from a terminal with wrangler logged in):

```bash
# Generate values
openssl rand -hex 32   # → AUTH_SERVICE_SECRET
openssl rand -hex 32   # → SESSION_SECRET

# Set them on the Pages project (production; preview inherits unless overridden
# in the dashboard → Settings → Environment Variables)
echo "<AUTH_SERVICE_SECRET>" | npx wrangler pages secret put AUTH_SERVICE_SECRET --project-name teeline-web
echo "<SESSION_SECRET>"      | npx wrangler pages secret put SESSION_SECRET      --project-name teeline-web
npx wrangler pages secret list --project-name teeline-web   # verify
```

- **`AUTH_SERVICE_SECRET` is the shared secret with the API** — the exact same value must be set on
  Fly.io (`fly secrets set AUTH_SERVICE_SECRET=… --app teeline-api`), or `POST /api/auth/keys/verify`
  rejects the API's calls. See `teeline-api/README.md` → *Required Fly.io Secrets*.
- **`SESSION_SECRET`** signs the session cookie (`__Host-teeline-session`); rotate it and everyone
  gets signed out (acceptable — it's just a re-login).
- Local dev: put both in `teeline-web/.dev.vars` (plus `ALLOWED_ORIGINS=http://localhost:8788`).

### Cloudflare D1 — auth database

```bash
# First-time only: create the database and apply the schema (id lands in wrangler.toml)
npx wrangler d1 create teeline-auth          # paste database_id into wrangler.toml
npx wrangler d1 migrations apply teeline-auth --remote

# Deploys run migrations automatically (deploy-web.yml), so this is only for a fresh setup.
```

### Manual Deploy

```bash
npm run deploy   # builds then runs wrangler pages deploy dist/
```

## Cloudflare MCP

The [Cloudflare API MCP server](https://developers.cloudflare.com/agents/model-context-protocol/mcp-servers-for-cloudflare/) exposes 2500+ Cloudflare API endpoints via `search()` and `execute()` tools, letting you manage Pages deployments, custom domains, and environment variables directly from Claude Code.

It is already configured in `.mcp.json` (project root):

```json
"cloudflare-api": {
  "type": "http",
  "url": "https://mcp.cloudflare.com/mcp"
}
```

On first use, Claude Code will prompt you to authenticate with your Cloudflare account.

## Architecture

Built with [Astro 7](https://astro.build) — file-based routing, content collections for the algorithm docs + blog, server-rendered by default (no client-side nav re-render needed). The main solver app and its Web Worker bridge are plain TypeScript, unaffected by the framework choice.

```text
teeline-web/
├── astro.config.mjs
├── src/
│   ├── pages/            # file-based routes (index, tsp, webmcp, api-key,
│   │                      #  algorithms/[id], algorithms/[id]/explainer, blog/*)
│   ├── layouts/           # BaseLayout, DocsLayout
│   ├── components/        # Topbar, Sidebar, AlgorithmCards, FeatureCards
│   ├── content.config.ts  # `docs` (sourced from ../docs/algorithms/*.md) +
│   │                       #  `blog` (src/content/blog/*.md) collections
│   ├── explainers/         # 10 interactive Preact islands (client:visible)
│   ├── main.ts             # app bootstrap + state
│   ├── upload.ts           # Step 01 — drag-drop file upload
│   ├── solver-form.ts      # Step 02 — solver config + checklist
│   ├── canvas.ts           # SVG tour rendering
│   ├── results.ts          # Step 03 — results table + run history
│   ├── download.ts         # .tour / .csv / .json / .svg export
│   └── worker.ts           # WASM Web Worker bridge
├── functions/
│   └── tunnel.js        # Cloudflare Pages Function — Sentry event proxy
└── public/
    └── examples/        # bundled berlin52, burma14, ulysses22 datasets
```

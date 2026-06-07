# teeline-web

Browser-based TSP solver — upload a `.tsp` file, configure a solver, and download the optimised tour in `.tour`, `.csv`, `.json`, or `.svg` format. Powered by a Rust/WASM solver core running in a Web Worker.

Live at **[tspsolver.com](https://tspsolver.com)** (Cloudflare Pages).

## Local Development

```bash
npm ci
npm run dev      # Vite dev server at http://localhost:5173
npm test         # Vitest unit tests
npm run build    # TypeScript check + production build → dist/
```

## Deployment

Pushes to `master` that touch `teeline-web/**` automatically trigger the [`deploy-web`](../.github/workflows/deploy-web.yml) GitHub Actions workflow, which builds and deploys to Cloudflare Pages.

### Required GitHub Secrets

Add these in **GitHub → repo Settings → Secrets and variables → Actions**:

| Secret | How to obtain |
|--------|--------------|
| `CLOUDFLARE_API_TOKEN` | [CF Dashboard](https://dash.cloudflare.com) → My Profile → API Tokens → **Create Token** → use the **Edit Cloudflare Pages** template (or custom token with *Cloudflare Pages: Edit* permission) |
| `CLOUDFLARE_ACCOUNT_ID` | [CF Dashboard](https://dash.cloudflare.com) → select your account → the Account ID appears in the right sidebar |

`SENTRY_AUTH_TOKEN` is optional — the build succeeds without it; Sentry source map uploads are simply skipped.

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

```
teeline-web/
├── src/
│   ├── main.ts          # app bootstrap + state
│   ├── upload.ts        # Step 01 — drag-drop file upload
│   ├── solver-form.ts   # Step 02 — solver config + checklist
│   ├── canvas.ts        # SVG tour rendering
│   ├── results.ts       # Step 03 — results table + run history
│   ├── download.ts      # .tour / .csv / .json / .svg export
│   └── worker.ts        # WASM Web Worker bridge
├── functions/
│   └── tunnel.js        # Cloudflare Pages Function — Sentry event proxy
└── public/
    └── examples/        # bundled berlin52, burma14, ulysses22 datasets
```

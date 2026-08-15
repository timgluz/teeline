// Shared Env shape for the WebAuthn auth Pages Functions.
// Bindings come from wrangler.toml; secrets are set on the Pages project
// (AUTH_SERVICE_SECRET, SESSION_SECRET) and ALLOWED_ORIGINS is a dev override.
export interface Env {
  DB: D1Database
  AUTH_SERVICE_SECRET?: string
  SESSION_SECRET?: string
  ALLOWED_ORIGINS?: string
}

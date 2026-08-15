// Phase 0 scaffold: proves the Pages Functions pipeline AND that
// @simplewebauthn/server (pinned 13.3.2) bundles + runs on the Workers
// runtime (ESM build; WebCrypto). Imports both the option generator and the
// verifier so the full import graph is exercised by the bundler.
// Replaced by real WebAuthn routes in Phase 2.
import { generateRegistrationOptions, verifyRegistrationResponse } from '@simplewebauthn/server'

export const onRequestGet: PagesFunction<Env> = async (context) => {
  try {
    const opts = await generateRegistrationOptions({
      rpName: 'Teeline',
      rpID: 'tspsolver.com',
      userName: 'probe',
      userID: new Uint8Array(16),
    })
    return Response.json({
      ok: true,
      service: 'webauthn-auth',
      hasDbBinding: !!context.env.DB,
      challengeLength: opts.challenge.length,
      verifierLoaded: typeof verifyRegistrationResponse === 'function',
    })
  } catch (err) {
    return Response.json({ ok: false, error: String(err) }, { status: 500 })
  }
}

// Env shape for this worker — D1 binding + secrets (moved to a shared .d.ts
// in Phase 1; inline here so the scaffold type-checks standalone).
interface Env {
  DB?: D1Database & { name?: string }
  AUTH_SERVICE_SECRET?: string
  SESSION_SECRET?: string
  ALLOWED_ORIGINS?: string
}

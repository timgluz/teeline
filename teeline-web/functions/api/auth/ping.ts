// Phase 0 scaffold: proves the Pages Functions pipeline AND that
// @simplewebauthn/server (pinned 13.3.2) bundles + runs on the Workers
// runtime (ESM build; WebCrypto). Imports both the option generator and the
// verifier so the full import graph is exercised by the bundler.
// Replaced by real WebAuthn routes in Phase 2.
import { generateRegistrationOptions, verifyRegistrationResponse } from '@simplewebauthn/server'
import type { Env } from '../../lib/env'

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

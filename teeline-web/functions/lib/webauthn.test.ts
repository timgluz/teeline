// WebAuthn origin/rpID policy tests.
import { describe, expect, it } from 'vitest'
import { clientOrigin, isAllowedOrigin, isClientOriginAllowed, rpIdFor, serverOrigin } from './webauthn'
import type { Env } from './env'

const baseEnv = { DB: null as never } as Env

describe('rpIdFor', () => {
  it('maps tspsolver.com and subdomains to the registrable domain', () => {
    expect(rpIdFor('tspsolver.com')).toBe('tspsolver.com')
    expect(rpIdFor('www.tspsolver.com')).toBe('tspsolver.com')
    expect(rpIdFor('accounts.tspsolver.com')).toBe('tspsolver.com')
  })

  it('uses the hostname for local dev', () => {
    expect(rpIdFor('localhost')).toBe('localhost')
    expect(rpIdFor('127.0.0.1')).toBe('127.0.0.1')
  })
})

describe('isAllowedOrigin', () => {
  it('allows production origin', () => {
    expect(isAllowedOrigin('https://tspsolver.com', baseEnv)).toBe(true)
  })

  it('rejects local dev origins unless explicitly listed (no blanket allowance)', () => {
    expect(isAllowedOrigin('http://localhost:8788', baseEnv)).toBe(false)
    expect(isAllowedOrigin('http://127.0.0.1:8788', baseEnv)).toBe(false)
    const env = { ...baseEnv, ALLOWED_ORIGINS: 'http://localhost:8788' }
    expect(isAllowedOrigin('http://localhost:8788', env)).toBe(true)
    expect(isAllowedOrigin('http://127.0.0.1:8788', env)).toBe(false)
  })

  it('rejects unknown remote origins', () => {
    expect(isAllowedOrigin('https://evil.com', baseEnv)).toBe(false)
    expect(isAllowedOrigin('https://tspsolver.com.evil.com', baseEnv)).toBe(false)
    expect(isAllowedOrigin('http://localhost.evil.com', baseEnv)).toBe(false)
  })

  it('honours ALLOWED_ORIGINS extras (preview deploys etc.)', () => {
    const env = { ...baseEnv, ALLOWED_ORIGINS: 'https://preview.teeline-web.pages.dev' }
    expect(isAllowedOrigin('https://preview.teeline-web.pages.dev', env)).toBe(true)
    expect(isAllowedOrigin('https://other.pages.dev', env)).toBe(false)
  })
})

describe('serverOrigin', () => {
  it('derives the destination origin from the request URL (WebAuthn expectedOrigin)', () => {
    expect(serverOrigin(new Request('https://tspsolver.com/api/auth/me'))).toBe('https://tspsolver.com')
    expect(serverOrigin(new Request('http://localhost:8788/api/auth/login/begin'))).toBe('http://localhost:8788')
  })
})

describe('clientOrigin / CSRF helper', () => {
  it('prefers the Origin header', () => {
    const req = new Request('https://tspsolver.com/api/auth/logout', {
      method: 'POST',
      headers: { Origin: 'https://tspsolver.com', Referer: 'https://evil.com/x' },
    })
    expect(clientOrigin(req)).toBe('https://tspsolver.com')
    expect(isClientOriginAllowed(req, baseEnv)).toBe(true)
  })

  it('falls back to Referer', () => {
    const req = new Request('https://tspsolver.com/api/auth/logout', {
      method: 'POST',
      headers: { Referer: 'https://tspsolver.com/some/page' },
    })
    expect(clientOrigin(req)).toBe('https://tspsolver.com')
  })

  it('returns null with neither header, and the origin check rejects', () => {
    const req = new Request('https://tspsolver.com/api/auth/logout', { method: 'POST' })
    expect(clientOrigin(req)).toBeNull()
    expect(isClientOriginAllowed(req, baseEnv)).toBe(false)
  })

  it('rejects a spoofed cross-site Origin', () => {
    const req = new Request('https://tspsolver.com/api/auth/logout', {
      method: 'POST',
      headers: { Origin: 'https://evil.com' },
    })
    expect(isClientOriginAllowed(req, baseEnv)).toBe(false)
  })
})

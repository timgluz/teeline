// WebAuthn origin/rpID policy tests.
import { describe, expect, it } from 'vitest'
import { isAllowedOrigin, requestOrigin, rpIdFor } from './webauthn'
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

describe('requestOrigin', () => {
  it('derives the origin from the request URL', () => {
    expect(requestOrigin(new Request('https://tspsolver.com/api/auth/me'))).toBe('https://tspsolver.com')
    expect(requestOrigin(new Request('http://localhost:8788/api/auth/login/begin'))).toBe('http://localhost:8788')
  })
})

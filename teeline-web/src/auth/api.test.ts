// apiFetch tests — mocked fetch, node env.
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError, apiFetch } from './api'

afterEach(() => {
  vi.unstubAllGlobals()
})

function stubFetch(status: number, body: unknown) {
  const res = new Response(JSON.stringify(body), { status })
  vi.stubGlobal('fetch', vi.fn(async () => res))
}

describe('apiFetch', () => {
  it('returns parsed JSON on success', async () => {
    stubFetch(200, { ok: true })
    await expect(apiFetch<{ ok: boolean }>('/api/auth/me')).resolves.toEqual({ ok: true })
  })

  it('sends same-origin credentials and JSON content type', async () => {
    stubFetch(200, {})
    await apiFetch('/api/auth/keys', { method: 'POST', body: '{}' })
    const [url, init] = vi.mocked(fetch).mock.calls[0]
    expect(url).toBe('/api/auth/keys')
    expect((init as RequestInit).credentials).toBe('same-origin')
    expect(((init as RequestInit).headers as Record<string, string>)['Content-Type']).toBe('application/json')
  })

  it('throws ApiError with the server error message', async () => {
    stubFetch(401, { error: 'Unauthorized' })
    const err = await apiFetch('/api/auth/me').catch((e: unknown) => e)
    expect(err).toBeInstanceOf(ApiError)
    expect((err as ApiError).status).toBe(401)
    expect((err as ApiError).message).toBe('Unauthorized')
  })

  it('falls back to a generic message when the body has no error field', async () => {
    stubFetch(500, {})
    const err = await apiFetch('/api/auth/me').catch((e: unknown) => e)
    expect((err as ApiError).message).toContain('500')
  })
})

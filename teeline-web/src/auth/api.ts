// Thin same-origin fetch wrapper for the auth service (/api/auth/*).
// Cookies (the __Host-teeline-session) are sent same-origin; the browser
// attaches the Origin header automatically, which the server checks for CSRF.

export class ApiError extends Error {
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

export async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  // Destructure `credentials` out so callers cannot override the same-origin
  // policy enforced below (see header comment).
  const { headers, credentials: _enforced, ...rest } = init
  const res = await fetch(path, {
    credentials: 'same-origin',
    headers: {
      'Content-Type': 'application/json',
      ...Object.fromEntries(new Headers(headers)),
    },
    ...rest,
  })

  if (!res.ok) {
    let message = `Request failed (${res.status})`
    try {
      const body = (await res.json()) as { error?: string }
      if (body.error) message = body.error
    } catch {
      /* keep the generic message */
    }
    throw new ApiError(res.status, message)
  }

  if (res.status === 204) return undefined as T
  return (await res.json()) as T
}

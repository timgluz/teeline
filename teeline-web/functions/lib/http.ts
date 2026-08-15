// Small HTTP helpers shared by the auth Pages Functions.

export function json(data: unknown, status = 200, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      'Content-Type': 'application/json',
      // Auth responses (challenges, session cookies, keys) must never be
      // cached by browsers or intermediaries.
      'Cache-Control': 'no-store',
      ...headers,
    },
  })
}

export function badRequest(message: string): Response {
  return json({ error: message }, 400)
}

export function forbidden(): Response {
  return json({ error: 'Forbidden' }, 403)
}

export function unauthorized(): Response {
  return json({ error: 'Unauthorized' }, 401)
}

export function serverError(message: string): Response {
  return json({ error: message }, 500)
}

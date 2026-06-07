// Cloudflare Pages Function — proxies Sentry envelope events to bypass ad blockers.
// Validates DSN host + project ID before forwarding to prevent abuse.

const SENTRY_HOST = 'o4511523471228928.ingest.de.sentry.io'
const ALLOWED_PROJECT_IDS = ['4511523482107984']

export async function onRequest({ request }) {
  if (request.method !== 'POST') {
    return new Response('Method Not Allowed', { status: 405 })
  }
  try {
    const body = await request.text()
    const header = JSON.parse(body.split('\n')[0])
    const dsn = new URL(header.dsn)
    const projectId = dsn.pathname.replace('/', '')
    if (dsn.hostname !== SENTRY_HOST || !ALLOWED_PROJECT_IDS.includes(projectId)) {
      return new Response('Forbidden', { status: 403 })
    }
    return fetch(`https://${SENTRY_HOST}/api/${projectId}/envelope/`, {
      method: 'POST',
      body,
    })
  } catch {
    return new Response('Bad Request', { status: 400 })
  }
}

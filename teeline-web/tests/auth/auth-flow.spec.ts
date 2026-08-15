import { expect, test, type Page } from '@playwright/test'

// The full passkey → API-key loop against the local Pages stack
// (wrangler pages dev + local D1). WebAuthn is simulated with a CDP virtual
// authenticator (CTAP2, resident keys, user verification).
async function setupVirtualAuthenticator(page: Page): Promise<void> {
  const client = await page.context().newCDPSession(page)
  await client.send('WebAuthn.enable')
  await client.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  })
}

test('register → mint → show-once → refresh wipes → login → revoke → verify', async ({ page, request }) => {
  await setupVirtualAuthenticator(page)

  // ---- anonymous state ----
  await page.goto('/api-key/')
  await expect(page.getByRole('button', { name: 'Create a passkey' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Sign in with passkey' })).toBeVisible()

  // ---- register (open registration: first passkey becomes the account) ----
  await page.getByRole('button', { name: 'Create a passkey' }).click()
  await expect(page.getByRole('heading', { name: 'API keys' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Generate API key' })).toBeVisible()

  // ---- mint an API key ----
  await page.getByRole('button', { name: 'Generate API key' }).click()
  const secretBox = page.getByTestId('fresh-secret')
  await expect(secretBox).toBeVisible()
  const secret = (await secretBox.textContent())?.trim() ?? ''
  expect(secret).toMatch(/^ak_[A-Za-z0-9_-]{43}$/)

  // ---- show-once: a refresh destroys the secret (not persisted) ----
  await page.reload()
  await expect(page.getByTestId('fresh-secret')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Generate API key' })).toBeVisible()

  // the key is listed (metadata only)
  await expect(page.locator('tbody tr')).toHaveCount(1)

  // ---- the minted key verifies against the auth service (full loop) ----
  const verify = await request.post('/api/auth/keys/verify', {
    headers: { 'X-Auth-Secret': 'e2e-shared-secret' },
    data: { secret },
  })
  expect(verify.status()).toBe(200)
  const body = (await verify.json()) as { subject: string; revoked: boolean; expired: boolean }
  expect(body).toMatchObject({ revoked: false, expired: false })
  expect(typeof body.subject).toBe('string')

  // a wrong secret still fails closed
  const badVerify = await request.post('/api/auth/keys/verify', {
    headers: { 'X-Auth-Secret': 'e2e-shared-secret' },
    data: { secret: 'ak_notminted000000000000000000000000000' },
  })
  expect(badVerify.status()).toBe(404)

  // ---- sign out, sign back in with the same passkey ----
  await page.getByRole('button', { name: 'Sign out' }).click()
  await expect(page.getByRole('button', { name: 'Sign in with passkey' })).toBeVisible()
  await page.getByRole('button', { name: 'Sign in with passkey' }).click()
  await expect(page.getByRole('button', { name: 'Generate API key' })).toBeVisible()
  await expect(page.locator('tbody tr')).toHaveCount(1)

  // ---- revoke (destructive → confirm dialog) ----
  page.once('dialog', (dialog) => dialog.accept())
  await page.getByRole('button', { name: /Revoke key/ }).click()
  // soft revoke: the key stays listed, marked as revoked, no revoke button
  await expect(page.getByText('revoked')).toBeVisible()
  await expect(page.getByRole('button', { name: /Revoke key/ })).toHaveCount(0)

  // the revoked key no longer verifies
  const revokedVerify = await request.post('/api/auth/keys/verify', {
    headers: { 'X-Auth-Secret': 'e2e-shared-secret' },
    data: { secret },
  })
  expect(revokedVerify.status()).toBe(404)
})

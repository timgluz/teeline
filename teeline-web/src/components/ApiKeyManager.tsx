import { useEffect, useState } from 'preact/hooks'
import { ApiError, apiFetch } from '../auth/api'
import { loginWithPasskey, logout, me, registerPasskey, type User } from '../auth/webauthn'

interface ApiKeyMeta {
  id: string
  name: string | null
  createdAt: number
  lastUsedAt: number | null
  revoked: boolean
}

interface FreshSecret {
  id: string
  secret: string
  name: string | null
}

type View = 'loading' | 'anonymous' | 'signedin'

function messageOf(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

function formatDate(ms: number | null): string {
  if (!ms) return '—'
  return new Date(ms).toLocaleString()
}

/**
 * The interactive core of /api-key/: passkey sign-in (open registration),
 * API-key minting with the show-once secret, and key management.
 *
 * Show-once: the freshly minted secret lives only in component state — it is
 * deliberately NOT persisted to sessionStorage/localStorage, so refreshing
 * the page destroys it (the reminder tells the user to save it first).
 */
export default function ApiKeyManager() {
  const [view, setView] = useState<View>('loading')
  const [user, setUser] = useState<User | null>(null)
  const [keys, setKeys] = useState<ApiKeyMeta[]>([])
  const [fresh, setFresh] = useState<FreshSecret | null>(null)
  const [copied, setCopied] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const refreshKeys = async () => {
    const { keys } = await apiFetch<{ keys: ApiKeyMeta[] }>('/api/auth/keys')
    setKeys(keys)
  }

  const load = async () => {
    try {
      const u = await me()
      if (!u) {
        setView('anonymous')
        return
      }
      setUser(u)
      await refreshKeys()
      setView('signedin')
    } catch (err) {
      setError(messageOf(err))
      setView('anonymous')
    }
  }

  useEffect(() => {
    void load()
  }, [])

  const run = async (fn: () => Promise<void>) => {
    setBusy(true)
    setError(null)
    try {
      await fn()
    } catch (err) {
      setError(messageOf(err))
    } finally {
      setBusy(false)
    }
  }

  const onRegister = () =>
    run(async () => {
      const u = await registerPasskey()
      setUser(u)
      await refreshKeys()
      setView('signedin')
    })

  const onLogin = () =>
    run(async () => {
      const u = await loginWithPasskey()
      setUser(u)
      await refreshKeys()
      setView('signedin')
    })

  const onGenerate = () =>
    run(async () => {
      const key = await apiFetch<FreshSecret>('/api/auth/keys', { method: 'POST', body: '{}' })
      setFresh(key)
      setCopied(false)
      await refreshKeys()
    })

  const onRevoke = (id: string) =>
    run(async () => {
      await apiFetch(`/api/auth/keys/${id}`, { method: 'DELETE' })
      await refreshKeys()
    })

  const onLogout = () =>
    run(async () => {
      await logout()
      setFresh(null)
      setUser(null)
      setKeys([])
      setView('anonymous')
    })

  const onCopySecret = async () => {
    try {
      await navigator.clipboard.writeText(fresh?.secret ?? '')
      setCopied(true)
    } catch {
      setError('Could not copy automatically — select and copy the key manually.')
    }
  }

  if (view === 'loading') {
    return <p class="text-text-dim">Loading…</p>
  }

  const btnBase = 'inline-flex items-center justify-center rounded-md px-5 py-2 font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50'

  return (
    <section class="my-6 max-w-2xl">
      {error && (
        <p class="mb-4 rounded-md border border-negative/40 bg-negative/10 px-4 py-2 text-sm text-negative" role="alert">
          {error}
        </p>
      )}

      {view === 'anonymous' && (
        <div class="rounded-lg border border-border bg-surface p-6">
          <h2 class="mb-1 text-xl font-semibold text-text">Sign in with a passkey</h2>
          <p class="mb-4 text-sm text-text-dim">
            No account or password needed. First time here? <strong class="text-text">Create a passkey</strong> —
            your browser or password manager stores it. Later visits are a single tap.
          </p>
          <div class="flex flex-wrap gap-3">
            <button class={`${btnBase} bg-accent text-white hover:bg-accent/90`} onClick={onRegister} disabled={busy}>
              {busy ? 'Working…' : 'Create a passkey'}
            </button>
            <button
              class={`${btnBase} border border-border bg-transparent text-text hover:border-accent hover:text-accent`}
              onClick={onLogin}
              disabled={busy}
            >
              Sign in with passkey
            </button>
          </div>
        </div>
      )}

      {view === 'signedin' && user && (
        <div class="rounded-lg border border-border bg-surface p-6">
          <div class="mb-5 flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 class="text-xl font-semibold text-text">API keys</h2>
              <p class="text-sm text-text-dim">
                Signed in as <span class="font-mono text-accent">{user.displayName ?? user.id.slice(0, 8)}</span>
              </p>
            </div>
            <button class={`${btnBase} border border-border px-3 py-1 text-sm text-text-dim hover:text-text`} onClick={onLogout} disabled={busy}>
              Sign out
            </button>
          </div>

          {fresh && (
            <div class="mb-5 rounded-md border border-positive/50 bg-positive/10 p-4" role="status">
              <p class="mb-2 text-sm font-semibold text-positive">Your new API key — shown once</p>
              <code class="block break-all rounded bg-surface-2 px-3 py-2 font-mono text-sm text-text" data-testid="fresh-secret">
                {fresh.secret}
              </code>
              <p class="mt-2 text-sm text-text">
                <strong>Copy it into your password manager now.</strong> It won't be shown again — after you
                refresh or leave this page it's gone for good.
              </p>
              <div class="mt-3 flex flex-wrap gap-2">
                <button class={`${btnBase} bg-accent px-4 py-1 text-sm text-white hover:bg-accent/90`} onClick={onCopySecret}>
                  {copied ? 'Copied!' : 'Copy to clipboard'}
                </button>
                <button class={`${btnBase} border border-border px-4 py-1 text-sm text-text hover:border-accent hover:text-accent`} onClick={() => setFresh(null)}>
                  I've saved it
                </button>
              </div>
            </div>
          )}

          <button class={`${btnBase} mb-5 bg-accent text-white hover:bg-accent/90`} onClick={onGenerate} disabled={busy}>
            {busy ? 'Working…' : 'Generate API key'}
          </button>

          {keys.length === 0 ? (
            <p class="text-sm text-text-dim">No keys yet — generate one above.</p>
          ) : (
            <table class="w-full border-collapse text-sm">
              <thead>
                <tr class="text-left text-text-dim">
                  <th class="border-b border-border py-2 pr-3 font-semibold">Name</th>
                  <th class="border-b border-border py-2 pr-3 font-semibold">Created</th>
                  <th class="border-b border-border py-2 pr-3 font-semibold">Last used</th>
                  <th class="border-b border-border py-2 text-right font-semibold">Revoke</th>
                </tr>
              </thead>
              <tbody>
                {keys.map((k) => (
                  <tr key={k.id} class={k.revoked ? 'opacity-50' : ''}>
                    <td class="border-b border-border py-2 pr-3 font-mono text-text">{k.name ?? k.id.slice(0, 10)}</td>
                    <td class="border-b border-border py-2 pr-3 text-text-dim">{formatDate(k.createdAt)}</td>
                    <td class="border-b border-border py-2 pr-3 text-text-dim">{formatDate(k.lastUsedAt)}</td>
                    <td class="border-b border-border py-2 text-right">
                      {k.revoked ? (
                        <span class="text-xs text-text-dim">revoked</span>
                      ) : (
                        <button
                          class="rounded px-2 py-1 text-xs text-negative hover:bg-negative/10 disabled:opacity-50"
                          onClick={() => onRevoke(k.id)}
                          disabled={busy}
                        >
                          Revoke
                        </button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </section>
  )
}

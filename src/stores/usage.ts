import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { ProviderId, ProviderQuota, UsageSnapshot } from '../types/usage'
import { emptyQuota } from '../types/usage'
import { invoke, listen, isTauri } from '../lib/tauri'
import { mockSnapshot } from '../lib/mock'

/** How often the cached backend snapshot is re-read as an event fallback. */
const SNAPSHOT_POLL_MS = 15_000

export const useUsageStore = defineStore('usage', () => {
  const providers = ref<Record<ProviderId, ProviderQuota>>({
    codex: emptyQuota('codex'),
    claude: emptyQuota('claude'),
  })
  const refreshing = ref(false)
  const lastError = ref<string | null>(null)
  /** Ticks every second so countdowns re-render without re-fetching provider data. */
  const now = ref(Math.floor(Date.now() / 1000))

  let unlistenUsage: (() => void) | null = null
  let tickTimer: ReturnType<typeof setInterval> | null = null
  let pollTimer: ReturnType<typeof setInterval> | null = null

  const codex = computed(() => providers.value.codex)
  const claude = computed(() => providers.value.claude)
  const hasAnyAvailableProvider = computed(() =>
    Object.values(providers.value).some((p) => p.status === 'ready' || p.status === 'stale'),
  )

  function applySnapshot(snapshot: UsageSnapshot) {
    providers.value = { codex: snapshot.codex, claude: snapshot.claude }
  }

  function applyProviderUpdate(payload: ProviderQuota) {
    providers.value = { ...providers.value, [payload.provider]: payload }
  }

  async function initialize() {
    if (!tickTimer) {
      tickTimer = setInterval(() => {
        now.value = Math.floor(Date.now() / 1000)
      }, 1000)
    }

    if (!isTauri) {
      applySnapshot(mockSnapshot())
      return
    }

    try {
      unlistenUsage ??= await listen<UsageSnapshot>('usage://updated', (event) => {
        applySnapshot(event.payload)
      })
    } catch (err) {
      lastError.value = String(err)
    }

    // The backend pushes `usage://updated` on its own schedule, but the widget
    // must not depend on that one channel: poll the cached snapshot too, so a
    // missed event can never leave the popup stuck on "waiting for data".
    if (!pollTimer) {
      pollTimer = setInterval(() => void refreshSnapshot(), SNAPSHOT_POLL_MS)
    }

    await refreshSnapshot()
  }

  async function refreshSnapshot() {
    try {
      applySnapshot(await invoke<UsageSnapshot>('get_usage_snapshot'))
      lastError.value = null
    } catch (err) {
      lastError.value = String(err)
    }
  }

  async function refreshAll() {
    if (refreshing.value) return
    refreshing.value = true
    try {
      if (!isTauri) {
        applySnapshot(mockSnapshot())
        return
      }
      applySnapshot(await invoke<UsageSnapshot>('refresh_all'))
      lastError.value = null
    } catch (err) {
      lastError.value = String(err)
    } finally {
      refreshing.value = false
    }
  }

  async function refreshProvider(id: ProviderId) {
    try {
      applyProviderUpdate(await invoke<ProviderQuota>('refresh_provider', { provider: id }))
    } catch (err) {
      lastError.value = String(err)
    }
  }

  function dispose() {
    unlistenUsage?.()
    unlistenUsage = null
    if (tickTimer) clearInterval(tickTimer)
    tickTimer = null
    if (pollTimer) clearInterval(pollTimer)
    pollTimer = null
  }

  return {
    providers,
    refreshing,
    lastError,
    now,
    codex,
    claude,
    hasAnyAvailableProvider,
    initialize,
    refreshAll,
    refreshProvider,
    refreshSnapshot,
    applySnapshot,
    applyProviderUpdate,
    dispose,
  }
})

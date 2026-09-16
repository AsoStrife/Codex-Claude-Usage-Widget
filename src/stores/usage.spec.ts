import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useUsageStore } from './usage'
import type { ProviderQuota, UsageSnapshot } from '../types/usage'

function quota(overrides: Partial<ProviderQuota> = {}): ProviderQuota {
  return {
    provider: 'codex',
    status: 'ready',
    fiveHour: { usedPercentage: 28, remainingPercentage: 72, resetsAt: 500, durationMinutes: 300 },
    weekly: null,
    plan: 'plus',
    updatedAt: 100,
    source: 'test',
    error: null,
    ...overrides,
  }
}

const snapshot = (): UsageSnapshot => ({
  codex: quota(),
  claude: quota({ provider: 'claude', status: 'waiting-for-data', fiveHour: null, updatedAt: null }),
})

describe('useUsageStore', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('starts with both providers waiting for data', () => {
    const store = useUsageStore()
    expect(store.codex.status).toBe('waiting-for-data')
    expect(store.claude.status).toBe('waiting-for-data')
    expect(store.hasAnyAvailableProvider).toBe(false)
  })

  it('applies a full snapshot to both providers', () => {
    const store = useUsageStore()
    store.applySnapshot(snapshot())
    expect(store.codex.fiveHour?.remainingPercentage).toBe(72)
    expect(store.claude.status).toBe('waiting-for-data')
    expect(store.hasAnyAvailableProvider).toBe(true)
  })

  it('applies a single provider update without touching the other', () => {
    const store = useUsageStore()
    store.applySnapshot(snapshot())
    store.applyProviderUpdate(quota({ provider: 'claude', status: 'ready', plan: 'max' }))

    expect(store.claude.plan).toBe('max')
    expect(store.claude.status).toBe('ready')
    expect(store.codex.plan).toBe('plus')
  })

  it('counts a stale provider as available so its last values keep showing', () => {
    const store = useUsageStore()
    store.applyProviderUpdate(quota({ status: 'stale' }))
    expect(store.hasAnyAvailableProvider).toBe(true)
  })

  it('does not count unavailable providers', () => {
    const store = useUsageStore()
    store.applyProviderUpdate(quota({ status: 'not-installed', fiveHour: null }))
    store.applyProviderUpdate(quota({ provider: 'claude', status: 'error', fiveHour: null }))
    expect(store.hasAnyAvailableProvider).toBe(false)
  })

  it('records the failure instead of throwing when the backend is unavailable', async () => {
    const store = useUsageStore()
    await store.refreshSnapshot()
    expect(store.lastError).toContain('get_usage_snapshot')
  })

  it('clears the refreshing flag after a refresh outside the desktop shell', async () => {
    const store = useUsageStore()
    await store.refreshAll()
    expect(store.refreshing).toBe(false)
    // The browser fallback renders mock data rather than an empty widget.
    expect(store.codex.status).toBe('ready')
  })
})

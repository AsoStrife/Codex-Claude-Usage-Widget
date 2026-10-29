import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useSettingsStore } from './settings'
import { defaultSettings } from '../types/usage'
import { invoke } from '../lib/tauri'

vi.mock('../lib/tauri', () => ({ isTauri: true, invoke: vi.fn() }))
beforeEach(() => { setActivePinia(createPinia()); vi.clearAllMocks() })

describe('provider monitoring preferences', () => {
  it('saves a provider choice independently of its status-line connection and restores it on load', async () => {
    const store = useSettingsStore()
    store.settings.claudeIntegrationEnabled = true
    const saved = { ...defaultSettings(), claudeIntegrationEnabled: true, monitoring: { codex: true, claude: false, antigravity: true } }
    vi.mocked(invoke).mockResolvedValue(saved)
    await store.setProviderMonitoring('claude', false)
    expect(invoke).toHaveBeenCalledExactlyOnceWith('set_provider_monitoring', { provider: 'claude', enabled: false })
    expect(store.settings.monitoring.claude).toBe(false)
    expect(store.settings.claudeIntegrationEnabled).toBe(true)
    setActivePinia(createPinia())
    const reloaded = useSettingsStore()
    await reloaded.initialize()
    expect(reloaded.settings.monitoring).toEqual(saved.monitoring)
  })

  it('preserves the previous selection and reports a failed save', async () => {
    const store = useSettingsStore()
    vi.mocked(invoke).mockRejectedValue(new Error('settings are read-only'))
    await store.setProviderMonitoring('codex', false)
    expect(store.settings.monitoring.codex).toBe(true)
    expect(store.lastError).toContain('settings are read-only')
    expect(store.busy).toBe(false)
  })
})

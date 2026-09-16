import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { AppSettings, Diagnostics } from '../types/usage'
import { invoke, isTauri } from '../lib/tauri'

const DEFAULTS: AppSettings = {
  startWithWindows: false,
  hideOnBlur: true,
  refreshIntervalSeconds: 60,
  claudeIntegrationEnabled: false,
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({ ...DEFAULTS })
  const diagnostics = ref<Diagnostics | null>(null)
  const busy = ref(false)
  const lastError = ref<string | null>(null)

  async function initialize() {
    if (!isTauri) return
    try {
      settings.value = await invoke<AppSettings>('get_settings')
    } catch (err) {
      lastError.value = String(err)
    }
  }

  async function loadDiagnostics() {
    if (!isTauri) return
    try {
      diagnostics.value = await invoke<Diagnostics>('get_diagnostics')
    } catch (err) {
      lastError.value = String(err)
    }
  }

  async function setStartWithWindows(enabled: boolean) {
    busy.value = true
    try {
      settings.value = await invoke<AppSettings>('set_start_with_windows', { enabled })
      lastError.value = null
    } catch (err) {
      lastError.value = String(err)
    } finally {
      busy.value = false
    }
  }

  async function setClaudeIntegration(enabled: boolean) {
    busy.value = true
    try {
      const cmd = enabled ? 'enable_claude_integration' : 'disable_claude_integration'
      settings.value = await invoke<AppSettings>(cmd)
      lastError.value = null
      await loadDiagnostics()
    } catch (err) {
      lastError.value = String(err)
    } finally {
      busy.value = false
    }
  }

  return { settings, diagnostics, busy, lastError, initialize, loadDiagnostics, setStartWithWindows, setClaudeIntegration }
})

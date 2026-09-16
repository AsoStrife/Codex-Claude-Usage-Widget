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

/** Diagnostics are expensive (two subprocess spawns); reuse them this long. */
const DIAGNOSTICS_TTL_MS = 10_000

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({ ...DEFAULTS })
  const diagnostics = ref<Diagnostics | null>(null)
  const busy = ref(false)
  const lastError = ref<string | null>(null)
  let diagnosticsLoadedAt = 0
  let diagnosticsInFlight: Promise<Diagnostics> | null = null

  async function initialize() {
    if (!isTauri) return
    try {
      settings.value = await invoke<AppSettings>('get_settings')
    } catch (err) {
      lastError.value = String(err)
    }
  }

  async function loadDiagnostics(force = false) {
    if (!isTauri) return
    // Diagnostics shell out to `codex --version` / `claude --version`; without
    // this guard every settings toggle spawned another pair of processes.
    const age = Date.now() - diagnosticsLoadedAt
    if (!force && diagnostics.value !== null && age < DIAGNOSTICS_TTL_MS) return
    if (diagnosticsInFlight) {
      await diagnosticsInFlight.catch(() => undefined)
      return
    }
    try {
      diagnosticsInFlight = invoke<Diagnostics>('get_diagnostics')
      diagnostics.value = await diagnosticsInFlight
      diagnosticsLoadedAt = Date.now()
    } catch (err) {
      lastError.value = String(err)
    } finally {
      diagnosticsInFlight = null
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
      await loadDiagnostics(true)
    } catch (err) {
      lastError.value = String(err)
    } finally {
      busy.value = false
    }
  }

  return { settings, diagnostics, busy, lastError, initialize, loadDiagnostics, setStartWithWindows, setClaudeIntegration }
})

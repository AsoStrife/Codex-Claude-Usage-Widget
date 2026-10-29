export type ProviderId = 'codex' | 'claude' | 'antigravity'

export type ProviderStatus =
  | 'disabled'
  | 'ready'
  | 'not-installed'
  | 'not-authenticated'
  | 'waiting-for-data'
  | 'stale'
  | 'error'

export interface QuotaWindow {
  usedPercentage: number
  remainingPercentage: number
  /** Unix epoch seconds, or null when the provider did not report a reset. */
  resetsAt: number | null
  durationMinutes: number | null
}

export interface QuotaBucket {
  id: string
  label: string
  remainingPercentage: number | null
  remainingAmount: number | null
  resetsAt: number | null
  disabled: boolean
}

export interface ProviderQuota {
  provider: ProviderId
  status: ProviderStatus
  fiveHour: QuotaWindow | null
  weekly: QuotaWindow | null
  buckets?: QuotaBucket[]
  plan?: string | null
  /** Unix epoch seconds of the last successful provider update. */
  updatedAt: number | null
  source: string
  error?: string | null
}

export interface UsageSnapshot {
  codex: ProviderQuota
  claude: ProviderQuota
  antigravity: ProviderQuota
}

export interface AppSettings {
  monitoring: Record<ProviderId, boolean>
  startWithWindows: boolean
  hideOnBlur: boolean
  refreshIntervalSeconds: number
  claudeIntegrationEnabled: boolean
  /** Claude's status-line command from before the bridge was installed. */
  claudePreviousStatusLine?: string | null
  antigravityIntegrationEnabled: boolean
  antigravityPreviousStatusLine?: Record<string, unknown> | null
}

export interface Diagnostics {
  appVersion: string
  codexPath: string | null
  codexVersion: string | null
  codexStatus: ProviderStatus
  codexAppServerStatus: string
  claudePath: string | null
  claudeVersion: string | null
  claudeIntegrationEnabled: boolean
  claudeCacheUpdatedAt: number | null
  claudeSettingsPath: string | null
  antigravityPath: string | null
  antigravityVersion: string | null
  antigravityIntegrationEnabled: boolean
  antigravityCacheUpdatedAt: number | null
  antigravitySettingsPath: string | null
}

export const PROVIDER_LABEL: Record<ProviderId, string> = {
  codex: 'Codex',
  claude: 'Claude',
  antigravity: 'Antigravity',
}

export function defaultSettings(): AppSettings {
  return {
    monitoring: { codex: true, claude: true, antigravity: true },
    startWithWindows: false,
    hideOnBlur: true,
    refreshIntervalSeconds: 60,
    claudeIntegrationEnabled: false,
    claudePreviousStatusLine: null,
    antigravityIntegrationEnabled: false,
    antigravityPreviousStatusLine: null,
  }
}

export function emptyQuota(provider: ProviderId): ProviderQuota {
  return {
    provider,
    status: 'waiting-for-data',
    fiveHour: null,
    weekly: null,
    plan: null,
    updatedAt: null,
    source: 'pending',
    error: null,
  }
}

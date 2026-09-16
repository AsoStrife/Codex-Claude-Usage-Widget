export type ProviderId = 'codex' | 'claude'

export type ProviderStatus =
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

export interface ProviderQuota {
  provider: ProviderId
  status: ProviderStatus
  fiveHour: QuotaWindow | null
  weekly: QuotaWindow | null
  plan?: string | null
  /** Unix epoch seconds of the last successful provider update. */
  updatedAt: number | null
  source: string
  error?: string | null
}

export interface UsageSnapshot {
  codex: ProviderQuota
  claude: ProviderQuota
}

export interface AppSettings {
  startWithWindows: boolean
  hideOnBlur: boolean
  refreshIntervalSeconds: number
  claudeIntegrationEnabled: boolean
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
}

export const PROVIDER_LABEL: Record<ProviderId, string> = {
  codex: 'Codex',
  claude: 'Claude',
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

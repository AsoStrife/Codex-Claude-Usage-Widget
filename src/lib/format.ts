import type { ProviderStatus, QuotaWindow } from '../types/usage'

export function clampPercentage(value: number): number {
  if (!Number.isFinite(value)) return 0
  return Math.min(100, Math.max(0, value))
}

export function remainingOf(win: QuotaWindow): number {
  return clampPercentage(win.remainingPercentage)
}

/** Rounds for display: keeps one decimal below 10% so near-empty quotas stay readable. */
export function formatPercentage(value: number): string {
  const v = clampPercentage(value)
  if (v > 0 && v < 10) return `${Math.round(v * 10) / 10}%`
  return `${Math.round(v)}%`
}

/**
 * Human countdown to an absolute epoch-seconds timestamp.
 * Returns null when there is no timestamp, and 'now' once it has passed.
 */
export function formatCountdown(resetsAt: number | null, nowSeconds: number): string | null {
  if (resetsAt === null) return null
  const delta = resetsAt - nowSeconds
  if (delta <= 0) return 'now'
  const days = Math.floor(delta / 86400)
  const hours = Math.floor((delta % 86400) / 3600)
  const minutes = Math.floor((delta % 3600) / 60)
  const seconds = Math.floor(delta % 60)
  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${minutes}m`
  if (minutes > 0) return `${minutes}m`
  return `${seconds}s`
}

/** Absolute clock label, e.g. "Thu 09:30" — or "09:30" when it resets today. */
export function formatResetClock(resetsAt: number | null, nowSeconds: number): string | null {
  if (resetsAt === null) return null
  const date = new Date(resetsAt * 1000)
  const now = new Date(nowSeconds * 1000)
  const time = date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', hour12: false })
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  if (sameDay) return time
  const weekday = date.toLocaleDateString(undefined, { weekday: 'short' })
  return `${weekday} ${time}`
}

export function formatRelativeAge(updatedAt: number | null, nowSeconds: number): string {
  if (updatedAt === null) return 'never updated'
  const delta = Math.max(0, nowSeconds - updatedAt)
  if (delta < 10) return 'just now'
  if (delta < 60) return `${Math.floor(delta)}s ago`
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`
  return `${Math.floor(delta / 86400)}d ago`
}

export const STATUS_LABEL: Record<ProviderStatus, string> = {
  ready: 'Ready',
  'not-installed': 'Not installed',
  'not-authenticated': 'Not signed in',
  'waiting-for-data': 'Waiting for data',
  stale: 'Stale',
  error: 'Error',
}

export interface StatusHint {
  title: string
  detail: string | null
}

export function statusHint(status: ProviderStatus, provider: string, error?: string | null): StatusHint {
  switch (status) {
    case 'not-installed':
      return { title: `${provider} is not installed`, detail: 'Install the CLI and it will be detected automatically.' }
    case 'not-authenticated':
      return { title: `${provider} is not signed in`, detail: 'Sign in with your subscription account to report usage.' }
    case 'waiting-for-data':
      // Claude's provider reports *why* it is waiting (integration off vs.
      // simply no status-line payload yet); prefer that over the generic hint.
      return provider === 'Claude'
        ? {
            title: 'Waiting for Claude usage data',
            detail: error ?? 'Use Claude Code once and its usage will appear here.',
          }
        : { title: 'Waiting for usage data', detail: null }
    case 'error':
      return { title: 'Could not read usage', detail: error ?? null }
    default:
      return { title: STATUS_LABEL[status], detail: error ?? null }
  }
}

/** Progress bar tone driven by how much quota is LEFT. */
export function toneForRemaining(remaining: number): 'good' | 'warn' | 'low' {
  const v = clampPercentage(remaining)
  if (v <= 10) return 'low'
  if (v <= 30) return 'warn'
  return 'good'
}

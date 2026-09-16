import { describe, expect, it } from 'vitest'
import {
  clampPercentage,
  formatCountdown,
  formatPercentage,
  formatRelativeAge,
  formatResetClock,
  statusHint,
  toneForRemaining,
} from './format'

describe('clampPercentage', () => {
  it('keeps values inside 0-100', () => {
    expect(clampPercentage(42)).toBe(42)
    expect(clampPercentage(140)).toBe(100)
    expect(clampPercentage(-3)).toBe(0)
  })

  it('treats non-finite input as zero', () => {
    expect(clampPercentage(Number.NaN)).toBe(0)
    expect(clampPercentage(Number.POSITIVE_INFINITY)).toBe(0)
  })
})

describe('formatPercentage', () => {
  it('rounds to whole percent above 10', () => {
    expect(formatPercentage(72.4)).toBe('72%')
    expect(formatPercentage(41.6)).toBe('42%')
  })

  it('keeps one decimal in the last 10% so near-empty quotas stay readable', () => {
    expect(formatPercentage(3.24)).toBe('3.2%')
    expect(formatPercentage(0.7)).toBe('0.7%')
  })

  it('shows an exhausted quota as a plain zero', () => {
    expect(formatPercentage(0)).toBe('0%')
  })
})

describe('formatCountdown', () => {
  const now = 1_000_000

  it('formats hours and minutes', () => {
    expect(formatCountdown(now + 2 * 3600 + 14 * 60, now)).toBe('2h 14m')
  })

  it('drops hours below one hour', () => {
    expect(formatCountdown(now + 48 * 60, now)).toBe('48m')
    expect(formatCountdown(now + 12 * 60, now)).toBe('12m')
  })

  it('falls back to seconds in the last minute', () => {
    expect(formatCountdown(now + 30, now)).toBe('30s')
  })

  it('uses days for weekly windows', () => {
    expect(formatCountdown(now + 3 * 86400 + 5 * 3600, now)).toBe('3d 5h')
  })

  it('reports a passed reset as now, never as a negative time', () => {
    expect(formatCountdown(now - 60, now)).toBe('now')
    expect(formatCountdown(now, now)).toBe('now')
  })

  it('has nothing to show without a timestamp', () => {
    expect(formatCountdown(null, now)).toBeNull()
  })
})

describe('formatResetClock', () => {
  it('omits the weekday when the reset is today', () => {
    const now = new Date(2026, 8, 15, 9, 0, 0).getTime() / 1000
    const later = new Date(2026, 8, 15, 18, 30, 0).getTime() / 1000
    expect(formatResetClock(later, now)).toBe('18:30')
  })

  it('includes the weekday for a later day', () => {
    const now = new Date(2026, 8, 15, 9, 0, 0).getTime() / 1000
    const thursday = new Date(2026, 8, 17, 9, 30, 0).getTime() / 1000
    expect(formatResetClock(thursday, now)).toMatch(/09:30$/)
    expect(formatResetClock(thursday, now)!.length).toBeGreaterThan(5)
  })

  it('has nothing to show without a timestamp', () => {
    expect(formatResetClock(null, 0)).toBeNull()
  })
})

describe('formatRelativeAge', () => {
  const now = 1_000_000

  it('describes recent updates', () => {
    expect(formatRelativeAge(now, now)).toBe('just now')
    expect(formatRelativeAge(now - 30, now)).toBe('30s ago')
    expect(formatRelativeAge(now - 180, now)).toBe('3m ago')
    expect(formatRelativeAge(now - 7200, now)).toBe('2h ago')
  })

  it('distinguishes never-updated from just-updated', () => {
    expect(formatRelativeAge(null, now)).toBe('never updated')
  })
})

describe('toneForRemaining', () => {
  it('escalates as the remaining quota shrinks', () => {
    expect(toneForRemaining(80)).toBe('good')
    expect(toneForRemaining(25)).toBe('warn')
    expect(toneForRemaining(4)).toBe('low')
  })
})

describe('statusHint', () => {
  it('tells the user what to do about a missing provider', () => {
    expect(statusHint('not-installed', 'Codex').title).toContain('not installed')
    expect(statusHint('not-authenticated', 'Codex').title).toContain('not signed in')
  })

  it('gives Claude-specific guidance while waiting for the first payload', () => {
    const hint = statusHint('waiting-for-data', 'Claude')
    expect(hint.title).toBe('Waiting for Claude usage data')
    expect(hint.detail).toContain('Use Claude Code once')
  })

  it('surfaces the backend error message', () => {
    expect(statusHint('error', 'Codex', 'app-server closed').detail).toBe('app-server closed')
  })
})

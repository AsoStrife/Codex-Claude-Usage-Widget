import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import ProviderCard from './ProviderCard.vue'
import type { ProviderQuota, ProviderStatus, QuotaWindow } from '../types/usage'

const NOW = 1_000_000

function quotaWindow(used: number, resetsIn: number | null, durationMinutes: number): QuotaWindow {
  return {
    usedPercentage: used,
    remainingPercentage: 100 - used,
    resetsAt: resetsIn === null ? null : NOW + resetsIn,
    durationMinutes,
  }
}

function provider(overrides: Partial<ProviderQuota> = {}): ProviderQuota {
  return {
    provider: 'codex',
    status: 'ready',
    fiveHour: quotaWindow(28, 8040, 300),
    weekly: quotaWindow(59, 210000, 10080),
    plan: 'plus',
    updatedAt: NOW - 8,
    source: 'test',
    error: null,
    ...overrides,
  }
}

const render = (quota: ProviderQuota) => mount(ProviderCard, { props: { quota, now: NOW } })

describe('ProviderCard', () => {
  it('renders both windows as remaining percentages, not used ones', () => {
    const text = render(provider()).text()
    expect(text).toContain('72% left')
    expect(text).toContain('41% left')
  })

  it('shows a live countdown for the 5-hour window', () => {
    expect(render(provider()).text()).toContain('resets in 2h 14m')
  })

  it('shows an absolute time for the weekly window', () => {
    // 210000s ahead: a date plus a countdown, not just a countdown.
    expect(render(provider()).text()).toMatch(/in 2d 10h/)
  })

  it('names the provider and its plan', () => {
    const text = render(provider()).text()
    expect(text).toContain('Codex')
    expect(text).toContain('plus')
  })

  it('marks a missing window as not reported rather than as full', () => {
    const text = render(provider({ fiveHour: null })).text()
    expect(text).toContain('Not reported')
    expect(text).not.toContain('100% left')
  })

  it('omits the countdown when the provider reported no reset time', () => {
    const text = render(provider({ fiveHour: quotaWindow(10, null, 300), weekly: null })).text()
    expect(text).toContain('90% left')
    expect(text).not.toContain('resets in')
  })

  it.each<[ProviderStatus, string]>([
    ['not-installed', 'not installed'],
    ['not-authenticated', 'not signed in'],
    ['waiting-for-data', 'Waiting for usage data'],
  ])('replaces the quota rows with guidance when the status is %s', (status, expected) => {
    const text = render(provider({ status, fiveHour: null, weekly: null })).text()
    expect(text).toContain(expected)
    expect(text).not.toContain('% left')
  })

  it('reports the backend error message', () => {
    const text = render(provider({ status: 'error', fiveHour: null, weekly: null, error: 'app-server closed' })).text()
    expect(text).toContain('app-server closed')
  })

  it('keeps showing the last known values when the data is stale', () => {
    const text = render(provider({ status: 'stale', updatedAt: NOW - 4000 })).text()
    expect(text).toContain('72% left')
    expect(text).toContain('stale')
  })

  it('shows how fresh a ready reading is', () => {
    expect(render(provider()).text()).toContain('just now')
    expect(render(provider({ updatedAt: NOW - 180 })).text()).toContain('3m ago')
  })

  it('uses the Claude palette and label for the Claude provider', () => {
    const card = render(provider({ provider: 'claude', plan: 'max' }))
    // The label is uppercased in CSS, so the DOM text keeps its original casing.
    expect(card.text()).toContain('Claude')
    expect(card.html()).toContain('bg-claude')
  })
})

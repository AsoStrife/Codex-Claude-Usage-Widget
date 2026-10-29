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

  it('shows the countdown and exact local time for the 5-hour window', () => {
    const reset = quotaWindow(28, 8040, 300)
    const time = new Date(reset.resetsAt! * 1000)
    const clock = `${String(time.getHours()).padStart(2, '0')}:${String(time.getMinutes()).padStart(2, '0')}`
    expect(render(provider()).text()).toContain(`resets in 2h 14m - ${clock}`)
  })

  it('shows the countdown and complete local date and time for the weekly window', () => {
    const reset = quotaWindow(59, 210000, 10080)
    const date = new Date(reset.resetsAt! * 1000)
    const dateTime = `${String(date.getDate()).padStart(2, '0')}/${String(date.getMonth() + 1).padStart(2, '0')}/${date.getFullYear()} ${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`
    expect(render(provider()).text()).toContain(`resets in 2d 10h - ${dateTime}`)
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

  it('renders Antigravity bucket names, percentages, balances and disabled states', () => {
    const card = render(provider({ provider: 'antigravity', plan: 'Pro', fiveHour: null, weekly: null, buckets: [
      { id: 'gemini-weekly', label: 'Gemini Weekly', remainingPercentage: 63, remainingAmount: null, resetsAt: NOW + 86400, disabled: false },
      { id: 'credits', label: 'Credits', remainingPercentage: null, remainingAmount: 42, resetsAt: null, disabled: false },
      { id: 'disabled', label: 'Unavailable model', remainingPercentage: null, remainingAmount: null, resetsAt: null, disabled: true },
    ] }))
    expect(card.text()).toContain('Antigravity')
    expect(card.html()).toContain('bg-antigravity')
    expect(card.text()).toContain('Gemini Weekly')
    expect(card.text()).toContain('63% left')
    expect(card.text()).toContain('42 left')
    expect(card.text()).toContain('Disabled')
    expect(card.text()).not.toContain('100% left')
    expect(card.text()).not.toContain('Not reported')
    expect(card.text()).toContain('resets in 1d')
  })

  it('hides expired Antigravity buckets and preserves a reported zero balance', () => {
    const card = render(provider({ provider: 'antigravity', fiveHour: null, weekly: null, buckets: [
      { id: 'expired', label: 'Expired', remainingPercentage: 90, remainingAmount: null, resetsAt: NOW, disabled: false },
      { id: 'live', label: 'Live', remainingPercentage: 0, remainingAmount: null, resetsAt: NOW + 60, disabled: false },
    ] }))
    expect(card.text()).not.toContain('Expired')
    expect(card.text()).toContain('0% left')
  })

  it('shows Antigravity setup instructions while waiting for its first payload', () => {
    const card = render(provider({ provider: 'antigravity', status: 'waiting-for-data', fiveHour: null, weekly: null,
      error: 'Enable Antigravity usage integration in settings.' }))
    expect(card.text()).toContain('Waiting for Antigravity usage data')
    expect(card.text()).toContain('Enable Antigravity usage integration in settings.')
  })
})

import { emptyQuota, type UsageSnapshot } from '../types/usage'

/** Fixture used only when the UI runs in a plain browser (`npm run dev`). */
export function mockSnapshot(): UsageSnapshot {
  const now = Math.floor(Date.now() / 1000)
  return {
    codex: {
      provider: 'codex',
      status: 'ready',
      fiveHour: { usedPercentage: 28, remainingPercentage: 72, resetsAt: now + 8040, durationMinutes: 300 },
      weekly: { usedPercentage: 59, remainingPercentage: 41, resetsAt: now + 210000, durationMinutes: 10080 },
      plan: 'plus',
      updatedAt: now - 8,
      source: 'mock',
      error: null,
    },
    claude: {
      provider: 'claude',
      status: 'ready',
      fiveHour: { usedPercentage: 46, remainingPercentage: 54, resetsAt: now + 3780, durationMinutes: 300 },
      weekly: { usedPercentage: 32, remainingPercentage: 68, resetsAt: now + 320000, durationMinutes: 10080 },
      plan: 'max',
      updatedAt: now - 8,
      source: 'mock',
      error: null,
    },
    antigravity: {
      ...emptyQuota('antigravity'),
      status: 'ready',
      buckets: [
        { id: 'gemini-5h', label: 'Gemini 5h', remainingPercentage: 78, remainingAmount: null, resetsAt: now + 7200, disabled: false },
        { id: 'gemini-weekly', label: 'Gemini Weekly', remainingPercentage: 93, remainingAmount: null, resetsAt: now + 320000, disabled: false },
      ],
      plan: 'Pro',
      updatedAt: now - 8,
      source: 'mock',
    },
  }
}

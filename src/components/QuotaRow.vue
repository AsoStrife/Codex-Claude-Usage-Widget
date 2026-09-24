<script setup lang="ts">
import { computed } from 'vue'
import type { QuotaWindow } from '../types/usage'
import { formatCountdown, formatPercentage, formatResetClock, formatResetDateTime, remainingOf } from '../lib/format'
import ProgressBar from './ProgressBar.vue'

const props = defineProps<{
  label: string
  window: QuotaWindow | null
  now: number
}>()

const remaining = computed(() => (props.window ? remainingOf(props.window) : null))
const countdown = computed(() => (props.window ? formatCountdown(props.window.resetsAt, props.now) : null))
const isLongWindow = computed(() => (props.window?.durationMinutes ?? 0) > 1440)
const resetTime = computed(() => {
  if (!props.window) return null
  return isLongWindow.value
    ? formatResetDateTime(props.window.resetsAt)
    : formatResetClock(props.window.resetsAt)
})

const resetLabel = computed(() => {
  if (!countdown.value) return null
  if (countdown.value === 'now') return 'resetting…'
  return `resets in ${countdown.value}${resetTime.value ? ` - ${resetTime.value}` : ''}`
})
</script>

<template>
  <div class="grid grid-cols-[3.25rem_1fr_auto] items-center gap-x-3 gap-y-1">
    <span class="text-[11px] font-medium tracking-wide text-ink-dim uppercase">{{ label }}</span>

    <template v-if="window && remaining !== null">
      <ProgressBar :remaining="remaining" />
      <span class="text-right text-[12px] font-semibold tabular-nums text-ink">
        {{ formatPercentage(remaining) }} <span class="font-normal text-ink-faint">left</span>
      </span>
      <span v-if="resetLabel" class="col-start-2 col-span-2 text-[11px] text-ink-faint tabular-nums">
        {{ resetLabel }}
      </span>
    </template>

    <template v-else>
      <span class="col-span-2 text-[12px] text-ink-faint italic">Not reported</span>
    </template>
  </div>
</template>

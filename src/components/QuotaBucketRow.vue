<script setup lang="ts">
import { computed } from 'vue'
import type { QuotaBucket } from '../types/usage'
import { formatCountdown, formatPercentage, formatResetDateTime } from '../lib/format'
import ProgressBar from './ProgressBar.vue'

const props = defineProps<{ bucket: QuotaBucket; now: number }>()
const resetLabel = computed(() => {
  if (props.bucket.disabled) return null
  const countdown = formatCountdown(props.bucket.resetsAt, props.now)
  if (!countdown) return null
  if (countdown === 'now') return 'resetting…'
  return `resets in ${countdown} - ${formatResetDateTime(props.bucket.resetsAt)}`
})
const amount = computed(() => props.bucket.remainingAmount?.toLocaleString(undefined, { maximumFractionDigits: 2 }))
</script>

<template>
  <div class="space-y-1">
    <div class="flex items-baseline gap-2">
      <span class="min-w-0 flex-1 truncate text-[11px] font-medium text-ink-dim" :title="bucket.id">
        {{ bucket.label }}
      </span>
      <span v-if="bucket.disabled" class="text-[11px] text-ink-faint">Disabled</span>
      <span v-else-if="bucket.remainingPercentage !== null" class="shrink-0 text-[12px] font-semibold tabular-nums text-ink">
        {{ formatPercentage(bucket.remainingPercentage) }} <span class="font-normal text-ink-faint">left</span>
      </span>
      <span v-else-if="bucket.remainingAmount !== null" class="shrink-0 text-[12px] font-semibold tabular-nums text-ink">
        {{ amount }} <span class="font-normal text-ink-faint">left</span>
      </span>
    </div>
    <ProgressBar v-if="!bucket.disabled && bucket.remainingPercentage !== null" :remaining="bucket.remainingPercentage" />
    <p v-if="resetLabel" class="text-[11px] text-ink-faint tabular-nums">{{ resetLabel }}</p>
  </div>
</template>

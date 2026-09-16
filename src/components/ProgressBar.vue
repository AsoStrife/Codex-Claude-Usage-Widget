<script setup lang="ts">
import { computed } from 'vue'
import { clampPercentage, toneForRemaining } from '../lib/format'

const props = defineProps<{
  /** Percentage of quota still available (0-100). */
  remaining: number
}>()

const width = computed(() => `${clampPercentage(props.remaining)}%`)
const tone = computed(() => toneForRemaining(props.remaining))

const TONE_CLASS: Record<'good' | 'warn' | 'low', string> = {
  good: 'bg-good',
  warn: 'bg-warn',
  low: 'bg-low',
}
</script>

<template>
  <div
    class="relative h-1.5 w-full overflow-hidden rounded-full bg-hairline/70"
    role="progressbar"
    :aria-valuenow="Math.round(clampPercentage(remaining))"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <div
      class="h-full rounded-full transition-[width] duration-500 ease-out"
      :class="TONE_CLASS[tone]"
      :style="{ width }"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { ProviderStatus } from '../types/usage'
import { statusHint } from '../lib/format'

const props = defineProps<{
  status: ProviderStatus
  providerLabel: string
  error?: string | null
}>()

const hint = computed(() => statusHint(props.status, props.providerLabel, props.error))
</script>

<template>
  <div class="rounded-lg border border-hairline/70 bg-panel/60 px-3 py-2.5">
    <p class="text-[12px] font-medium text-ink-dim">{{ hint.title }}</p>
    <p v-if="hint.detail" class="mt-0.5 text-[11px] leading-snug text-ink-faint">{{ hint.detail }}</p>
  </div>
</template>

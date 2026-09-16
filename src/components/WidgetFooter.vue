<script setup lang="ts">
import { computed } from 'vue'
import type { ProviderQuota } from '../types/usage'
import { formatRelativeAge } from '../lib/format'

const props = defineProps<{
  providers: ProviderQuota[]
  now: number
  error: string | null
}>()

/** The footer reports the freshest successful update across every provider. */
const newestUpdate = computed(() => {
  const stamps = props.providers.map((p) => p.updatedAt).filter((v): v is number => v !== null)
  return stamps.length ? Math.max(...stamps) : null
})

const anyStale = computed(() => props.providers.some((p) => p.status === 'stale'))
</script>

<template>
  <footer class="flex items-center gap-2 border-t border-hairline/70 px-4 py-2">
    <p v-if="error" class="truncate text-[11px] text-low" :title="error">{{ error }}</p>
    <p v-else class="text-[11px] tabular-nums" :class="anyStale ? 'text-warn' : 'text-ink-faint'">
      <template v-if="anyStale">Stale · last update {{ formatRelativeAge(newestUpdate, now) }}</template>
      <template v-else-if="newestUpdate">Updated {{ formatRelativeAge(newestUpdate, now) }}</template>
      <template v-else>No usage data yet</template>
    </p>
    <span class="ml-auto text-[10px] text-ink-faint/70">Esc to close</span>
  </footer>
</template>

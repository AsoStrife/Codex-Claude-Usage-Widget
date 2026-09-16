<script setup lang="ts">
import { computed } from 'vue'
import type { ProviderQuota } from '../types/usage'
import { PROVIDER_LABEL } from '../types/usage'
import { formatRelativeAge } from '../lib/format'
import QuotaRow from './QuotaRow.vue'
import ProviderStatus from './ProviderStatus.vue'

const props = defineProps<{
  quota: ProviderQuota
  now: number
}>()

const label = computed(() => PROVIDER_LABEL[props.quota.provider])
const accent = computed(() => (props.quota.provider === 'codex' ? 'bg-codex' : 'bg-claude'))
const hasAnyWindow = computed(() => props.quota.fiveHour !== null || props.quota.weekly !== null)
const showQuotas = computed(() => hasAnyWindow.value && (props.quota.status === 'ready' || props.quota.status === 'stale'))
const isStale = computed(() => props.quota.status === 'stale')
</script>

<template>
  <section class="rounded-panel border border-hairline bg-panel-raised/80 p-3.5">
    <header class="mb-3 flex items-center gap-2">
      <span class="size-1.5 rounded-full" :class="accent" />
      <h2 class="text-[11px] font-semibold tracking-[0.14em] text-ink uppercase">{{ label }}</h2>
      <span
        v-if="quota.plan"
        class="rounded-full border border-hairline px-1.5 py-px text-[10px] font-medium text-ink-faint uppercase"
      >
        {{ quota.plan }}
      </span>
      <span class="ml-auto text-[10px] tabular-nums" :class="isStale ? 'text-warn' : 'text-ink-faint'">
        <template v-if="isStale">stale · {{ formatRelativeAge(quota.updatedAt, now) }}</template>
        <template v-else-if="showQuotas">{{ formatRelativeAge(quota.updatedAt, now) }}</template>
      </span>
    </header>

    <div v-if="showQuotas" class="space-y-3">
      <QuotaRow label="5h" :window="quota.fiveHour" :now="now" />
      <QuotaRow label="Weekly" :window="quota.weekly" :now="now" />
    </div>

    <ProviderStatus v-else :status="quota.status" :provider-label="label" :error="quota.error" />
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { PROVIDER_LABEL, type ProviderId, type AppSettings, type Diagnostics } from '../types/usage'
import { formatRelativeAge } from '../lib/format'
import ToggleRow from './ToggleRow.vue'

const props = defineProps<{
  settings: AppSettings
  diagnostics: Diagnostics | null
  busy: boolean
  now: number
  error: string | null
}>()

const emit = defineEmits<{
  'set-autostart': [boolean]
  'set-claude-integration': [boolean]
  'set-antigravity-integration': [boolean]
  'set-provider-monitoring': [ProviderId, boolean]
}>()

const claudeInstalled = computed(() => Boolean(props.diagnostics?.claudePath))
const antigravityInstalled = computed(() => Boolean(props.diagnostics?.antigravityPath))
const providerIds: ProviderId[] = ['codex', 'claude', 'antigravity']

const rows = computed(() => {
  const d = props.diagnostics
  if (!d) return []
  const codexLabel = d.codexStatus === 'not-installed'
    ? 'not found'
    : d.codexVersion ?? (d.codexPath ? 'detected' : 'not found')
  return [
    ['Version', d.appVersion],
    ['Codex', codexLabel],
    ['Codex app-server', d.codexAppServerStatus],
    ['Claude Code', d.claudeVersion ?? (d.claudePath ? 'detected' : 'not found')],
    ['Claude data', d.claudeCacheUpdatedAt ? formatRelativeAge(d.claudeCacheUpdatedAt, props.now) : 'none yet'],
    ['Antigravity CLI', d.antigravityVersion ?? (d.antigravityPath ? 'detected' : 'not found')],
    ['Antigravity data', d.antigravityCacheUpdatedAt ? formatRelativeAge(d.antigravityCacheUpdatedAt, props.now) : 'none yet'],
  ] as const
})
</script>

<template>
  <div class="max-h-[330px] space-y-1 overflow-y-auto border-t border-hairline/70 px-2.5 py-2">
    <ToggleRow
      label="Start with Windows"
      :model-value="settings.startWithWindows"
      :disabled="busy"
      @update:model-value="emit('set-autostart', $event)"
    />
    <div class="border-t border-hairline/50 pt-2">
      <h2 class="px-2 text-[10px] font-semibold tracking-wider text-ink-dim uppercase">Monitored providers</h2>
      <p class="px-2 pt-1 pb-0.5 text-[11px] text-ink-faint">Choose the subscriptions you want to see.</p>
      <ToggleRow
        v-for="provider in providerIds"
        :key="provider"
        :label="PROVIDER_LABEL[provider]"
        :model-value="settings.monitoring[provider]"
        :disabled="busy"
        @update:model-value="emit('set-provider-monitoring', provider, $event)"
      />
    </div>

    <details class="border-t border-hairline/50 px-2 pt-2 pb-1">
      <summary class="cursor-pointer text-[11px] text-ink-dim focus-visible:outline-ink-faint">Usage connections</summary>
      <ToggleRow
        label="Claude usage integration"
        :hint="
          claudeInstalled
            ? 'Installs this widget as Claude Code\'s status line. Any existing status line keeps working.'
            : 'Claude Code was not found on PATH.'
        "
        :model-value="settings.claudeIntegrationEnabled"
        :disabled="busy || !claudeInstalled"
        @update:model-value="emit('set-claude-integration', $event)"
      />

      <ToggleRow
        label="Antigravity usage integration"
        :hint="antigravityInstalled
          ? 'Enable, then open Antigravity CLI and run /usage. Your current status line is preserved.'
          : 'Antigravity CLI (agy) was not found.'"
        :model-value="settings.antigravityIntegrationEnabled"
        :disabled="busy || (!antigravityInstalled && !settings.antigravityIntegrationEnabled)"
        @update:model-value="emit('set-antigravity-integration', $event)"
      />
    </details>

    <p v-if="error" class="px-2 pt-1 text-[11px] leading-snug text-low">{{ error }}</p>

    <details v-if="rows.length" class="border-t border-hairline/50 px-2 pt-2 pb-1">
      <summary class="cursor-pointer text-[11px] text-ink-dim focus-visible:outline-ink-faint">Diagnostics</summary>
      <dl class="mt-2 space-y-0.5">
        <div v-for="[key, value] in rows" :key="key" class="flex gap-2 text-[10.5px]">
          <dt class="w-28 shrink-0 text-ink-faint">{{ key }}</dt>
          <dd class="min-w-0 flex-1 truncate text-ink-dim" :title="value">{{ value }}</dd>
        </div>
      </dl>
    </details>
  </div>
</template>

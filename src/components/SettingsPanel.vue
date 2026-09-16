<script setup lang="ts">
import { computed } from 'vue'
import type { AppSettings, Diagnostics } from '../types/usage'
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
}>()

const claudeInstalled = computed(() => Boolean(props.diagnostics?.claudePath))

const rows = computed(() => {
  const d = props.diagnostics
  if (!d) return []
  return [
    ['Version', d.appVersion],
    ['Codex', d.codexVersion ?? (d.codexPath ? 'detected' : 'not found')],
    ['Codex app-server', d.codexAppServerStatus],
    ['Claude Code', d.claudeVersion ?? (d.claudePath ? 'detected' : 'not found')],
    ['Claude data', d.claudeCacheUpdatedAt ? formatRelativeAge(d.claudeCacheUpdatedAt, props.now) : 'none yet'],
  ] as const
})
</script>

<template>
  <div class="space-y-1 border-t border-hairline/70 px-2.5 py-2">
    <ToggleRow
      label="Start with Windows"
      :model-value="settings.startWithWindows"
      :disabled="busy"
      @update:model-value="emit('set-autostart', $event)"
    />
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

    <p v-if="error" class="px-2 pt-1 text-[11px] leading-snug text-low">{{ error }}</p>

    <dl v-if="rows.length" class="mt-1 space-y-0.5 border-t border-hairline/50 px-2 pt-2">
      <div v-for="[key, value] in rows" :key="key" class="flex gap-2 text-[10.5px]">
        <dt class="w-28 shrink-0 text-ink-faint">{{ key }}</dt>
        <dd class="min-w-0 flex-1 truncate text-ink-dim" :title="value">{{ value }}</dd>
      </div>
    </dl>
  </div>
</template>

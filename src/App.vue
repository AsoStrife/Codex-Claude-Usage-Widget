<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useUsageStore } from './stores/usage'
import { useSettingsStore } from './stores/settings'
import { invoke, isTauri, listen } from './lib/tauri'
import WidgetHeader from './components/WidgetHeader.vue'
import ProviderCard from './components/ProviderCard.vue'
import WidgetFooter from './components/WidgetFooter.vue'
import SettingsPanel from './components/SettingsPanel.vue'

const usage = useUsageStore()
const settings = useSettingsStore()
const settingsOpen = ref(false)
const root = ref<HTMLElement | null>(null)

/** Keeps the window exactly as tall as its content, so no dead space is shown. */
async function syncWindowHeight() {
  if (!isTauri || !root.value) return
  await nextTick()
  const height = Math.ceil(root.value.getBoundingClientRect().height)
  try {
    await invoke('resize_popup', { height })
  } catch {
    /* the window may be closing */
  }
}

const providerList = computed(() => [usage.codex, usage.claude])

async function hideWindow() {
  if (!isTauri) return
  try {
    await invoke('hide_popup')
  } catch {
    /* the window may already be hidden */
  }
}

function beginMove() {
  if (!isTauri) return
  void invoke('begin_popup_drag').catch(() => {
    /* the native window may already be closing */
  })
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    if (settingsOpen.value) {
      settingsOpen.value = false
      return
    }
    void hideWindow()
  }
  if (event.key === 'r' && (event.ctrlKey || event.metaKey)) {
    event.preventDefault()
    void usage.refreshAll()
  }
}

async function toggleSettings() {
  settingsOpen.value = !settingsOpen.value
  if (settingsOpen.value) await settings.loadDiagnostics()
}

let unlistenShown: (() => void) | null = null
let resizeObserver: ResizeObserver | null = null

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  await Promise.all([usage.initialize(), settings.initialize()])

  if (root.value) {
    resizeObserver = new ResizeObserver(() => void syncWindowHeight())
    resizeObserver.observe(root.value)
  }
  await syncWindowHeight()
  // The backend re-emits this whenever the popup is revealed from the tray.
  unlistenShown = await listen('popup://shown', () => {
    settingsOpen.value = false
    void usage.refreshSnapshot()
  })
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  unlistenShown?.()
  resizeObserver?.disconnect()
  usage.dispose()
})

// Provider cards grow and shrink as statuses change; follow them.
watch([providerList, settingsOpen], () => void syncWindowHeight(), { deep: true })
</script>

<template>
  <div
    ref="root"
    class="flex w-screen flex-col overflow-hidden border border-hairline bg-panel text-ink"
  >
    <WidgetHeader
      :refreshing="usage.refreshing"
      :settings-open="settingsOpen"
      @refresh="usage.refreshAll()"
      @toggle-settings="toggleSettings()"
      @move-start="beginMove()"
    />

    <main class="max-h-[420px] flex-1 space-y-2.5 overflow-y-auto px-3.5 pb-3">
      <ProviderCard v-for="quota in providerList" :key="quota.provider" :quota="quota" :now="usage.now" />
    </main>

    <Transition name="fade-slide">
      <SettingsPanel
        v-if="settingsOpen"
        :settings="settings.settings"
        :diagnostics="settings.diagnostics"
        :busy="settings.busy"
        :now="usage.now"
        :error="settings.lastError"
        @set-autostart="settings.setStartWithWindows($event)"
        @set-claude-integration="settings.setClaudeIntegration($event)"
      />
    </Transition>

    <WidgetFooter :providers="providerList" :now="usage.now" :error="usage.lastError" />
  </div>
</template>

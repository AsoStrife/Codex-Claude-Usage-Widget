<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
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

let pendingFrame = 0
let lastSentHeight = -1

/**
 * Keeps the window exactly as tall as its content, so no dead space is shown.
 *
 * Measurements are coalesced into a single animation frame and identical
 * heights are dropped: `resize_popup` is an IPC round-trip that triggers a
 * layout, which the ResizeObserver would otherwise feed straight back in.
 */
function syncWindowHeight() {
  if (!isTauri) return
  if (pendingFrame) return
  pendingFrame = requestAnimationFrame(() => {
    pendingFrame = 0
    const el = root.value
    if (!el) return
    const height = Math.ceil(el.getBoundingClientRect().height)
    if (height <= 0 || height === lastSentHeight) return
    lastSentHeight = height
    void invoke('resize_popup', { height }).catch(() => {
      // The window may be closing; let the next measurement retry.
      lastSentHeight = -1
    })
  })
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

function toggleSettings() {
  settingsOpen.value = !settingsOpen.value
  if (settingsOpen.value) void settings.loadDiagnostics()
}

let unlistenShown: (() => void) | null = null
let resizeObserver: ResizeObserver | null = null

onMounted(() => {
  window.addEventListener('keydown', onKeydown)

  // Wired up before any `await`: a rejected IPC call used to abort the rest of
  // this hook, leaving the window stuck at its design-time height with no
  // observer and no event subscriptions.
  if (root.value) {
    resizeObserver = new ResizeObserver(syncWindowHeight)
    resizeObserver.observe(root.value)
  }
  syncWindowHeight()

  void usage.initialize()
  void settings.initialize()

  // The backend re-emits this whenever the popup is revealed from the tray.
  void listen('popup://shown', () => {
    settingsOpen.value = false
    void usage.refreshAll()
  })
    .then((unlisten) => {
      unlistenShown = unlisten
    })
    .catch(() => {
      /* events unavailable: the polling fallback in the store still applies */
    })
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  if (pendingFrame) cancelAnimationFrame(pendingFrame)
  unlistenShown?.()
  resizeObserver?.disconnect()
  usage.dispose()
})
</script>

<template>
  <div
    ref="root"
    class="flex w-full flex-col overflow-hidden border border-hairline bg-panel text-ink"
  >
    <WidgetHeader
      :refreshing="usage.refreshing"
      :settings-open="settingsOpen"
      @refresh="usage.refreshAll()"
      @toggle-settings="toggleSettings()"
      @move-start="beginMove()"
    />

    <main class="max-h-[420px] space-y-2.5 overflow-y-auto px-3.5 pb-3">
      <ProviderCard v-for="quota in providerList" :key="quota.provider" :quota="quota" :now="usage.now" />
    </main>

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

    <WidgetFooter
      :providers="providerList"
      :now="usage.now"
      :error="usage.lastError"
      @move-start="beginMove()"
    />
  </div>
</template>

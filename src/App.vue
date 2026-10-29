<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { PROVIDER_LABEL, type ProviderId } from './types/usage'
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
const pinned = ref(false)
const pinBusy = ref(isTauri)
const pinError = ref<string | null>(null)
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

type TabId = 'main' | 'antigravity'
const activeTab = ref<TabId>('main')
const tabGroups: { id: TabId; providers: ProviderId[] }[] = [
  { id: 'main', providers: ['codex', 'claude'] },
  { id: 'antigravity', providers: ['antigravity'] },
]
const availableTabs = computed(() => tabGroups.map((group) => {
  const providers = group.providers.filter((id) => settings.settings.monitoring[id])
  return { id: group.id, providers, label: providers.map((id) => PROVIDER_LABEL[id]).join(' & ') }
}).filter((group) => group.providers.length > 0))
watch(availableTabs, (tabs) => {
  if (!tabs.some((tab) => tab.id === activeTab.value)) activeTab.value = tabs[0]?.id ?? 'main'
})
const currentTab = computed(() => availableTabs.value.find((tab) => tab.id === activeTab.value) ?? availableTabs.value[0])
const providerList = computed(() => (currentTab.value?.providers ?? []).map((id) => usage[id]))

function resetTab() {
  activeTab.value = availableTabs.value.find((tab) => tab.id === 'main')?.id ?? availableTabs.value[0]?.id ?? 'main'
}

function onTabKeydown(event: KeyboardEvent, index: number) {
  const tabs = availableTabs.value
  let next = index
  if (event.key === 'ArrowRight') next = (index + 1) % tabs.length
  else if (event.key === 'ArrowLeft') next = (index + tabs.length - 1) % tabs.length
  else if (event.key === 'Home') next = 0
  else if (event.key === 'End') next = tabs.length - 1
  else return
  event.preventDefault()
  const tab = tabs[next]
  if (!tab) return
  activeTab.value = tab.id
  document.getElementById(`tab-${tab.id}`)?.focus()
}

async function togglePin() {
  if (pinBusy.value) return
  pinBusy.value = true
  pinError.value = null
  try {
    const next = !pinned.value
    pinned.value = isTauri ? await invoke<boolean>('set_popup_pinned', { pinned: next }) : next
  } catch (error) {
    pinError.value = `Unable to change widget pin: ${String(error)}`
  } finally {
    pinBusy.value = false
  }
}

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

  if (isTauri) {
    void invoke<boolean>('get_popup_pinned')
      .then((value) => { pinned.value = value })
      .catch((error) => { pinError.value = `Unable to read widget pin: ${String(error)}` })
      .finally(() => { pinBusy.value = false })
  }

  // The backend re-emits this whenever the popup is revealed from the tray.
  void listen('popup://shown', () => {
    settingsOpen.value = false
    resetTab()
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
      :pinned="pinned"
      :pin-busy="pinBusy"
      @refresh="usage.refreshAll()"
      @toggle-settings="toggleSettings()"
      @toggle-pin="togglePin()"
      @move-start="beginMove()"
    />

    <div v-if="availableTabs.length > 1" role="tablist" aria-label="Usage providers" class="mx-3.5 mb-3 flex rounded-lg border border-hairline bg-panel-raised/50 p-1">
      <button
        v-for="(tab, index) in availableTabs"
        :id="`tab-${tab.id}`"
        :key="tab.id"
        type="button"
        role="tab"
        :aria-selected="activeTab === tab.id"
        :aria-controls="`panel-${tab.id}`"
        :tabindex="activeTab === tab.id ? 0 : -1"
        class="flex-1 rounded-md px-3 py-1.5 text-[11px] font-medium transition-colors focus-visible:ring-1 focus-visible:ring-ink-faint focus-visible:outline-none"
        :class="activeTab === tab.id ? 'bg-hairline/70 text-ink' : 'text-ink-faint hover:text-ink-dim'"
        @click="activeTab = tab.id"
        @keydown="onTabKeydown($event, index)"
      >{{ tab.label }}</button>
    </div>

    <main
      :id="`panel-${currentTab?.id ?? 'main'}`"
      :role="availableTabs.length > 1 ? 'tabpanel' : undefined"
      :aria-labelledby="availableTabs.length > 1 ? `tab-${currentTab?.id}` : undefined"
      :aria-label="availableTabs.length === 1 ? currentTab?.label : undefined"
      :tabindex="availableTabs.length > 1 ? 0 : undefined"
      class="space-y-2.5 overflow-y-auto px-3.5 pb-3 focus-visible:outline-none"
      :class="settingsOpen ? 'max-h-[240px]' : 'max-h-[420px]'"
    >
      <ProviderCard v-for="quota in providerList" :key="quota.provider" :quota="quota" :now="usage.now" />
      <div v-if="!providerList.length" class="rounded-panel border border-hairline bg-panel-raised/80 px-4 py-7 text-center">
        <p class="text-[12px] text-ink-dim">No providers enabled</p>
        <button type="button" class="mt-2 rounded-md px-2 py-1 text-[11px] text-ink underline underline-offset-4 focus-visible:ring-1 focus-visible:ring-ink-faint focus-visible:outline-none" @click="!settingsOpen && toggleSettings()">
          Choose providers in Settings
        </button>
      </div>
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
      @set-antigravity-integration="settings.setAntigravityIntegration($event)"
      @set-provider-monitoring="settings.setProviderMonitoring"
    />

    <WidgetFooter
      :providers="providerList"
      :now="usage.now"
      :error="pinError || usage.lastError"
      @move-start="beginMove()"
    />
  </div>
</template>

<script setup lang="ts">
defineProps<{
  refreshing: boolean
  settingsOpen: boolean
}>()

const emit = defineEmits<{
  refresh: []
  'toggle-settings': []
  'move-start': []
}>()

function onMouseDown(event: MouseEvent) {
  if (event.button !== 0 || (event.target as HTMLElement).closest('button, a, input, select, textarea')) return
  emit('move-start')
}
</script>

<template>
  <header
    class="flex cursor-move items-center gap-2 px-4 pt-3.5 pb-2"
    title="Drag to move the widget"
    @mousedown="onMouseDown"
  >
    <h1 class="text-[13px] font-semibold tracking-tight text-ink">AI Usage</h1>
    <div class="ml-auto flex cursor-default items-center gap-1">
      <button
        type="button"
        class="grid size-7 place-items-center rounded-md text-ink-dim transition-colors hover:bg-hairline/60 hover:text-ink focus-visible:ring-1 focus-visible:ring-ink-faint focus-visible:outline-none disabled:opacity-50"
        :disabled="refreshing"
        title="Refresh"
        aria-label="Refresh"
        @click="emit('refresh')"
      >
        <svg
          viewBox="0 0 16 16"
          class="size-3.5"
          :class="{ 'animate-spin': refreshing }"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        >
          <path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9" />
          <path d="M13.2 1.8v2.9h-2.9" />
        </svg>
      </button>
      <button
        type="button"
        class="grid size-7 place-items-center rounded-md transition-colors hover:bg-hairline/60 hover:text-ink focus-visible:ring-1 focus-visible:ring-ink-faint focus-visible:outline-none"
        :class="settingsOpen ? 'bg-hairline/60 text-ink' : 'text-ink-dim'"
        title="Settings"
        aria-label="Settings"
        @click="emit('toggle-settings')"
      >
        <svg viewBox="0 0 16 16" class="size-3.5" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="8" cy="8" r="2.1" />
          <path
            d="M8 1.6v1.5M8 12.9v1.5M14.4 8h-1.5M3.1 8H1.6M12.5 3.5l-1 1M4.5 11.5l-1 1M12.5 12.5l-1-1M4.5 4.5l-1-1"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>
  </header>
</template>

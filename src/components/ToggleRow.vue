<script setup lang="ts">
defineProps<{
  label: string
  hint?: string | null
  modelValue: boolean
  disabled?: boolean
}>()

const emit = defineEmits<{ 'update:modelValue': [boolean] }>()
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    class="flex w-full items-start gap-3 rounded-lg px-2 py-2 text-left transition-colors hover:bg-hairline/40 focus-visible:ring-1 focus-visible:ring-ink-faint focus-visible:outline-none disabled:opacity-50"
    @click="emit('update:modelValue', !modelValue)"
  >
    <span class="min-w-0 flex-1">
      <span class="block text-[12px] text-ink">{{ label }}</span>
      <span v-if="hint" class="mt-0.5 block text-[11px] leading-snug text-ink-faint">{{ hint }}</span>
    </span>
    <span
      class="mt-0.5 flex h-4 w-7 shrink-0 items-center rounded-full p-0.5 transition-colors"
      :class="modelValue ? 'bg-codex/80' : 'bg-hairline'"
    >
      <span
        class="size-3 rounded-full bg-panel transition-transform"
        :class="modelValue ? 'translate-x-3' : 'translate-x-0'"
      />
    </span>
  </button>
</template>

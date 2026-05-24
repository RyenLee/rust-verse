<script setup lang="ts">
withDefaults(defineProps<{
  modelValue: string
  placeholder?: string
  showShortcut?: boolean
}>(), {
  placeholder: 'Search...',
  showShortcut: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function onInput(event: Event) {
  emit('update:modelValue', (event.target as HTMLInputElement).value)
}
</script>

<template>
  <div class="h-9 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg flex items-center focus-within:ring-2 focus-within:ring-sky-500 focus-within:border-transparent">
    <iconify-icon icon="mdi:magnify" width="18" class="text-gray-400 ml-3 shrink-0" />
    <input
      type="text"
      :value="modelValue"
      :placeholder="placeholder"
      class="w-full h-full px-2 bg-transparent text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none"
      @input="onInput"
    />
    <span v-if="showShortcut" class="text-xs text-gray-400 mr-3 shrink-0">⌘K</span>
  </div>
</template>

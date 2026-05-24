<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'

const props = withDefaults(defineProps<{
  modelValue: string
  toolchains: Array<{ name: string; is_default: boolean; channel: string }>
  disabled?: boolean
}>(), {
  disabled: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  change: [value: string]
}>()

const open = ref(false)
const dropdownRef = ref<HTMLElement | null>(null)

function toggle() {
  if (!props.disabled) {
    open.value = !open.value
  }
}

function select(toolchain: { name: string }) {
  emit('update:modelValue', toolchain.name)
  emit('change', toolchain.name)
  open.value = false
}

function onClickOutside(event: MouseEvent) {
  if (dropdownRef.value && !dropdownRef.value.contains(event.target as Node)) {
    open.value = false
  }
}

onMounted(() => {
  if (!props.modelValue) {
    const defaultToolchain = props.toolchains.find(t => t.is_default)
    if (defaultToolchain) {
      emit('update:modelValue', defaultToolchain.name)
    }
  }
  document.addEventListener('click', onClickOutside)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', onClickOutside)
})
</script>

<template>
  <div ref="dropdownRef" class="relative inline-flex h-9">
    <button
      type="button"
      :disabled="disabled"
      class="flex items-center gap-2 h-9 px-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-gray-100 hover:border-gray-300 dark:hover:border-gray-600 transition-colors disabled:opacity-50"
      @click="toggle"
    >
      <iconify-icon icon="mdi:wrench-outline" width="16" class="text-gray-400 shrink-0" />
      <span class="truncate max-w-[200px]">{{ modelValue || 'Select...' }}</span>
      <iconify-icon icon="mdi:chevron-down" width="16" class="text-gray-400 shrink-0" />
    </button>
    <div
      v-if="open"
      class="absolute top-full left-0 right-0 mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg z-50 overflow-hidden"
    >
      <button
        v-for="toolchain in toolchains"
        :key="toolchain.name"
        type="button"
        class="flex items-center gap-2 w-full px-3 py-2 text-sm hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors text-left"
        @click="select(toolchain)"
      >
        <span class="truncate">{{ toolchain.name }}</span>
        <span class="text-xs text-gray-400 shrink-0">{{ toolchain.channel }}</span>
        <span v-if="toolchain.is_default" class="text-xs text-sky-500 shrink-0">默认</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = withDefaults(defineProps<{
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
  loading?: boolean
  disabled?: boolean
}>(), {
  variant: 'primary',
  loading: false,
  disabled: false,
})

const emit = defineEmits<{
  click: []
}>()

function onClick() {
  if (!props.loading && !props.disabled) {
    emit('click')
  }
}
</script>

<template>
  <button
    :disabled="disabled || loading"
    :class="[
      'inline-flex items-center justify-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors',
      variant === 'primary' && 'bg-sky-600 hover:bg-sky-500 text-white disabled:opacity-50',
      variant === 'secondary' && 'bg-gray-100 hover:bg-gray-200 text-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-300 disabled:opacity-50',
      variant === 'danger' && 'bg-red-600 hover:bg-red-500 text-white disabled:opacity-50',
      variant === 'ghost' && 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-50',
    ]"
    @click="onClick"
  >
    <svg
      v-if="loading"
      class="animate-spin h-4 w-4"
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
    >
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
    </svg>
    <slot />
  </button>
</template>

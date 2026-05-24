<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  value: number | null
  max?: number
  label?: string
}>(), {
  max: 500,
})

const percentage = computed(() => {
  if (props.value === null) return 0
  return Math.min((props.value / props.max) * 100, 100)
})

const barColor = computed(() => {
  if (props.value === null) return ''
  if (props.value < 100) return 'bg-green-500'
  if (props.value <= 300) return 'bg-amber-500'
  return 'bg-red-500'
})

const textColor = computed(() => {
  if (props.value === null) return ''
  if (props.value < 100) return 'text-green-500'
  if (props.value <= 300) return 'text-amber-500'
  return 'text-red-500'
})
</script>

<template>
  <div>
    <div v-if="label" class="text-xs text-gray-500 dark:text-gray-400 mb-1">{{ label }}</div>
    <div v-if="value === null" class="text-gray-300">—</div>
    <template v-else>
      <div class="h-1.5 bg-gray-100 dark:bg-gray-700 rounded-full overflow-hidden">
        <div
          class="h-full rounded-full transition-all"
          :class="barColor"
          :style="{ width: percentage + '%' }"
        />
      </div>
      <span class="text-xs font-mono" :class="textColor">{{ value }}ms</span>
    </template>
  </div>
</template>

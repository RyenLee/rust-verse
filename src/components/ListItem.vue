<script setup lang="ts">
withDefaults(defineProps<{
  title: string
  description?: string
  active?: boolean
}>(), {
  active: false,
})
</script>

<template>
  <div class="bg-white dark:bg-gray-800 rounded-xl px-4 py-3 border border-gray-200 dark:border-gray-700 flex items-center justify-between hover:shadow-sm transition-shadow">
    <div class="flex items-center gap-3 min-w-0">
      <span
        class="w-2 h-2 rounded-full shrink-0"
        :class="active ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"
      />
      <div class="min-w-0">
        <div class="item-title truncate"><slot name="title">{{ title }}</slot></div>
        <div v-if="description" class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate">{{ description }}</div>
      </div>
    </div>
    <div class="flex items-center gap-2 shrink-0">
      <slot name="badges" />
      <slot name="actions" />
    </div>
  </div>
</template>

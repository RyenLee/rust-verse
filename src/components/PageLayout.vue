<script setup lang="ts">
defineProps<{
  group: string
  title: string
  description?: string
}>()
</script>

<template>
  <div class="flex flex-col h-full">
    <!-- TopBar -->
    <header class="shrink-0 h-14 border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-950 flex items-center px-6 gap-4">
      <!-- Left: breadcrumb + title + description -->
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2 text-sm">
          <span class="text-gray-400 dark:text-gray-500">{{ group }}</span>
          <iconify-icon icon="mdi:chevron-right" width="16" class="text-gray-300 dark:text-gray-600"></iconify-icon>
          <span class="page-title text-gray-900 dark:text-gray-100">{{ title }}</span>
        </div>
        <p v-if="description" class="text-xs text-gray-500 dark:text-gray-400 mt-0.5 truncate">{{ description }}</p>
      </div>
      <!-- Right: actions slot -->
      <div class="flex items-center gap-2 shrink-0">
        <slot name="actions" />
      </div>
    </header>

    <!-- Filters (optional) -->
    <div v-if="$slots.filters" class="shrink-0 pt-4 pb-3">
      <div class="max-w-6xl mx-auto px-6 flex flex-wrap items-center gap-3">
        <slot name="filters" />
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto scroll-container">
      <div class="px-6 pt-4 pb-6 max-w-6xl mx-auto">
        <slot />
      </div>
    </div>
  </div>
</template>

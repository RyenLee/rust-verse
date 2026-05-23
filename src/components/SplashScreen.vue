<script setup lang="ts">
import { computed } from 'vue'
import { useStore } from '../store'

const store = useStore()

const props = defineProps<{
  /** Current progress percentage (0-100) */
  progress?: number
  /** Current status text shown below the progress bar */
  statusText?: string
  /** Startup errors collected during boot sequence */
  startupErrors?: string[]
}>()

const hasErrors = computed(() => (props.startupErrors?.length ?? 0) > 0)
const displayProgress = computed(() => props.progress ?? 0)
</script>

<template>
  <div class="fixed inset-0 z-[999] flex items-center justify-center bg-white dark:bg-gray-950 select-none">
    <div class="flex flex-col items-center gap-8">
      <!-- Logo -->
      <div class="flex items-center gap-4">
        <svg viewBox="0 0 48 48" width="48" height="48" fill="none" xmlns="http://www.w3.org/2000/svg">
          <circle cx="24" cy="24" r="22" stroke="#f97316" stroke-width="4" fill="none" />
          <path d="M24 12 L32 20 L24 36 L16 20 Z" fill="#f97316" />
          <circle cx="24" cy="20" r="4" fill="#f97316" />
        </svg>
        <h1 class="text-3xl font-bold text-gray-900 dark:text-gray-100 tracking-tight">{{ store.appName || 'RustVerse' }}</h1>
      </div>

      <!-- Progress bar with percentage -->
      <div class="w-64">
        <div class="flex items-center justify-between mb-2">
          <span class="text-xs text-gray-400">{{ statusText || 'Loading...' }}</span>
          <span class="text-xs font-mono text-gray-500">{{ displayProgress }}%</span>
        </div>
        <div class="h-1.5 bg-gray-200 dark:bg-gray-800 rounded-full overflow-hidden">
          <div
            class="h-full bg-orange-500 rounded-full transition-all duration-500 ease-out"
            :style="{ width: displayProgress + '%' }"
          />
        </div>
      </div>

      <!-- Startup errors (if any steps failed or timed out) -->
      <div
        v-if="hasErrors"
        class="w-80 max-h-40 overflow-y-auto bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-3"
      >
        <p class="text-xs font-semibold text-amber-700 dark:text-amber-400 mb-2">
          Startup Warnings
        </p>
        <ul class="space-y-1">
          <li
            v-for="(err, i) in startupErrors"
            :key="i"
            class="text-xs font-mono text-amber-600 dark:text-amber-400 break-all"
          >
            {{ err }}
          </li>
        </ul>
        <p class="text-xs text-amber-500 dark:text-amber-500 mt-2">
          Some startup steps failed. App may have limited functionality.
        </p>
      </div>
    </div>
  </div>
</template>

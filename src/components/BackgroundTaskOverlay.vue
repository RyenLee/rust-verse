<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useBackgroundTask } from '@/composables/useBackgroundTask'

const { t } = useI18n()
const { state, restore, requestCancel } = useBackgroundTask()

const statusIcon = computed(() => {
  switch (state.status) {
    case 'running':
      return 'mdi:loading'
    case 'completed':
      return 'mdi:check-circle-outline'
    case 'failed':
      return 'mdi:alert-circle-outline'
    default:
      return 'mdi:information-outline'
  }
})

const iconClass = computed(() => {
  switch (state.status) {
    case 'running':
      return 'animate-spin text-sky-500'
    case 'completed':
      return 'text-green-500'
    case 'failed':
      return 'text-red-500'
    default:
      return 'text-gray-400'
  }
})

const statusBadge = computed(() => {
  switch (state.status) {
    case 'running':
      return t('progress.status.running')
    case 'completed':
      return t('progress.status.completed')
    case 'failed':
      return t('progress.status.failed')
    default:
      return ''
  }
})
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay-slide">
      <div
        v-if="state.visible && state.minimized"
        class="fixed bottom-6 right-6 z-[9999] bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-2xl w-80 overflow-hidden"
      >
        <!-- Header -->
        <div
          class="flex items-center justify-between px-4 py-2.5 border-b border-gray-100 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50"
        >
          <div class="flex items-center gap-2 min-w-0">
            <iconify-icon :icon="statusIcon" width="16" :class="iconClass" />
            <span class="text-sm font-medium text-gray-800 dark:text-gray-200 truncate">
              {{ state.title }}
            </span>
          </div>
          <span
            class="text-xs px-1.5 py-0.5 rounded-full bg-sky-50 dark:bg-sky-900/30 text-sky-600 dark:text-sky-400 shrink-0 ml-2"
          >
            {{ statusBadge }}
          </span>
        </div>

        <!-- Progress bar (running only) -->
        <div v-if="state.status === 'running'" class="h-1 bg-gray-100 dark:bg-gray-700">
          <div
            class="h-full bg-sky-500 transition-all duration-500 ease-out"
            :style="{ width: state.progress + '%' }"
          />
        </div>

        <!-- Last log line -->
        <div class="px-4 py-2">
          <p class="text-xs text-gray-500 dark:text-gray-400 font-mono truncate">
            {{ state.lines.length > 0 ? state.lines[state.lines.length - 1] : t('progress.status.waiting') }}
          </p>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-1 px-3 py-2 border-t border-gray-100 dark:border-gray-700">
          <button
            v-if="state.status === 'running'"
            class="flex-1 py-1.5 rounded-lg text-xs font-medium text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors cursor-pointer"
            @click="requestCancel()"
          >
            {{ t('progress.task.cancel') }}
          </button>
          <button
            class="py-1.5 px-3 rounded-lg text-xs font-medium text-sky-600 dark:text-sky-400 hover:bg-sky-50 dark:hover:bg-sky-900/20 transition-colors cursor-pointer"
            @click="restore()"
          >
            {{ t('progress.task.showDetails') }}
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.overlay-slide-enter-active {
  transition: all 0.3s ease-out;
}
.overlay-slide-leave-active {
  transition: all 0.2s ease-in;
}
.overlay-slide-enter-from {
  opacity: 0;
  transform: translateY(24px) scale(0.95);
}
.overlay-slide-leave-to {
  opacity: 0;
  transform: translateY(12px) scale(0.95);
}
</style>

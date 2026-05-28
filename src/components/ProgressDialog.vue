<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = withDefaults(
  defineProps<{
    visible: boolean
    title: string
    status: 'running' | 'success' | 'error'
    statusText?: string
    lines: string[]
    closable?: boolean
  }>(),
  {
    closable: true,
    statusText: '',
  }
)

const emit = defineEmits<{
  close: []
  cancel: []
  minimize: []
}>()

const logContainer = ref<HTMLElement | null>(null)

// Auto-scroll to bottom when new lines arrive
watch(
  () => props.lines.length,
  async () => {
    await nextTick()
    if (logContainer.value) {
      logContainer.value.scrollTo({
        top: logContainer.value.scrollHeight,
        behavior: 'smooth',
      })
    }
  }
)

function handleClose() {
  if (props.closable && props.status !== 'running') {
    emit('close')
  }
}

const statusConfig = computed(() => ({
  running: {
    icon: 'mdi:loading',
    iconClass: 'animate-spin text-sky-600 dark:text-sky-400',
    badgeClass: 'bg-sky-50 dark:bg-sky-900/30 text-sky-700 dark:text-sky-300',
    label: t('progress.status.running'),
  },
  success: {
    icon: 'mdi:check-circle-outline',
    iconClass: 'text-green-600 dark:text-green-400',
    badgeClass: 'bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-300',
    label: t('progress.status.completed'),
  },
  error: {
    icon: 'mdi:alert-circle-outline',
    iconClass: 'text-red-600 dark:text-red-400',
    badgeClass: 'bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-300',
    label: t('progress.status.failed'),
  },
}))
</script>

<template>
  <Teleport to="body">
    <Transition name="progress-dialog">
      <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center p-4" @click.self="handleClose">
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/50" @click="handleClose" />

        <!-- Dialog -->
        <div
          class="relative bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-2xl w-[90vw] max-w-2xl flex flex-col max-h-[85vh]"
        >
          <!-- Header -->
          <div
            class="flex items-center justify-between px-5 py-4 border-b border-gray-200 dark:border-gray-700 shrink-0"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div
                class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0"
                :class="statusConfig[status].badgeClass"
              >
                <iconify-icon
                  :icon="statusConfig[status].icon"
                  width="18"
                  :class="statusConfig[status].iconClass"
                ></iconify-icon>
              </div>
              <div class="min-w-0">
                <h2 class="text-base font-semibold text-gray-900 dark:text-gray-100 truncate">{{ title }}</h2>
                <p v-if="statusText" class="text-xs text-gray-500 dark:text-gray-400 truncate mt-0.5">
                  {{ statusText }}
                </p>
              </div>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <span class="text-xs font-medium px-2 py-0.5 rounded-full" :class="statusConfig[status].badgeClass">
                {{ statusConfig[status].label }}
              </span>
              <button
                v-if="closable && status !== 'running'"
                class="p-1 rounded-md text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                @click="emit('close')"
              >
                <iconify-icon icon="mdi:close" width="18"></iconify-icon>
              </button>
            </div>
          </div>

          <!-- Progress bar (indeterminate when running) -->
          <div v-if="status === 'running'" class="h-0.5 bg-gray-100 dark:bg-gray-700 shrink-0 overflow-hidden">
            <div class="h-full bg-sky-500 animate-indeterminate-progress" />
          </div>

          <!-- Log content -->
          <div
            ref="logContainer"
            class="log-scroll-area flex-1 min-h-0 overflow-y-auto p-4 bg-gray-50 dark:bg-gray-900"
            :class="lines.length === 0 ? 'flex items-center justify-center' : ''"
          >
            <div
              v-if="lines.length > 0"
              class="text-xs text-gray-600 dark:text-gray-400 whitespace-pre-wrap font-mono leading-relaxed"
            >
              <div v-for="(line, i) in lines" :key="i" class="log-line">{{ line }}</div>
            </div>
            <div v-else class="text-gray-400 dark:text-gray-500 text-sm">{{ t('progress.status.waiting') }}</div>
          </div>

          <!-- Footer -->
          <div class="px-5 py-3 border-t border-gray-200 dark:border-gray-700 flex justify-between shrink-0">
            <div class="flex gap-2">
              <button
                v-if="status === 'running'"
                class="px-4 py-2 rounded-lg text-sm font-medium text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                @click="emit('cancel')"
              >
                {{ t('progress.task.cancel') }}
              </button>
              <button
                v-if="status === 'running'"
                class="px-4 py-2 rounded-lg text-sm font-medium text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                @click="emit('minimize')"
              >
                {{ t('progress.task.minimize') }}
              </button>
            </div>
            <div>
              <button
                v-if="status === 'running'"
                disabled
                class="px-4 py-2 rounded-lg text-sm font-medium text-gray-400 dark:text-gray-500 cursor-not-allowed"
              >
                {{ t('progress.status.inProgress') }}
              </button>
              <button
                v-else
                class="px-4 py-2 rounded-lg text-sm font-medium bg-sky-600 hover:bg-sky-500 text-white transition-colors"
                @click="emit('close')"
              >
                {{ t('common.action.close') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.progress-dialog-enter-active {
  transition: opacity 0.2s ease;
}
.progress-dialog-enter-active > div:last-child {
  transition: transform 0.25s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.2s ease;
}
.progress-dialog-leave-active {
  transition: opacity 0.15s ease;
}
.progress-dialog-enter-from,
.progress-dialog-leave-to {
  opacity: 0;
}
.progress-dialog-enter-from > div:last-child {
  transform: scale(0.95);
  opacity: 0;
}

@keyframes indeterminate-progress {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(400%);
  }
}
.animate-indeterminate-progress {
  animation: indeterminate-progress 1.5s ease-in-out infinite;
  width: 25%;
}

/* Log line scroll-in animation */
@keyframes log-line-in {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
.log-line {
  animation: log-line-in 0.15s ease-out;
}
</style>

<!-- Unscoped styles for scrollbar and dark mode — scoped :root selectors don't work -->
<style>
/* Custom scrollbar for log area */
.log-scroll-area::-webkit-scrollbar {
  width: 6px;
}
.log-scroll-area::-webkit-scrollbar-track {
  background: transparent;
}
.log-scroll-area::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.4);
  border-radius: 3px;
}
.log-scroll-area::-webkit-scrollbar-thumb:hover {
  background-color: rgba(156, 163, 175, 0.6);
}
.dark .log-scroll-area::-webkit-scrollbar-thumb {
  background-color: rgba(75, 85, 99, 0.5);
}
.dark .log-scroll-area::-webkit-scrollbar-thumb:hover {
  background-color: rgba(75, 85, 99, 0.7);
}
</style>

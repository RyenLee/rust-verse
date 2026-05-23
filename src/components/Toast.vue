<script setup lang="ts">
import { useToast } from '../composables/useToast'

const { toasts, removeToast } = useToast()
</script>

<template>
  <Teleport to="body">
    <div class="fixed top-4 right-4 z-[100] flex flex-col gap-2 max-w-sm">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          :class="[
            'flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg border text-sm',
            toast.type === 'success' && 'bg-green-50 dark:bg-green-900/80 border-green-200 dark:border-green-700 text-green-800 dark:text-green-200',
            toast.type === 'error' && 'bg-red-50 dark:bg-red-900/80 border-red-200 dark:border-red-700 text-red-800 dark:text-red-200',
            toast.type === 'info' && 'bg-sky-50 dark:bg-sky-900/80 border-sky-200 dark:border-sky-700 text-sky-800 dark:text-sky-200',
          ]"
        >
          <span class="flex-1">{{ toast.message }}</span>
          <button
            class="opacity-60 hover:opacity-100 transition-opacity text-current"
            @click="removeToast(toast.id)"
          >
            <iconify-icon icon="mdi:close" width="16"></iconify-icon>
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-enter-active {
  transition: all 0.3s ease-out;
}
.toast-leave-active {
  transition: all 0.2s ease-in;
}
.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}
.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>

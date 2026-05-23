<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

withDefaults(
  defineProps<{
    visible?: boolean
    title: string
    message: string
    confirmLabel?: string
    danger?: boolean
  }>(),
  {
    visible: true,
  }
)

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      @click.self="emit('cancel')"
    >
      <div
        class="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-sm border border-gray-200 dark:border-gray-700"
      >
        <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">{{ title }}</h2>
        <p class="text-gray-600 dark:text-gray-400 text-sm mb-4">
          <slot>{{ message }}</slot>
        </p>
        <div class="flex justify-end gap-2">
          <button
            class="px-4 py-2 rounded-lg text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 transition-colors"
            @click="emit('cancel')"
          >
            {{ t('common.action.cancel') }}
          </button>
          <button
            :class="[
              'px-4 py-2 rounded-lg text-sm font-medium text-white transition-colors',
              danger ? 'bg-red-600 hover:bg-red-500' : 'bg-sky-600 hover:bg-sky-500',
            ]"
            @click="emit('confirm')"
          >
            {{ confirmLabel || t('common.action.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

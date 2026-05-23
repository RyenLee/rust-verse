<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  message?: string
}>()
</script>

<template>
  <p class="text-gray-400 dark:text-gray-500 text-center py-8">
    <slot>{{ message || t('common.empty.noItems') }}</slot>
  </p>
</template>

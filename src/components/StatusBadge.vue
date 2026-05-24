<script setup lang="ts">
defineProps<{
  type: 'installed' | 'default' | 'active' | 'updatable' | 'uninstalled' | 'danger'
  label: string
}>()

const classMap: Record<string, string> = {
  installed: 'badge-installed',
  default: 'badge-default',
  active: 'badge-active',
  updatable: 'badge-updatable',
  uninstalled: 'badge-uninstalled',
  danger: 'badge-danger',
}
</script>

<template>
  <span :class="classMap[type]">{{ label }}</span>
</template>

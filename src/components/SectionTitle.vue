<script setup lang="ts">
defineProps<{
  title: string
  count?: number
}>()
</script>

<template>
  <h2 class="section-title flex items-center gap-2 mb-4">
    {{ title }}
    <span v-if="count !== undefined" class="text-sm text-gray-400">({{ count }})</span>
  </h2>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useStore } from '../store'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const { t } = useI18n()
const store = useStore()

const aboutText = computed(() => {
  const name = store.appName || 'RustVerse'
  const version = store.appVersion || '0.0.0'
  return t('help.desc.about', { name, version })
})

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    emit('close')
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="slide-panel">
      <div v-if="visible" class="fixed inset-0 z-50 flex justify-end" @keydown="handleKeydown">
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/40" @click="$emit('close')" />
        <!-- Panel -->
        <div
          class="relative w-full max-w-md bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 shadow-xl flex flex-col"
        >
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700 shrink-0">
            <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{{ t('help.title') }}</h2>
            <button
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
              @click="$emit('close')"
            >
              <iconify-icon icon="mdi:close" width="20"></iconify-icon>
            </button>
          </div>

          <!-- Body -->
          <div class="flex-1 overflow-y-auto p-6 space-y-6">
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.gettingStarted') }}</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400 break-words overflow-hidden" v-html="t('help.desc.gettingStarted', { rustup: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>rustup</code>' })" />
            </section>

            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.toolchains') }}</h3>
              <ul class="list-disc list-inside text-sm text-gray-600 dark:text-gray-400 space-y-1">
                <li>{{ t('help.desc.toolchainsInstall') }}</li>
                <li>{{ t('help.desc.toolchainsDefault') }}</li>
                <li>{{ t('help.desc.toolchainsUninstall') }}</li>
              </ul>
            </section>

            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.components') }}</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400 break-words overflow-hidden" v-html="t('help.desc.components', { rustfmt: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>rustfmt</code>', clippy: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>clippy</code>', miri: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>miri</code>' })" />
            </section>

            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.targets') }}</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.targets') }}</p>
            </section>

            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.directoryOverrides') }}</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400 break-words overflow-hidden" v-html="t('help.desc.directoryOverrides', { command: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>rustup override set &lt;toolchain&gt;</code>' })" />
            </section>

            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.updates') }}</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.updates') }}</p>
            </section>

            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.cargoPlugins') }}</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400 break-words overflow-hidden" v-html="t('help.desc.cargoPlugins', { audit: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>cargo-audit</code>', expand: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>cargo-expand</code>' })" />
            </section>

            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.about') }}</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ aboutText }}</p>
            </section>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.slide-panel-enter-active,
.slide-panel-leave-active {
  transition: opacity 0.2s ease;
}
.slide-panel-enter-active > div:last-child,
.slide-panel-leave-active > div:last-child {
  transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
}
.slide-panel-enter-from,
.slide-panel-leave-to {
  opacity: 0;
}
.slide-panel-enter-from > div:last-child,
.slide-panel-leave-to > div:last-child {
  transform: translateX(100%);
}
</style>

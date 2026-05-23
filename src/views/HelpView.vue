<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useStore } from '../store'

const { t } = useI18n()
const store = useStore()

const aboutText = computed(() => {
  const name = store.appName || 'RustVerse'
  const version = store.appVersion || '0.0.0'
  return t('help.desc.about', { name, version })
})
</script>

<template>
  <div class="max-w-3xl mx-auto space-y-8 h-full overflow-y-auto p-6">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-white">{{ t('help.title') }}</h1>

    <section class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.gettingStarted') }}</h2>
      <p class="text-gray-600 dark:text-gray-400 break-words overflow-hidden" v-html="t('help.desc.gettingStarted', { rustup: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded\'>rustup</code>' })" />
    </section>

    <section class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.toolchains') }}</h2>
      <ul class="list-disc list-inside text-gray-600 dark:text-gray-400 space-y-1">
        <li>{{ t('help.desc.toolchainsInstall') }}</li>
        <li>{{ t('help.desc.toolchainsDefault') }}</li>
        <li>{{ t('help.desc.toolchainsUninstall') }}</li>
      </ul>
    </section>

    <section class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.components') }}</h2>
      <p class="text-gray-600 dark:text-gray-400 break-words overflow-hidden" v-html="t('help.desc.components', { rustfmt: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded\'>rustfmt</code>', clippy: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded\'>clippy</code>', miri: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded\'>miri</code>' })" />
    </section>

    <section class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.targets') }}</h2>
      <p class="text-gray-600 dark:text-gray-400">
        {{ t('help.desc.targets') }}
      </p>
    </section>

    <section class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.directoryOverrides') }}</h2>
      <p class="text-gray-600 dark:text-gray-400 break-words overflow-hidden" v-html="t('help.desc.directoryOverrides', { command: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded\'>rustup override set &lt;toolchain&gt;</code>' })" />
    </section>

    <section class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.updates') }}</h2>
      <p class="text-gray-600 dark:text-gray-400">
        {{ t('help.desc.updates') }}
      </p>
    </section>

    <section class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.cargoPlugins') }}</h2>
      <p class="text-gray-600 dark:text-gray-400 break-words overflow-hidden" v-html="t('help.desc.cargoPlugins', { audit: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded\'>cargo-audit</code>', expand: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded\'>cargo-expand</code>' })" />
    </section>

    <section class="space-y-3">
      <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">{{ t('help.section.about') }}</h2>
      <p class="text-gray-600 dark:text-gray-400">{{ aboutText }}</p>
    </section>
  </div>
</template>

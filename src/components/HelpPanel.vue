<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-shell'
import { useStore } from '../store'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const { t } = useI18n()
const store = useStore()

const PROJECT_URL = 'https://github.com/RyenLee/rust-verse'
const HOMEPAGE_URL = 'https://ryenlee.github.io/rust-verse/'

const aboutText = computed(() => {
  const name = store.appName || 'RustVerse'
  const version = store.appVersion || '0.0.0'
  return t('help.desc.about', { name, version })
})

function openProjectUrl() {
  open(PROJECT_URL)
}

function openHomepage() {
  open(HOMEPAGE_URL)
}

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
          <div
            class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700 shrink-0"
          >
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
            <!-- Overview -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.overview') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.overview') }}</p>
            </section>

            <!-- Getting Started -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.gettingStarted') }}
              </h3>
              <p
                class="text-sm text-gray-600 dark:text-gray-400 break-words overflow-hidden"
                v-html="
                  t('help.desc.gettingStarted', {
                    rustup: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>rustup</code>',
                  })
                "
              />
            </section>

            <!-- Dashboard -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.dashboard') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.dashboard') }}</p>
            </section>

            <!-- Toolchains -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.toolchains') }}
              </h3>
              <ul class="list-disc list-inside text-sm text-gray-600 dark:text-gray-400 space-y-1">
                <li>{{ t('help.desc.toolchainsInstall') }}</li>
                <li>{{ t('help.desc.toolchainsDefault') }}</li>
                <li>{{ t('help.desc.toolchainsUninstall') }}</li>
              </ul>
            </section>

            <!-- Components -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.components') }}
              </h3>
              <p
                class="text-sm text-gray-600 dark:text-gray-400 break-words overflow-hidden"
                v-html="
                  t('help.desc.components', {
                    rustfmt: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>rustfmt</code>',
                    clippy: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>clippy</code>',
                    miri: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>miri</code>',
                  })
                "
              />
            </section>

            <!-- Targets -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.targets') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.targets') }}</p>
            </section>

            <!-- Directory Overrides -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.overrides') }}
              </h3>
              <p
                class="text-sm text-gray-600 dark:text-gray-400 break-words overflow-hidden"
                v-html="
                  t('help.desc.overrides', {
                    command:
                      '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>rustup override set &lt;toolchain&gt;</code>',
                  })
                "
              />
            </section>

            <!-- Updates -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.updates') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.updates') }}</p>
            </section>

            <!-- Cargo Plugins -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.plugins') }}
              </h3>
              <p
                class="text-sm text-gray-600 dark:text-gray-400 break-words overflow-hidden"
                v-html="
                  t('help.desc.plugins', {
                    audit: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>cargo-audit</code>',
                    expand: '<code class=\'bg-gray-100 dark:bg-gray-800 px-1 rounded text-xs\'>cargo-expand</code>',
                  })
                "
              />
            </section>

            <!-- Environment Variables -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.envVars') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.envVars') }}</p>
            </section>

            <!-- Mirrors -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.mirrors') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.mirrors') }}</p>
            </section>

            <!-- Notifications -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.notifications') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.notifications') }}</p>
            </section>

            <!-- Settings -->
            <section class="space-y-2">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.settings') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ t('help.desc.settings') }}</p>
            </section>

            <!-- About & Project URL -->
            <section class="space-y-3">
              <h3 class="section-title text-sm font-semibold text-gray-800 dark:text-gray-200">
                {{ t('help.section.about') }}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{{ aboutText }}</p>
              <a
                class="flex items-center gap-2 px-3 py-2.5 rounded-lg bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-sm text-gray-600 dark:text-gray-400 hover:text-indigo-600 dark:hover:text-indigo-400 hover:border-indigo-300 dark:hover:border-indigo-700 transition-colors cursor-pointer"
                @click.prevent="openProjectUrl"
              >
                <iconify-icon icon="mdi:github" width="18" class="shrink-0"></iconify-icon>
                <span>{{ t('help.desc.projectUrl') }}</span>
                <iconify-icon icon="mdi:open-in-new" width="14" class="ml-auto shrink-0"></iconify-icon>
              </a>
              <a
                class="flex items-center gap-2 px-3 py-2.5 rounded-lg bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-sm text-gray-600 dark:text-gray-400 hover:text-indigo-600 dark:hover:text-indigo-400 hover:border-indigo-300 dark:hover:border-indigo-700 transition-colors cursor-pointer"
                @click.prevent="openHomepage"
              >
                <iconify-icon icon="mdi:web" width="18" class="shrink-0"></iconify-icon>
                <span>{{ t('help.desc.homepage') }}</span>
                <iconify-icon icon="mdi:open-in-new" width="14" class="ml-auto shrink-0"></iconify-icon>
              </a>
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

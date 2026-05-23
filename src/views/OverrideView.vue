<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import EmptyState from '../components/EmptyState.vue'
import { useRustup, type OverrideInfo } from '../composables/useRustup'
import { useToolchainOptions } from '../composables/useToolchainOptions'

const { t } = useI18n()
const { listOverrides, setOverride: doSet, removeOverride: doRemove } = useRustup()
const { toolchains } = useToolchainOptions()

const overrides = ref<OverrideInfo[]>([])
const loading = ref(true)
const dirPath = ref('')
const selectedToolchain = ref('')
const message = ref('')

async function browseDir() {
  const selected = await open({ directory: true, multiple: false })
  if (selected && typeof selected === 'string') {
    dirPath.value = selected
  }
}

async function refresh() {
  loading.value = true
  try {
    overrides.value = await listOverrides()
  } catch (e) {
    console.error('Failed to load overrides:', e)
  } finally {
    loading.value = false
  }
}

async function setOverride() {
  if (!dirPath.value || !selectedToolchain.value) return
  try {
    await doSet(dirPath.value, selectedToolchain.value)
    message.value = t('overrides.message.setSuccess', { dir: dirPath.value, toolchain: selectedToolchain.value })
    dirPath.value = ''
    await refresh()
  } catch (e) {
    message.value = t('overrides.message.setError', { error: String(e) })
  }
}

async function removeOverride(path: string) {
  try {
    await doRemove(path)
    message.value = t('overrides.message.removeSuccess', { path })
    await refresh()
  } catch (e) {
    message.value = t('overrides.message.removeError', { error: String(e) })
  }
}

onMounted(refresh)
</script>

<template>
  <div class="p-6 space-y-4 h-full overflow-y-auto relative">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ t('overrides.title') }}</h1>

    <!-- Add override -->
    <div class="bg-white dark:bg-gray-800 rounded-lg p-4 border border-gray-200 dark:border-gray-700 space-y-3">
      <h2 class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ t('overrides.action.addOverride') }}</h2>
      <div class="flex flex-wrap gap-3 items-center">
        <input
          v-model="dirPath"
          :placeholder="t('overrides.placeholder.dirPath')"
          class="flex-1 min-w-[200px] bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-200 rounded-md px-3 py-2 border border-gray-200 dark:border-gray-600 h-10"
        />
        <BaseButton variant="secondary" @click="browseDir" class="h-10">
          {{ t('common.action.browse') }}
        </BaseButton>
        <select
          v-model="selectedToolchain"
          class="bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-200 rounded-md px-3 py-2 border border-gray-200 dark:border-gray-600 h-10"
        >
          <option value="" disabled>{{ t('overrides.placeholder.selectToolchain') }}</option>
          <option v-for="tc in toolchains" :key="tc.name" :value="tc.name">
            {{ tc.name }}
          </option>
        </select>
        <BaseButton :disabled="!dirPath || !selectedToolchain" @click="setOverride" class="h-10">
          {{ t('common.action.set') }}
        </BaseButton>
      </div>
      <p v-if="message" class="text-sm text-gray-500 dark:text-gray-400">{{ message }}</p>
    </div>

    <!-- Override list -->
    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('common.status.loading') }}</div>
    <div v-else class="flex flex-wrap gap-3">
      <div
        v-for="ov in overrides"
        :key="ov.path"
        class="bg-white dark:bg-gray-800 rounded-lg px-4 py-3 border border-gray-200 dark:border-gray-700 flex items-center gap-3 min-w-[280px] flex-1"
      >
        <div class="min-w-0 flex-1">
          <p class="text-gray-800 dark:text-gray-200 font-mono text-sm truncate">{{ ov.path }}</p>
          <p class="text-gray-500 dark:text-gray-400 text-sm mt-0.5">{{ ov.toolchain }}</p>
        </div>
        <button
          class="shrink-0 text-xs bg-red-100 hover:bg-red-200 text-red-700 dark:bg-red-900 dark:hover:bg-red-800 dark:text-red-300 px-3 py-1.5 rounded transition-colors"
          @click="removeOverride(ov.path)"
        >
          {{ t('common.action.remove') }}
        </button>
      </div>
      <EmptyState v-if="overrides.length === 0" :message="t('overrides.status.noOverrides')" />
    </div>

    <!-- No-toolchain overlay -->
    <div
      v-if="toolchains.length === 0"
      class="absolute inset-0 bg-white/80 dark:bg-gray-950/80 z-10 flex items-center justify-center"
    >
      <div class="flex flex-col items-center gap-4 text-center">
        <iconify-icon icon="mdi:cog-off-outline" width="40" class="text-gray-400 dark:text-gray-500"></iconify-icon>
        <p class="text-gray-500 dark:text-gray-400 text-sm">{{ t('toolchains.status.installFirst') }}</p>
        <router-link
          to="/toolchains"
          class="inline-flex items-center gap-2 px-4 py-2 bg-sky-600 hover:bg-sky-500 text-white text-sm font-medium rounded-lg transition-colors"
        >
          <iconify-icon icon="mdi:arrow-right" width="16"></iconify-icon>
          {{ t('toolchains.status.goInstall') }}
        </router-link>
      </div>
    </div>
  </div>
</template>

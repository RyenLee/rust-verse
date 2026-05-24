<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useRustup, type ComponentInfo } from '../composables/useRustup'
import { useToolchainOptions } from '../composables/useToolchainOptions'

const { t } = useI18n()
const { listComponents, addComponent, removeComponent } = useRustup()
const { toolchains, refresh: refreshToolchains } = useToolchainOptions()

const selectedToolchain = ref('')
const components = ref<ComponentInfo[]>([])
const loading = ref(false)
const loaded = ref(false)
const search = ref('')

// Auto-select default toolchain when shared list changes
async function ensureSelection() {
  if (toolchains.value.length > 0 && !selectedToolchain.value) {
    const def = toolchains.value.find((t) => t.is_default)
    selectedToolchain.value = def ? def.name : toolchains.value[0].name
  }
}

// Progress dialog state
const showProgress = ref(false)
const progressStatus = ref<'running' | 'success' | 'error'>('running')
const progressTitle = ref('')
const progressStatusText = ref('')
const progressLogs = ref<string[]>([])

async function loadComponents() {
  if (!selectedToolchain.value) return
  loading.value = true
  try {
    components.value = await listComponents(selectedToolchain.value)
    loaded.value = true
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
}

async function toggleComponent(comp: ComponentInfo) {
  const isInstall = !comp.installed
  const action = isInstall ? t('components.action.installing') : t('components.action.removing')
  const compName = comp.name

  progressTitle.value = isInstall ? t('components.progress.installTitle') : t('components.progress.removeTitle')
  progressStatusText.value = t('components.progress.running', { action, name: compName })
  progressStatus.value = 'running'
  progressLogs.value = [t('components.progress.log', { action, name: compName, toolchain: selectedToolchain.value })]
  showProgress.value = true

  try {
    if (comp.installed) {
      await removeComponent(selectedToolchain.value, comp.name)
    } else {
      await addComponent(selectedToolchain.value, comp.name)
    }
    progressStatus.value = 'success'
    progressStatusText.value = t('components.progress.success', { name: compName })
    progressLogs.value.push(t('common.status.done'))
    await loadComponents()
  } catch (e) {
    progressStatus.value = 'error'
    progressStatusText.value = t('components.progress.failed', { name: compName })
    progressLogs.value.push(`Error: ${e}`)
  }
}

function closeProgress() {
  showProgress.value = false
}

const filteredComponents = computed(() => {
  const list = search.value
    ? components.value.filter((c) => c.name.toLowerCase().includes(search.value.toLowerCase()))
    : components.value
  return [...list].sort((a, b) => (b.installed ? 1 : 0) - (a.installed ? 1 : 0))
})
</script>

<template>
  <div class="p-6 space-y-4 flex flex-col h-full relative">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ t('components.title') }}</h1>
      <BaseButton variant="secondary" :loading="loading" :disabled="!selectedToolchain" @click="loadComponents">
        <iconify-icon icon="mdi:download" width="16"></iconify-icon>
        {{ t('common.action.load') }}
      </BaseButton>
    </div>

    <div class="flex gap-3 items-center">
      <select
        v-model="selectedToolchain"
        class="bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-200 rounded-md px-3 py-2 border border-gray-200 dark:border-gray-600"
        @change="loaded = false"
      >
        <option disabled value="">{{ t('components.placeholder.toolchainPlaceholder') }}</option>
        <option v-for="tc in toolchains" :key="tc.name" :value="tc.name">
          {{ tc.name }}
        </option>
      </select>
      <div class="flex items-center flex-1 min-w-0 h-[38px] bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-md focus-within:ring-2 focus-within:ring-sky-500 focus-within:border-transparent">
        <iconify-icon
          icon="mdi:magnify"
          width="18"
          class="shrink-0 ml-3 text-gray-400"
        ></iconify-icon>
        <input
          v-model="search"
          :placeholder="t('components.placeholder.search')"
          class="w-full h-full px-2 bg-transparent text-sm text-gray-900 dark:text-gray-200 placeholder-gray-400 focus:outline-none"
        />
      </div>
    </div>

    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('common.status.loading') }}</div>
    <div v-else-if="!loaded" class="text-gray-400 dark:text-gray-500 text-sm py-8 text-center">
      {{ t('components.status.selectPrompt') }}
    </div>

    <div v-else class="flex-1 overflow-y-auto min-h-0 space-y-1">
      <div
        v-for="comp in filteredComponents"
        :key="comp.name"
        class="bg-white dark:bg-gray-800 rounded-lg px-4 py-3 border border-gray-200 dark:border-gray-700 flex items-center justify-between"
      >
        <div class="flex items-center gap-3">
          <span
            class="w-2 h-2 rounded-full"
            :class="comp.installed ? 'bg-green-500 dark:bg-green-400' : 'bg-gray-300 dark:bg-gray-600'"
          />
          <span class="text-gray-800 dark:text-gray-200">{{ comp.name }}</span>
        </div>
        <button
          :class="[
            'text-xs px-3 py-1.5 rounded transition-colors',
            comp.installed
              ? 'bg-red-100 hover:bg-red-200 text-red-700 dark:bg-red-900 dark:hover:bg-red-800 dark:text-red-300'
              : 'bg-green-100 hover:bg-green-200 text-green-700 dark:bg-green-900 dark:hover:bg-green-800 dark:text-green-300',
          ]"
          @click="toggleComponent(comp)"
        >
          {{ comp.installed ? t('common.action.remove') : t('common.action.install') }}
        </button>
      </div>
    </div>

    <!-- Progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="progressTitle"
      :status="progressStatus"
      :status-text="progressStatusText"
      :lines="progressLogs"
      @close="closeProgress"
    />

    <!-- No-toolchain overlay: covers content area but NOT sidebar -->
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

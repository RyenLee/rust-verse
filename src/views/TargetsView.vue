<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useRustup, type TargetInfo } from '../composables/useRustup'
import { useToolchainOptions } from '../composables/useToolchainOptions'

const { t } = useI18n()
const { listTargets, addTarget, removeTarget } = useRustup()
const { toolchains, refresh: refreshToolchains } = useToolchainOptions()

const selectedToolchain = ref('')
const targets = ref<TargetInfo[]>([])
const loading = ref(false)
const loaded = ref(false)
const search = ref('')

// Auto-select default toolchain when shared list changes
async function ensureSelection() {
  if (toolchains.value.length > 0 && !selectedToolchain.value) {
    const def = toolchains.value.find(t => t.is_default)
    selectedToolchain.value = def ? def.name : toolchains.value[0].name
  }
}

// Progress dialog state
const showProgress = ref(false)
const progressStatus = ref<'running' | 'success' | 'error'>('running')
const progressTitle = ref('')
const progressStatusText = ref('')
const progressLogs = ref<string[]>([])

async function loadTargets() {
  if (!selectedToolchain.value) return
  loading.value = true
  try {
    targets.value = await listTargets(selectedToolchain.value)
    loaded.value = true
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
}

async function toggleTarget(target: TargetInfo) {
  const isInstall = !target.installed
  const action = isInstall ? t('targets.action.installing') : t('targets.action.removing')
  const targetName = target.name

  progressTitle.value = isInstall ? t('targets.progress.installTitle') : t('targets.progress.removeTitle')
  progressStatusText.value = t('targets.progress.running', { action, name: targetName })
  progressStatus.value = 'running'
  progressLogs.value = [t('targets.progress.log', { action, name: targetName, toolchain: selectedToolchain.value })]
  showProgress.value = true

  try {
    if (target.installed) {
      await removeTarget(selectedToolchain.value, target.name)
    } else {
      await addTarget(selectedToolchain.value, target.name)
    }
    progressStatus.value = 'success'
    progressStatusText.value = t('targets.progress.success', { name: targetName })
    progressLogs.value.push(t('common.status.done'))
    await loadTargets()
  } catch (e) {
    progressStatus.value = 'error'
    progressStatusText.value = t('targets.progress.failed', { name: targetName })
    progressLogs.value.push(`Error: ${e}`)
  }
}

function closeProgress() {
  showProgress.value = false
}

const filteredTargets = computed(() => {
  const list = search.value
    ? targets.value.filter(t => t.name.toLowerCase().includes(search.value.toLowerCase()))
    : targets.value
  return [...list].sort((a, b) => (b.installed ? 1 : 0) - (a.installed ? 1 : 0))
})
</script>

<template>
  <div class="p-6 space-y-4 flex flex-col h-full relative">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ t('targets.title') }}</h1>
      <BaseButton variant="secondary" :loading="loading" :disabled="!selectedToolchain" @click="loadTargets">
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
        <option disabled value="">{{ t('targets.placeholder.toolchainPlaceholder') }}</option>
        <option v-for="tc in toolchains" :key="tc.name" :value="tc.name">
          {{ tc.name }}
        </option>
      </select>
      <input
        v-model="search"
        :placeholder="t('targets.placeholder.search')"
        class="bg-gray-100 dark:bg-gray-700 text-gray-900 dark:text-gray-200 rounded-md px-3 py-2 border border-gray-200 dark:border-gray-600 flex-1"
      />
    </div>

    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('common.status.loading') }}</div>
    <div v-else-if="!loaded" class="text-gray-400 dark:text-gray-500 text-sm py-8 text-center">
      {{ t('targets.status.selectPrompt') }}
    </div>

    <div v-else class="flex-1 overflow-y-auto min-h-0 space-y-1">
      <div
        v-for="target in filteredTargets"
        :key="target.name"
        class="bg-white dark:bg-gray-800 rounded-lg px-4 py-3 border border-gray-200 dark:border-gray-700 flex items-center justify-between"
      >
        <div class="flex items-center gap-3">
          <span
            class="w-2 h-2 rounded-full"
            :class="target.installed ? 'bg-green-500 dark:bg-green-400' : 'bg-gray-300 dark:bg-gray-600'"
          />
          <span class="text-gray-800 dark:text-gray-200">{{ target.name }}</span>
        </div>
        <button
          :class="[
            'text-xs px-3 py-1.5 rounded transition-colors',
            target.installed
              ? 'bg-red-100 hover:bg-red-200 text-red-700 dark:bg-red-900 dark:hover:bg-red-800 dark:text-red-300'
              : 'bg-green-100 hover:bg-green-200 text-green-700 dark:bg-green-900 dark:hover:bg-green-800 dark:text-green-300',
          ]"
          @click="toggleTarget(target)"
        >
          {{ target.installed ? t('common.action.remove') : t('common.action.install') }}
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

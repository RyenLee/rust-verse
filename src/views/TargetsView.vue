<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import ListItem from '../components/ListItem.vue'
import PageLayout from '../components/PageLayout.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import SearchInput from '../components/SearchInput.vue'
import SectionTitle from '../components/SectionTitle.vue'
import StatusBadge from '../components/StatusBadge.vue'
import ToolchainSelector from '../components/ToolchainSelector.vue'
import { useResponsiveListHeight } from '../composables/useResponsiveListHeight'
import { useRustup, type TargetInfo } from '../composables/useRustup'
import { useToolchainOptions } from '../composables/useToolchainOptions'

const { t } = useI18n()
const { listTargets, addTarget, removeTarget } = useRustup()
const { toolchains } = useToolchainOptions()

const selectedToolchain = ref('')
const targets = ref<TargetInfo[]>([])
const loading = ref(false)
const loaded = ref(false)
const searchQuery = ref('')

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

function onToolchainChange() {
  loaded.value = false
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
    progressLogs.value.push(`Error: ${e?.message || String(e)}`)
  }
}

function closeProgress() {
  showProgress.value = false
}

const installedTargets = computed(() => {
  const list = searchQuery.value
    ? targets.value.filter((t) => t.installed && t.name.toLowerCase().includes(searchQuery.value.toLowerCase()))
    : targets.value.filter((t) => t.installed)
  return list
})

const availableTargets = computed(() => {
  const list = searchQuery.value
    ? targets.value.filter((t) => !t.installed && t.name.toLowerCase().includes(searchQuery.value.toLowerCase()))
    : targets.value.filter((t) => !t.installed)
  return list
})

// Responsive list height: nav(56) + pageHeader(56) + filters(60) + aboveList(60) + buffer(80)
const { listHeight, listContainerRef } = useResponsiveListHeight({
  filters: 60,
  aboveList: 60,
  buffer: 80,
})
</script>

<template>
  <PageLayout :group="t('nav.group.toolchain')" :title="t('targets.title')" :description="t('targets.description')">
    <template #actions>
      <BaseButton variant="secondary" :loading="loading" :disabled="!selectedToolchain" @click="loadTargets">
        <iconify-icon icon="mdi:download" width="16"></iconify-icon>
        {{ t('common.action.load') }}
      </BaseButton>
    </template>

    <template #filters>
      <ToolchainSelector v-model="selectedToolchain" :toolchains="toolchains" @change="onToolchainChange" />
      <SearchInput v-model="searchQuery" :placeholder="t('common.action.search')" class="flex-1" />
    </template>

    <!-- No toolchain prompt -->
    <div v-if="toolchains.length === 0" class="flex flex-col items-center justify-center py-20">
      <iconify-icon icon="mdi:cog-off-outline" width="40" class="text-gray-400 dark:text-gray-500"></iconify-icon>
      <p class="text-gray-500 dark:text-gray-400 text-sm mt-4">{{ t('toolchains.status.installFirst') }}</p>
      <router-link
        to="/toolchains"
        class="inline-flex items-center gap-2 px-4 py-2 mt-4 bg-sky-600 hover:bg-sky-500 text-white text-sm font-medium rounded-lg transition-colors"
      >
        <iconify-icon icon="mdi:arrow-right" width="16"></iconify-icon>
        {{ t('toolchains.status.goInstall') }}
      </router-link>
    </div>

    <template v-else>
      <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('common.status.loading') }}</div>
      <div v-else-if="!loaded" class="text-gray-400 dark:text-gray-500 text-sm py-8 text-center">
        {{ t('targets.status.selectPrompt') }}
      </div>

      <div v-else class="space-y-6">
        <div v-if="installedTargets.length > 0">
          <SectionTitle title="已安装" :count="installedTargets.length" />
          <div
            ref="listContainerRef"
            class="overflow-y-auto scroll-container space-y-2 rounded-lg"
            :style="{ maxHeight: listHeight }"
          >
            <ListItem
              v-for="target in installedTargets"
              :key="target.name"
              :title="target.name"
              :active="true"
            >
              <template #badges>
                <StatusBadge type="installed" label="已安装" />
              </template>
              <template #actions>
                <button
                  class="text-xs bg-red-100 hover:bg-red-200 text-red-700 dark:bg-red-900 dark:hover:bg-red-800 dark:text-red-300 px-3 py-1.5 rounded transition-colors"
                  @click="toggleTarget(target)"
                >
                  {{ t('common.action.remove') }}
                </button>
              </template>
            </ListItem>
          </div>
        </div>

        <div v-if="availableTargets.length > 0">
          <SectionTitle title="可安装" :count="availableTargets.length" />
          <div class="space-y-2">
            <ListItem
              v-for="target in availableTargets"
              :key="target.name"
              :title="target.name"
              :active="false"
            >
              <template #actions>
                <button
                  class="text-xs bg-green-100 hover:bg-green-200 text-green-700 dark:bg-green-900 dark:hover:bg-green-800 dark:text-green-300 px-3 py-1.5 rounded transition-colors"
                  @click="toggleTarget(target)"
                >
                  {{ t('common.action.install') }}
                </button>
              </template>
            </ListItem>
          </div>
        </div>
      </div>
    </template>

    <!-- Progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="progressTitle"
      :status="progressStatus"
      :status-text="progressStatusText"
      :lines="progressLogs"
      @close="closeProgress"
    />
  </PageLayout>
</template>

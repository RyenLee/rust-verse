<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import EmptyState from '../components/EmptyState.vue'
import PageLayout from '../components/PageLayout.vue'
import SectionTitle from '../components/SectionTitle.vue'
import ListItem from '../components/ListItem.vue'
import StatusBadge from '../components/StatusBadge.vue'
import SearchInput from '../components/SearchInput.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useRustup, type CargoPluginInfo, type SearchResult } from '../composables/useRustup'
import { useResponsiveListHeight } from '../composables/useResponsiveListHeight'
import { useBackgroundTask } from '../composables/useBackgroundTask'

const { t } = useI18n()
const {
  listCargoPlugins,
  searchPlugins: doSearch,
  installPlugin: doInstall,
  uninstallPlugin: doUninstall,
  updatePlugin: doUpdate,
  checkPluginUpdates: doCheckUpdates,
} = useRustup()

const bgTask = useBackgroundTask()
const plugins = ref<CargoPluginInfo[]>([])
const loading = ref(true)
const installing = ref(false)
const installLogs = ref<string[]>([])
const installStatus = ref<'running' | 'success' | 'error'>('running')
const installTarget = ref('')
const showProgress = ref(false)
const confirmUninstall = ref<string | null>(null)
const checkingUpdates = ref(false)

// Search state - unified search box
const searchQuery = ref('')
const searchResults = ref<SearchResult[]>([])
const searching = ref(false)
const searchError = ref('')

// Filter for installed plugins
const pluginFilter = ref('')

const filteredPlugins = computed(() => {
  if (!pluginFilter.value) return plugins.value
  const q = pluginFilter.value.toLowerCase()
  return plugins.value.filter(p => p.name.toLowerCase().includes(q) || p.crate_name.toLowerCase().includes(q))
})

// Responsive list height: filters(56) + search results(~100) + SectionTitle(~30)
const { listHeight } = useResponsiveListHeight({ filters: 56, aboveList: 130 })

async function refresh() {
  loading.value = true
  try {
    plugins.value = await listCargoPlugins()
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
  // Clear search results if search box is empty
  if (!searchQuery.value.trim()) {
    searchResults.value = []
    searchError.value = ''
  }
}

async function checkUpdates() {
  if (plugins.value.length === 0) return
  checkingUpdates.value = true
  try {
    const pluginVersions: Array<[string, string]> = plugins.value.map(p => [p.crate_name, p.version])
    const results = await doCheckUpdates(pluginVersions)
    for (const [crateName, hasUpdate] of results) {
      const plugin = plugins.value.find(p => p.crate_name === crateName)
      if (plugin) {
        plugin.update_available = hasUpdate
      }
    }
  } catch {
    // ignore
  } finally {
    checkingUpdates.value = false
  }
}

async function handleSearch() {
  const q = searchQuery.value.trim()
  if (!q) return
  searching.value = true
  searchResults.value = []
  searchError.value = ''
  try {
    searchResults.value = await doSearch(q)
  } catch (e: any) {
    const msg = e?.message || String(e)
    searchError.value =
      msg.includes('500') || msg.includes('Internal Server Error') ? t('plugins.status.serverError') : msg
  } finally {
    searching.value = false
  }
}

function handleSearchEnter() {
  const q = searchQuery.value.trim()
  if (!q) return
  // If no search results yet, try to install directly
  if (searchResults.value.length === 0) {
    installCrateByName(q)
  } else {
    handleSearch()
  }
}

function installFromSearch(crateName: string) {
  installCrateByName(crateName)
}

async function installCrateByName(crateName: string) {
  if (!(await bgTask.guardStart())) {
    return
  }
  installing.value = true
  installLogs.value = []
  installStatus.value = 'running'
  installTarget.value = crateName
  showProgress.value = true
  bgTask.startTask(t('plugins.progress.title'))
  try {
    await doInstall(crateName)
    installStatus.value = 'success'
    bgTask.finishTask('completed')
    searchQuery.value = ''
    await refresh()
  } catch (e: any) {
    installStatus.value = 'error'
    const msg = e?.message || String(e)
    installLogs.value.push(`Error: ${msg}`)
    bgTask.finishTask('failed')
  } finally {
    installing.value = false
  }
}

async function uninstallPlugin(crateName: string) {
  try {
    await doUninstall(crateName)
    confirmUninstall.value = null
    await refresh()
  } catch {
    // ignore
  }
}

function closeProgress() {
  showProgress.value = false
}

async function cancelPluginOp() {
  await bgTask.requestCancel()
  installStatus.value = 'error'
  installLogs.value.push('操作已取消')
}

function minimizePluginOp() {
  bgTask.minimize(
    () => { showProgress.value = false },
    () => { showProgress.value = true }
  )
}

onMounted(async () => {
  await refresh()

  await listen<string>('plugin-install-log', event => {
    installLogs.value.push(event.payload)
  })
  await listen('plugin-install-finished', () => {
    installing.value = false
  })
})
</script>

<template>
  <PageLayout :group="t('nav.group.extend')" :title="t('plugins.title')" :description="t('plugins.description')">
    <template #actions>
      <BaseButton variant="secondary" :loading="loading" @click="refresh">
        <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
        {{ t('common.action.refresh') }}
      </BaseButton>
    </template>

    <template #filters>
      <div class="flex-1 max-w-md">
        <SearchInput
          v-model="searchQuery"
          :placeholder="t('plugins.placeholder.search')"
          @keyup.enter="handleSearchEnter"
        />
      </div>
      <BaseButton variant="secondary" :loading="searching" :disabled="!searchQuery.trim()" @click="handleSearch">
        {{ t('common.action.search') }}
      </BaseButton>
    </template>

    <!-- Search results -->
    <div v-if="searchResults.length > 0 || searching || searchError" class="mb-6">
      <SectionTitle :title="t('plugins.section.searchCrates')" :count="searchResults.length || undefined" />
      <div v-if="searching" class="text-sm text-gray-400 py-2">{{ t('common.status.searching') }}</div>
      <p v-else-if="searchError" class="text-sm text-red-500 py-2">{{ searchError }}</p>
      <div v-else class="space-y-2 max-h-[360px] overflow-y-auto pr-1 scroll-container">
        <ListItem
          v-for="r in searchResults"
          :key="r.name"
          :title="`${r.name} v${r.version}`"
          :description="r.description"
        >
          <template #actions>
            <button
              :disabled="installing"
              class="bg-sky-600 hover:bg-sky-500 disabled:opacity-50 text-white px-3 py-1.5 rounded text-xs font-medium transition-colors"
              @click="installFromSearch(r.name)"
            >
              {{ t('common.action.install') }}
            </button>
          </template>
        </ListItem>
      </div>
    </div>

    <!-- Installed plugins -->
    <div>
      <div class="flex items-center gap-3 mb-4">
        <SectionTitle :title="t('plugins.section.installedPlugins')" :count="plugins.length" />
        <div class="flex-1 max-w-xs">
          <SearchInput
            v-model="pluginFilter"
            :placeholder="t('plugins.placeholder.filter')"
          />
        </div>
        <BaseButton variant="secondary" :loading="checkingUpdates" @click="checkUpdates">
          <iconify-icon icon="mdi:refresh" width="14"></iconify-icon>
          {{ t('common.action.checkUpdates') }}
        </BaseButton>
      </div>

      <div v-if="loading" class="flex items-center justify-center py-16">
        <div class="text-center space-y-3">
          <div class="inline-flex items-center justify-center w-12 h-12 rounded-full bg-sky-50 dark:bg-sky-900/30">
            <svg
              class="animate-spin h-6 w-6 text-sky-600 dark:text-sky-400"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
          </div>
          <p class="text-gray-500 dark:text-gray-400 text-sm">{{ t('common.status.loading') }}</p>
        </div>
      </div>

      <div v-else class="space-y-3 max-h-[300px] overflow-y-auto pr-2 scroll-container">
          <ListItem
            v-for="p in filteredPlugins"
            :key="p.crate_name"
            :title="`${p.name} v${p.version}`"
            :description="p.crate_name"
          >
            <template #badges>
              <StatusBadge v-if="p.is_official" type="default" :label="t('plugins.badge.official')" />
              <StatusBadge v-if="p.update_available" type="success" :label="t('plugins.badge.updateAvailable')" />
            </template>
            <template #actions>
              <button
                v-if="p.update_available"
                class="text-xs bg-amber-100 hover:bg-amber-200 text-amber-700 dark:bg-amber-900 dark:hover:bg-amber-800 dark:text-amber-300 px-3 py-1.5 rounded transition-colors mr-2"
                @click="installCrateByName(p.crate_name)"
              >
                {{ t('common.action.update') }}
              </button>
              <button
                class="text-xs bg-red-100 hover:bg-red-200 text-red-700 dark:bg-red-900 dark:hover:bg-red-800 dark:text-red-300 px-3 py-1.5 rounded transition-colors"
                @click="confirmUninstall = p.crate_name"
              >
                {{ t('common.action.uninstall') }}
              </button>
            </template>
          </ListItem>
        </div>

        <EmptyState
          v-if="filteredPlugins.length === 0 && plugins.length > 0"
          :message="t('plugins.status.noFilterMatch')"
        />
        <EmptyState v-else-if="plugins.length === 0" :message="t('plugins.status.noPlugins')" />
    </div>

    <!-- Uninstall confirm -->
    <ConfirmDialog
      v-if="confirmUninstall"
      :title="t('plugins.dialog.confirmUninstall')"
      :message="t('plugins.dialog.uninstallConfirm', { name: confirmUninstall })"
      :confirm-label="t('common.action.uninstall')"
      :danger="true"
      @confirm="uninstallPlugin(confirmUninstall!)"
      @cancel="confirmUninstall = null"
    />

    <!-- Install progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="t('plugins.progress.title')"
      :status="installStatus"
      :status-text="
        installStatus === 'running'
          ? t('plugins.progress.running', { name: installTarget })
          : installStatus === 'success'
          ? t('plugins.progress.success', { name: installTarget })
          : t('plugins.progress.failed', { name: installTarget })
      "
      :lines="installLogs"
      @close="closeProgress"
      @cancel="cancelPluginOp"
      @minimize="minimizePluginOp"
    />
  </PageLayout>
</template>

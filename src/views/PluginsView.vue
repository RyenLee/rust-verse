<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import EmptyState from '../components/EmptyState.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useRustup, type CargoPluginInfo, type SearchResult } from '../composables/useRustup'

const { t } = useI18n()
const {
  listCargoPlugins,
  searchPlugins: doSearch,
  installPlugin: doInstall,
  uninstallPlugin: doUninstall,
} = useRustup()

const plugins = ref<CargoPluginInfo[]>([])
const loading = ref(true)
const installing = ref(false)
const installLogs = ref<string[]>([])
const installStatus = ref<'running' | 'success' | 'error'>('running')
const installTarget = ref('')
const showProgress = ref(false)
const installCrateName = ref('')
const confirmUninstall = ref<string | null>(null)

// Search state
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

async function installFromSearch(crateName: string) {
  installCrateName.value = crateName
  await installPlugin()
}

async function installPlugin() {
  if (!installCrateName.value) return
  installing.value = true
  installLogs.value = []
  installStatus.value = 'running'
  installTarget.value = installCrateName.value
  showProgress.value = true
  try {
    await doInstall(installCrateName.value)
    installStatus.value = 'success'
    installCrateName.value = ''
    await refresh()
  } catch (e: any) {
    installStatus.value = 'error'
    const msg = e?.message || String(e)
    installLogs.value.push(`Error: ${msg}`)
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
  <div class="h-full flex flex-col overflow-hidden">
    <!-- Fixed header area -->
    <div class="shrink-0 px-6 lg:px-8 pt-6 lg:pt-8 pb-4 w-full space-y-4">
      <!-- Title row -->
      <div class="flex items-center justify-between">
        <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ t('plugins.title') }}</h1>
        <BaseButton variant="secondary" :loading="loading" @click="refresh">
          <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
          {{ t('common.action.refresh') }}
        </BaseButton>
      </div>

      <!-- Search on crates.io -->
      <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 space-y-3">
        <h2 class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ t('plugins.section.searchCrates') }}</h2>
        <div class="flex items-center gap-2 flex-nowrap">
          <div class="relative flex-1 min-w-0">
            <iconify-icon
              icon="mdi:magnify"
              width="18"
              class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
            ></iconify-icon>
            <input
              v-model="searchQuery"
              :placeholder="t('plugins.placeholder.search')"
              class="w-full h-9 box-border pl-10 pr-3 bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-600 rounded-lg text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500 focus:border-transparent"
              @keyup.enter="handleSearch"
            />
          </div>
          <BaseButton variant="secondary" :loading="searching" :disabled="!searchQuery.trim()" @click="handleSearch">
            {{ t('common.action.search') }}
          </BaseButton>
        </div>
        <div v-if="searchResults.length > 0" class="space-y-2 max-h-60 overflow-y-auto">
          <div
            v-for="r in searchResults"
            :key="r.name"
            class="flex items-center justify-between bg-gray-50 dark:bg-gray-900 rounded-lg p-3"
          >
            <div class="min-w-0 flex-1">
              <p class="text-sm font-medium text-gray-900 dark:text-gray-200">
                {{ r.name }} <span class="text-gray-400 font-normal">v{{ r.version }}</span>
              </p>
              <p class="text-xs text-gray-500 dark:text-gray-400 truncate">{{ r.description }}</p>
            </div>
            <button
              :disabled="installing"
              class="ml-3 shrink-0 bg-sky-600 hover:bg-sky-500 disabled:opacity-50 text-white px-3 py-1.5 rounded text-xs font-medium transition-colors"
              @click="installFromSearch(r.name)"
            >
              {{ t('common.action.install') }}
            </button>
          </div>
        </div>
        <p v-else-if="searching" class="text-sm text-gray-400">{{ t('common.status.searching') }}</p>
        <p v-else-if="searchError" class="text-sm text-red-500">{{ searchError }}</p>
      </div>

      <!-- Install by name -->
      <div class="flex items-center gap-2 flex-nowrap">
        <div class="relative flex-1 min-w-0">
          <iconify-icon
            icon="mdi:download"
            width="18"
            class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
          ></iconify-icon>
          <input
            v-model="installCrateName"
            :placeholder="t('plugins.placeholder.installByName')"
            class="w-full h-9 box-border pl-10 pr-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500 focus:border-transparent"
            @keyup.enter="installPlugin"
          />
        </div>
        <BaseButton :loading="installing" :disabled="!installCrateName" @click="installPlugin">
          {{ t('common.action.install') }}
        </BaseButton>
      </div>

      <!-- Installed plugins header with filter -->
      <div class="flex items-center gap-2 flex-nowrap">
        <h2 class="text-sm font-medium text-gray-700 dark:text-gray-300 shrink-0">
          {{ t('plugins.section.installedPlugins') }}
        </h2>
        <div class="relative flex-1 max-w-xs">
          <iconify-icon
            icon="mdi:filter-outline"
            width="16"
            class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
          ></iconify-icon>
          <input
            v-model="pluginFilter"
            :placeholder="t('plugins.placeholder.filter')"
            class="w-full h-9 box-border pl-10 pr-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-sky-500 focus:border-transparent"
          />
        </div>
      </div>
    </div>

    <!-- Scrollable content area -->
    <div class="flex-1 min-h-0 overflow-y-auto px-6 lg:px-8 pb-6 lg:pb-8">
      <div class="space-y-2">
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

        <template v-else>
          <div
            v-for="p in filteredPlugins"
            :key="p.crate_name"
            class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 flex items-center justify-between hover:shadow-sm transition-shadow"
          >
            <div>
              <p class="text-gray-900 dark:text-gray-200">
                {{ p.name }}
                <span
                  v-if="p.is_official"
                  class="text-xs bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 px-1.5 py-0.5 rounded ml-1"
                >
                  {{ t('plugins.badge.official') }}
                </span>
              </p>
              <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">{{ p.crate_name }} v{{ p.version }}</p>
            </div>
            <button
              class="text-xs bg-red-100 hover:bg-red-200 text-red-700 dark:bg-red-900 dark:hover:bg-red-800 dark:text-red-300 px-3 py-1.5 rounded transition-colors"
              @click="confirmUninstall = p.crate_name"
            >
              {{ t('common.action.uninstall') }}
            </button>
          </div>

          <EmptyState
            v-if="filteredPlugins.length === 0 && plugins.length > 0"
            :message="t('plugins.status.noFilterMatch')"
          />
          <EmptyState v-else-if="plugins.length === 0" :message="t('plugins.status.noPlugins')" />
        </template>
      </div>
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
    />
  </div>
</template>

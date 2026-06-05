<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import BaseButton from '../components/BaseButton.vue'
import DateRangePicker from '../components/DateRangePicker.vue'
import EmptyState from '../components/EmptyState.vue'
import ListItem from '../components/ListItem.vue'
import PageLayout from '../components/PageLayout.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import SearchInput from '../components/SearchInput.vue'
import SectionTitle from '../components/SectionTitle.vue'
import StatusBadge from '../components/StatusBadge.vue'
import { useHistoryVersions } from '../composables/useHistoryVersions'
import { useRustup } from '../composables/useRustup'
import { useDataRefresh } from '../composables/useDataRefresh'
import { useToolchainOptions } from '../composables/useToolchainOptions'
import { useResponsiveListHeight } from '../composables/useResponsiveListHeight'
import { useBackgroundTask } from '../composables/useBackgroundTask'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const { installToolchain } = useRustup()
const { notifyToolchainChange } = useDataRefresh()
const { toolchains } = useToolchainOptions()
const { releases, loading, loadingMore, hasMore, totalCount, syncing, syncError, syncFromManifests, refresh, loadMore, search, searchLoadMore } = useHistoryVersions()
const bgTask = useBackgroundTask()

const selectedChannel = ref('stable')
const searchQuery = ref('')
const syncingChannel = ref<string | null>(null)
const dateRange = ref<{ start: string | null; end: string | null }>({ start: null, end: null })

// Whether user came from toolchains page (enables "select & go back" mode)
const selectMode = computed(() => route.query.from === 'toolchains')

// Initialize selected channel from route query
watch(
  () => route.query.channel,
  ch => {
    if (ch && ['stable', 'beta', 'nightly'].includes(ch as string)) {
      selectedChannel.value = ch as string
    }
  },
  { immediate: true }
)

// Install progress
const installing = ref(false)
const installLogs = ref<string[]>([])
const installStatus = ref<'running' | 'success' | 'error'>('running')
const showProgress = ref(false)
const installingRelease = ref<{ version: string; date: string; channel: string } | null>(null)

const { listHeight } = useResponsiveListHeight({ filters: 48 })

const channelOptions = computed(() => [
  { value: 'stable', label: t('histver.channel.stable'), icon: 'mdi:shield-check' },
  { value: 'beta', label: t('histver.channel.beta'), icon: 'mdi:flask' },
  { value: 'nightly', label: t('histver.channel.nightly'), icon: 'mdi:weather-night' },
])

const filteredReleases = computed(() => {
  let result = releases.value
  const query = searchQuery.value.toLowerCase().trim()
  if (query) {
    result = result.filter(r => r.version.toLowerCase().includes(query) || r.date.includes(query))
  }
  if (dateRange.value.start) {
    result = result.filter(r => r.date >= dateRange.value.start!)
  }
  if (dateRange.value.end) {
    result = result.filter(r => r.date <= dateRange.value.end!)
  }
  return result
})

// Group releases by channel for display
const groupedReleases = computed(() => {
  const groups: Record<string, typeof releases.value> = {}
  for (const r of filteredReleases.value) {
    if (!groups[r.channel]) groups[r.channel] = []
    groups[r.channel].push(r)
  }
  return groups
})

// Check if a version is already installed
const installedVersions = computed(() => {
  const versionSet = new Set<string>()
  const dateSet = new Set<string>()
  for (const tc of toolchains.value) {
    const displayMatch = tc.display_name.match(
      /^(?:stable|beta|nightly)?-?(\d+\.\d+\.\d+(?:-[a-z]+\.\d+)?)/
    )
    if (displayMatch) versionSet.add(displayMatch[1])
    const versionMatch = tc.name.match(/^(\d+\.\d+\.\d+(?:-[a-z]+\.\d+)?)/)
    if (versionMatch) versionSet.add(versionMatch[1])
    const nightlyMatch = tc.name.match(/^nightly-(\d{4}-\d{2}-\d{2})/)
    if (nightlyMatch) dateSet.add(nightlyMatch[1])
    const dateMatch = tc.name.match(/^(?:stable|beta)-(\d{4}-\d{2}-\d{2})/)
    if (dateMatch) dateSet.add(dateMatch[1])
  }
  return { versionSet, dateSet }
})

function isInstalled(release: { version: string; date: string; channel: string }): boolean {
  const cleanVer = release.version.split(' ')[0]
  if (installedVersions.value.versionSet.has(cleanVer)) {
    return true
  }
  if (installedVersions.value.versionSet.has(release.version)) {
    return true
  }
  if (installedVersions.value.dateSet.has(release.date)) {
    return true
  }
  return false
}

async function syncReleases() {
  syncingChannel.value = selectedChannel.value
  try {
    await syncFromManifests()
  } catch {
    // error is captured in syncError ref
  } finally {
    syncingChannel.value = null
  }
}

async function searchReleases() {
  const query = searchQuery.value.trim()
  if (!query) {
    await refresh(selectedChannel.value)
    return
  }
  await search(query, selectedChannel.value)
}

async function installRelease(release: { version: string; date: string; channel: string }) {
  if (!(await bgTask.guardStart())) {
    return
  }
  installing.value = true
  installLogs.value = []
  installStatus.value = 'running'
  installingRelease.value = release
  showProgress.value = true
  bgTask.startTask(t('histver.progress.title'))

  try {
    if (release.channel === 'nightly') {
      await installToolchain('nightly', undefined, release.date)
    } else {
      await installToolchain(release.channel, release.version, release.date)
    }
    installStatus.value = 'success'
    bgTask.finishTask('completed')
    notifyToolchainChange()
  } catch (e) {
    installStatus.value = 'error'
    installLogs.value.push(`Error: ${e?.message || String(e)}`)
    bgTask.finishTask('failed')
  } finally {
    installing.value = false
  }
}

/** Select a version and navigate back to toolchains page with pre-fill params */
function selectAndGoBack(release: { version: string; date: string; channel: string }) {
  router.push({
    path: '/toolchains',
    query: { channel: release.channel },
  })
}

async function handleLoadMore() {
  const query = searchQuery.value.trim()
  if (query) {
    await searchLoadMore(query, selectedChannel.value)
  } else {
    await loadMore(selectedChannel.value)
  }
}

function goBackToToolchains() {
  router.push('/toolchains')
}

function closeProgress() {
  showProgress.value = false
  installingRelease.value = null
}

async function cancelInstallOp() {
  await bgTask.requestCancel()
  installStatus.value = 'error'
  installLogs.value.push('操作已取消')
}

function minimizeInstallOp() {
  bgTask.minimize(
    () => { showProgress.value = false },
    () => { showProgress.value = true }
  )
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr + 'T00:00:00')
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' })
  } catch {
    return dateStr
  }
}

let unlistenLog: (() => void) | null = null
let unlistenFinish: (() => void) | null = null

onMounted(async () => {
  await refresh(selectedChannel.value)
  unlistenLog = await listen<string>('install-log', event => {
    installLogs.value.push(event.payload)
  })
  unlistenFinish = await listen('install-finished', () => {
    installing.value = false
  })
})

onBeforeUnmount(() => {
  unlistenLog?.()
  unlistenFinish?.()
})

watch(selectedChannel, async () => {
  searchQuery.value = ''
  await refresh(selectedChannel.value)
})

// Debounced search on query change
let searchTimer: ReturnType<typeof setTimeout> | null = null
watch(searchQuery, () => {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => searchReleases(), 300)
})
</script>

<template>
  <PageLayout
    :group="t('nav.group.toolchain')"
    :title="t('histver.title')"
    :description="selectMode ? t('histver.descriptionSelect') : t('histver.description')"
  >
    <template #actions>
      <BaseButton variant="ghost" @click="goBackToToolchains">
        <iconify-icon icon="mdi:arrow-left" width="16"></iconify-icon>
        {{ t('histver.action.backToToolchains') }}
      </BaseButton>
      <BaseButton :loading="syncing" @click="syncReleases">
        <iconify-icon icon="mdi:sync" width="16"></iconify-icon>
        {{ syncing ? t('histver.action.syncing') : t('histver.action.sync') }}
      </BaseButton>
    </template>

    <template #filters>
      <!-- Channel tabs -->
      <div class="flex bg-gray-100 dark:bg-gray-800 rounded-lg p-0.5 gap-0.5 shrink-0">
        <button
          v-for="opt in channelOptions"
          :key="opt.value"
          :class="[
            'px-3 py-1.5 text-xs font-medium rounded-md transition-colors',
            selectedChannel === opt.value
              ? 'bg-white dark:bg-gray-700 text-sky-600 dark:text-sky-400 shadow-sm'
              : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200',
          ]"
          @click="selectedChannel = opt.value"
        >
          <iconify-icon :icon="opt.icon" width="14" class="mr-1"></iconify-icon>
          {{ opt.label }}
        </button>
      </div>
      <!-- Date range -->
      <DateRangePicker v-model="dateRange" :placeholder="t('histver.filter.dateRange')" class="w-64 shrink-0" />
      <!-- Search -->
      <SearchInput v-model="searchQuery" :placeholder="t('histver.action.searchPlaceholder')" class="flex-1 min-w-0" />
    </template>

    <!-- Sync error -->
    <div
      v-if="syncError"
      class="mb-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg"
    >
      <div class="flex items-start gap-2">
        <iconify-icon
          icon="mdi:alert-circle"
          width="18"
          class="mt-0.5 text-red-500 dark:text-red-400 shrink-0"
        ></iconify-icon>
        <div class="min-w-0">
          <p class="text-sm font-medium text-red-700 dark:text-red-300">{{ syncError }}</p>
          <p class="mt-1 text-xs text-red-500 dark:text-red-400">{{ t('histver.error.syncHint') }}</p>
        </div>
        <button
          class="ml-auto shrink-0 text-red-400 hover:text-red-600 dark:hover:text-red-300 transition-colors"
          @click="syncError = null"
        >
          <iconify-icon icon="mdi:close" width="16"></iconify-icon>
        </button>
      </div>
    </div>

    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('common.status.loading') }}</div>

    <div v-else :style="{ maxHeight: listHeight }" class="overflow-y-auto scroll-container space-y-6 rounded-lg">
      <!-- Grouped by channel -->
      <div v-for="(groupReleases, channel) in groupedReleases" :key="channel">
        <SectionTitle
          :title="
            channel === 'stable'
              ? t('histver.channel.stable')
              : channel === 'beta'
              ? t('histver.channel.beta')
              : t('histver.channel.nightly')
          "
          :count="totalCount"
        />
        <div class="space-y-2">
          <ListItem
            v-for="rel in groupReleases"
            :key="`${rel.version}-${rel.date}`"
            :title="rel.version"
            :description="formatDate(rel.date)"
            :active="isInstalled(rel)"
          >
            <template #badges>
              <StatusBadge type="installed" :label="formatDate(rel.date)" />
              <StatusBadge v-if="isInstalled(rel)" type="default" :label="t('histver.status.installed')" />
            </template>
            <template #actions>
              <button
                v-if="selectMode && !isInstalled(rel)"
                :disabled="installing"
                class="text-xs bg-sky-100 hover:bg-sky-200 text-sky-700 dark:bg-sky-900 dark:hover:bg-sky-800 dark:text-sky-300 px-3 py-1.5 rounded transition-colors disabled:opacity-50"
                @click="selectAndGoBack(rel)"
              >
                <iconify-icon icon="mdi:check" width="12" class="mr-0.5"></iconify-icon>
                {{ t('histver.action.select') }}
              </button>
              <button
                v-else-if="!isInstalled(rel)"
                :disabled="installing"
                class="text-xs bg-green-100 hover:bg-green-200 text-green-700 dark:bg-green-900 dark:hover:bg-green-800 dark:text-green-300 px-3 py-1.5 rounded transition-colors disabled:opacity-50"
                @click="installRelease(rel)"
              >
                {{ t('common.action.install') }}
              </button>
              <span v-else class="text-xs text-gray-400 dark:text-gray-500">
                <iconify-icon icon="mdi:check-circle" width="14" class="text-green-500"></iconify-icon>
              </span>
            </template>
          </ListItem>
        </div>
      </div>

      <EmptyState v-if="filteredReleases.length === 0 && releases.length > 0" :message="t('common.status.noMatch')" />
      <EmptyState v-if="releases.length === 0" :message="t('histver.status.noData')" />

      <!-- Load more -->
      <div v-if="hasMore && releases.length > 0" class="flex justify-center pb-4">
        <BaseButton
          variant="ghost"
          :loading="loadingMore"
          :disabled="loadingMore"
          @click="handleLoadMore"
        >
          <iconify-icon icon="mdi:chevron-down" width="16" class="mr-1"></iconify-icon>
          <span v-if="loadingMore">{{ t('histver.action.loadingMore') }}</span>
          <span v-else>{{ t('histver.action.loadMore') }} ({{ releases.length }} / {{ totalCount }})</span>
        </BaseButton>
      </div>
      <div v-if="!hasMore && releases.length > 0" class="flex justify-center pb-4">
        <span class="text-xs text-gray-400 dark:text-gray-500">
          {{ t('histver.status.allLoaded', { count: totalCount }) }}
        </span>
      </div>
    </div>

    <!-- Install progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="t('histver.progress.title')"
      :status="installStatus"
      :status-text="
        installStatus === 'running'
          ? t('histver.progress.running', { version: installingRelease?.version || '' })
          : installStatus === 'success'
          ? t('histver.progress.success', { version: installingRelease?.version || '' })
          : t('histver.progress.failed')
      "
      :lines="installLogs"
      @close="closeProgress"
      @cancel="cancelInstallOp"
      @minimize="minimizeInstallOp"
    />
  </PageLayout>
</template>

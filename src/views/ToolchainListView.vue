<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, onBeforeUnmount, onActivated, onDeactivated, ref, computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import BaseButton from '../components/BaseButton.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import EmptyState from '../components/EmptyState.vue'
import PageLayout from '../components/PageLayout.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import SearchInput from '../components/SearchInput.vue'
import StatusBadge from '../components/StatusBadge.vue'
import { useRustup } from '../composables/useRustup'
import { useDataRefresh } from '../composables/useDataRefresh'
import { useToolchainOptions } from '../composables/useToolchainOptions'
import { useBackgroundTask } from '../composables/useBackgroundTask'
import { useToast } from '../composables/useToast'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const toast = useToast()
const {
  installToolchain: doInstall,
  uninstallToolchain: doUninstall,
  setDefaultToolchain: doSetDefault,
  checkUpdate: doCheck,
  updateAll: doUpdateAll,
  updateRustup: doUpdateRustup,
} = useRustup()
const { notifyToolchainChange, onToolchainChange } = useDataRefresh()
const { toolchains, loading, refresh } = useToolchainOptions()
const bgTask = useBackgroundTask()
const installing = ref(false)
const installLogs = ref<string[]>([])
const installStatus = ref<'running' | 'success' | 'error'>('running')
const showInstallPanel = ref(false)
const showProgress = ref(false)
const newChannel = ref('stable')
const confirmUninstall = ref<string | null>(null)
const uninstalling = ref(false)
const searchQuery = ref('')
const updating = ref(false)
const updatingRustup = ref(false)
const updateLogs = ref<string[]>([])
const updateStatus = ref<'running' | 'success' | 'error'>('running')
const updateMode = ref<'all' | 'rustup'>('all')
const showUpdateProgress = ref(false)
const showUpdateDropdown = ref(false)
const updateDropdownRef = ref<HTMLElement | null>(null)

const filteredToolchains = computed(() => {
  const query = searchQuery.value.toLowerCase().trim()
  if (!query) return toolchains.value
  return toolchains.value.filter(tc => tc.name.toLowerCase().includes(query))
})

const channelOptions = computed(() => [
  { value: 'stable', label: t('toolchains.channel.stable'), desc: t('toolchains.channel.stableDesc') },
  { value: 'beta', label: t('toolchains.channel.beta'), desc: t('toolchains.channel.latestVersion') },
  { value: 'nightly', label: t('toolchains.channel.nightly'), desc: t('toolchains.channel.latestVersion') },
])

// Watch route query for channel pre-fill from history versions page
watch(
  () => route.query,
  query => {
    const channel = query.channel as string
    if (channel && ['stable', 'beta', 'nightly'].includes(channel)) {
      newChannel.value = channel
      showInstallPanel.value = true
      router.replace({ path: '/toolchains', query: {} })
    }
  },
  { immediate: true }
)

async function installToolchain() {
  if (!(await bgTask.guardStart())) {
    return
  }
  bgTask.startTask(t('toolchains.progress.title'))
  installing.value = true
  installLogs.value = []
  installStatus.value = 'running'
  showProgress.value = true
  showInstallPanel.value = false
  try {
    await doInstall(newChannel.value)
    installStatus.value = 'success'
    bgTask.finishTask('completed')
    notifyToolchainChange()
    await refresh()
  } catch (e: any) {
    installStatus.value = 'error'
    installLogs.value.push(`Error: ${e?.message || e?.toString?.() || String(e)}`)
    bgTask.finishTask('failed')
  } finally {
    installing.value = false
  }
}

async function uninstallToolchain(name: string) {
  uninstalling.value = true
  try {
    await doUninstall(name)
    confirmUninstall.value = null
    notifyToolchainChange()
    await refresh()
    toast.success(t('toolchains.dialog.uninstallSuccess', { name }))
  } catch (e: any) {
    toast.error(e?.message || e?.toString?.() || String(e))
  } finally {
    uninstalling.value = false
  }
}

async function setDefault(name: string) {
  try {
    await doSetDefault(name)
    notifyToolchainChange()
    await refresh()
  } catch {
    // ignore
  }
}

function openInstallPanel() {
  newChannel.value = 'stable'
  showInstallPanel.value = true
}

function closeProgress() {
  // Don't call reset() here — the background task manages its own lifecycle.
  // finishTask auto-resets after 3s display period.
  // If the user hides the dialog, the overlay (if minimized) stays visible.
  showProgress.value = false
}

function cancelInstall() {
  bgTask.requestCancel()
  installStatus.value = 'error'
  installLogs.value.push(t('toolchains.progress.cancelled'))
  toast.info(t('toolchains.progress.cancelled'))
  if (!showProgress.value) {
    showProgress.value = true
  }
}

function minimizeInstall() {
  bgTask.minimize(
    () => {
      showProgress.value = false
    },
    () => {
      showProgress.value = true
    }
  )
}

async function updateAll() {
  if (!(await bgTask.guardStart())) {
    return
  }
  updating.value = true
  updateLogs.value = []
  updateStatus.value = 'running'
  updateMode.value = 'all'
  showUpdateProgress.value = true
  bgTask.startTask(t('updates.progress.updatingAllTitle'))
  try {
    await doUpdateAll()
    updateStatus.value = 'success'
    bgTask.finishTask('completed')
    notifyToolchainChange()
    await refresh()
  } catch (e: any) {
    updateStatus.value = 'error'
    updateLogs.value.push(`Error: ${e?.message || String(e)}`)
    bgTask.finishTask('failed')
  } finally {
    updating.value = false
  }
}

async function updateRustup() {
  if (!(await bgTask.guardStart())) {
    return
  }
  updatingRustup.value = true
  updateLogs.value = []
  updateStatus.value = 'running'
  updateMode.value = 'rustup'
  showUpdateProgress.value = true
  bgTask.startTask(t('updates.progress.updatingRustupTitle'))
  try {
    await doUpdateRustup()
    updateStatus.value = 'success'
    bgTask.finishTask('completed')
    notifyToolchainChange()
    await refresh()
  } catch (e: any) {
    updateStatus.value = 'error'
    updateLogs.value.push(`Error: ${e?.message || String(e)}`)
    bgTask.finishTask('failed')
  } finally {
    updatingRustup.value = false
  }
}

function closeUpdateProgress() {
  showUpdateProgress.value = false
}

async function cancelUpdateOp() {
  await bgTask.requestCancel()
  updateStatus.value = 'error'
  updateLogs.value.push('操作已取消')
}

function minimizeUpdateOp() {
  bgTask.minimize(
    () => {
      showUpdateProgress.value = false
    },
    () => {
      showUpdateProgress.value = true
    }
  )
}

function goToHistoryVersions() {
  router.push('/history-versions')
}

function handleClickOutside(e: MouseEvent) {
  if (updateDropdownRef.value && !updateDropdownRef.value.contains(e.target as Node)) {
    showUpdateDropdown.value = false
  }
}

function handleUpdateAction(action: 'all' | 'rustup') {
  showUpdateDropdown.value = false
  if (action === 'all') {
    updateAll()
  } else {
    updateRustup()
  }
}

let unlistenLog: (() => void) | null = null
let unlistenFinish: (() => void) | null = null
let unlistenUpdateLog: (() => void) | null = null

// Refresh when any page installs/uninstalls a toolchain (keep-alive safe)
onToolchainChange(() => refresh())

onMounted(async () => {
  document.addEventListener('click', handleClickOutside, true)
  unlistenLog = await listen<string>('install-log', event => {
    installLogs.value.push(event.payload)
    bgTask.appendLine(event.payload)
  })
  unlistenFinish = await listen('install-finished', () => {
    installing.value = false
  })
  unlistenUpdateLog = await listen<string>('update-log', event => {
    updateLogs.value.push(event.payload)
  })
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside, true)
  unlistenLog?.()
  unlistenFinish?.()
  unlistenUpdateLog?.()
})

let progressWasVisible = false
let updateProgressWasVisible = false

onDeactivated(() => {
  progressWasVisible = showProgress.value
  updateProgressWasVisible = showUpdateProgress.value
  showInstallPanel.value = false
  showProgress.value = false
  showUpdateProgress.value = false
})

onActivated(() => {
  if (progressWasVisible || (bgTask.state.status === 'running' && !bgTask.state.minimized)) {
    showProgress.value = true
    progressWasVisible = false
  }
  if (updateProgressWasVisible) {
    showUpdateProgress.value = true
    updateProgressWasVisible = false
  }
})
</script>

<template>
  <PageLayout
    :group="t('nav.group.toolchain')"
    :title="t('toolchains.title')"
    :description="t('toolchains.description')"
  >
    <template #actions>
      <BaseButton variant="ghost" :loading="loading" @click="refresh">
        <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
        {{ t('common.action.refresh') }}
      </BaseButton>
      <BaseButton variant="ghost" @click="goToHistoryVersions">
        <iconify-icon icon="mdi:history" width="16"></iconify-icon>
        {{ t('toolchains.action.historyVersions') }}
      </BaseButton>
      <BaseButton @click="openInstallPanel">
        {{ t('toolchains.action.installNew') }}
      </BaseButton>

      <div class="relative" ref="updateDropdownRef">
        <BaseButton
          variant="secondary"
          :loading="updating || updatingRustup"
          @click="showUpdateDropdown = !showUpdateDropdown"
        >
          <iconify-icon icon="mdi:update" width="16"></iconify-icon>
          {{ t('updates.action.update') }}
          <iconify-icon :icon="showUpdateDropdown ? 'mdi:chevron-up' : 'mdi:chevron-down'" width="16"></iconify-icon>
        </BaseButton>
        <Transition name="dropdown">
          <div
            v-if="showUpdateDropdown"
            class="absolute right-0 top-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg overflow-hidden z-50 min-w-[180px]"
          >
            <button
              class="flex items-center gap-2.5 w-full px-4 py-2.5 text-sm transition-colors text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700/50 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="updating || updatingRustup"
              @click="handleUpdateAction('rustup')"
            >
              <iconify-icon
                icon="mdi:cloud-download-outline"
                width="16"
                class="text-gray-400 dark:text-gray-500"
              ></iconify-icon>
              <span class="flex-1 text-left">{{ t('updates.action.updateRustup') }}</span>
            </button>
            <button
              class="flex items-center gap-2.5 w-full px-4 py-2.5 text-sm transition-colors text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700/50 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="updating || updatingRustup"
              @click="handleUpdateAction('all')"
            >
              <iconify-icon icon="mdi:update" width="16" class="text-gray-400 dark:text-gray-500"></iconify-icon>
              <span class="flex-1 text-left">{{ t('updates.action.updateAll') }}</span>
            </button>
          </div>
        </Transition>
      </div>
    </template>

    <template #filters>
      <SearchInput v-model="searchQuery" :placeholder="t('common.action.search')" />
    </template>

    <div v-if="loading" class="text-gray-500 dark:text-gray-400">{{ t('common.status.loading') }}</div>

    <div v-else class="overflow-y-auto scroll-container rounded-lg pr-1">
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="tc in filteredToolchains"
          :key="tc.name"
          class="relative rounded-xl border p-4 transition-all hover:shadow-md"
          :class="[
            tc.is_default
              ? 'border-sky-500/50 bg-sky-50 dark:bg-sky-900/15 ring-1 ring-sky-500/20'
              : tc.is_active
              ? 'border-green-500/40 bg-green-50 dark:bg-green-900/10'
              : 'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:border-gray-300 dark:hover:border-gray-600',
          ]"
        >
          <div class="flex items-start justify-between gap-3 mb-3">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2 mb-1">
                <span
                  class="w-2 h-2 rounded-full shrink-0"
                  :class="tc.is_default ? 'bg-sky-500' : tc.is_active ? 'bg-green-500' : 'bg-gray-300 dark:bg-gray-600'"
                />
                <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate">{{ tc.name }}</h3>
              </div>
              <p class="text-xs text-gray-500 dark:text-gray-400">{{ tc.channel }}</p>
            </div>
            <div class="flex items-center gap-1 shrink-0">
              <StatusBadge v-if="tc.is_default" type="default" :label="t('common.status.default')" />
              <StatusBadge v-if="tc.is_active && !tc.is_default" type="active" :label="t('common.status.active')" />
            </div>
          </div>
          <div class="flex items-center gap-2">
            <button
              v-if="!tc.is_default"
              class="flex-1 text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-300 px-3 py-1.5 rounded-lg transition-colors"
              @click="setDefault(tc.name)"
            >
              {{ t('toolchains.action.setDefault') }}
            </button>
            <button
              v-if="!tc.is_default"
              class="flex-1 text-xs bg-red-50 hover:bg-red-100 text-red-600 dark:bg-red-900/20 dark:hover:bg-red-900/40 dark:text-red-400 px-3 py-1.5 rounded-lg transition-colors"
              @click="confirmUninstall = tc.name"
            >
              {{ t('common.action.uninstall') }}
            </button>
            <div v-if="tc.is_default" class="flex-1 text-xs text-gray-400 dark:text-gray-500 text-center py-1.5">
              {{ t('common.status.default') }}
            </div>
          </div>
        </div>
      </div>

      <EmptyState
        v-if="filteredToolchains.length === 0 && toolchains.length > 0"
        :message="t('common.status.noMatch')"
      />
      <EmptyState v-if="toolchains.length === 0" :message="t('toolchains.status.noToolchains')" />
    </div>

    <!-- Install panel (slide-in from right) -->
    <Teleport to="body">
      <Transition name="slide-panel">
        <div v-if="showInstallPanel" class="fixed inset-0 z-50 flex justify-end">
          <div class="absolute inset-0 bg-black/40" @click="showInstallPanel = false" />
          <div
            class="relative w-full max-w-md bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 shadow-xl flex flex-col"
          >
            <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
              <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                {{ t('toolchains.dialog.installTitle') }}
              </h2>
              <button
                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                @click="showInstallPanel = false"
              >
                <iconify-icon icon="mdi:close" width="20"></iconify-icon>
              </button>
            </div>
            <div class="flex-1 overflow-y-auto p-6 scroll-container">
              <div>
                <label class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2 block">{{
                  t('toolchains.form.channel')
                }}</label>
                <div class="space-y-2">
                  <label
                    v-for="opt in channelOptions"
                    :key="opt.value"
                    :class="[
                      'flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors',
                      newChannel === opt.value
                        ? 'border-sky-500 bg-sky-50 dark:bg-sky-900/20'
                        : 'border-gray-200 dark:border-gray-600 hover:border-gray-300 dark:hover:border-gray-500',
                    ]"
                  >
                    <input v-model="newChannel" type="radio" :value="opt.value" class="mt-0.5 accent-sky-600" />
                    <div>
                      <p class="text-sm font-medium text-gray-900 dark:text-gray-100">{{ opt.label }}</p>
                      <p class="text-xs text-gray-500 dark:text-gray-400">{{ opt.desc }}</p>
                    </div>
                  </label>
                </div>
              </div>
            </div>
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-2">
              <BaseButton variant="ghost" @click="showInstallPanel = false">{{ t('common.action.cancel') }}</BaseButton>
              <BaseButton :loading="installing" @click="installToolchain">{{
                installing ? t('toolchains.progress.installing') : t('common.action.install')
              }}</BaseButton>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <ConfirmDialog
      v-if="confirmUninstall"
      :title="t('toolchains.dialog.confirmUninstall')"
      :message="t('toolchains.dialog.uninstallConfirm', { name: confirmUninstall })"
      :confirm-label="t('common.action.uninstall')"
      :danger="true"
      :loading="uninstalling"
      @confirm="uninstallToolchain(confirmUninstall!)"
      @cancel="confirmUninstall = null"
    />

    <ProgressDialog
      :visible="showProgress"
      :title="t('toolchains.progress.title')"
      :status="installStatus"
      :status-text="
        installStatus === 'running'
          ? t('toolchains.progress.running', { channel: newChannel })
          : installStatus === 'success'
          ? t('toolchains.progress.success', { channel: newChannel })
          : t('toolchains.progress.failed')
      "
      :lines="installLogs"
      @close="closeProgress"
      @cancel="cancelInstall"
      @minimize="minimizeInstall"
    />

    <ProgressDialog
      :visible="showUpdateProgress"
      :title="updateMode === 'all' ? t('updates.progress.updatingAllTitle') : t('updates.progress.updatingRustupTitle')"
      :status="updateStatus"
      :status-text="
        updateStatus === 'running'
          ? updateMode === 'all'
            ? t('updates.progress.updatingAllStatus')
            : t('updates.progress.updatingRustupStatus')
          : updateStatus === 'success'
          ? updateMode === 'all'
            ? t('updates.progress.allUpdated')
            : t('updates.progress.rustupUpdated')
          : t('updates.progress.failed')
      "
      :lines="updateLogs"
      @close="closeUpdateProgress"
      @cancel="cancelUpdateOp"
      @minimize="minimizeUpdateOp"
    />
  </PageLayout>
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

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>

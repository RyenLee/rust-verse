<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import BaseButton from '../components/BaseButton.vue'
import EmptyState from '../components/EmptyState.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useMirror, type MirrorInfo, type MirrorLatency } from '../composables/useMirror'
import { useToast } from '../composables/useToast'

const route = useRoute()

const { t } = useI18n()
const {
  checkInstalled,
  install: doInstall,
  list: doList,
  current: doCurrent,
  version: doVersion,
  useMirror: doUse,
  best: doBest,
  restoreDefault: doDefault,
  test: doTest,
} = useMirror()
const { success, error } = useToast()

// Page state: 'loading' | 'guide' | 'main'
const pageState = ref<'loading' | 'guide' | 'main'>('loading')

// Data
const mirrors = ref<MirrorInfo[]>([])
const currentMirror = ref('')
const crmVersion = ref('')
const loading = ref(false)

// Latency data: keyed by mirror name
const latencyMap = ref<Record<string, MirrorLatency>>({})

// Install state
const installing = ref(false)
const installLogs = ref<string[]>([])
const installStatus = ref<'running' | 'success' | 'error'>('running')
const showProgress = ref(false)

// Best operation loading states
const bestLoading = ref('')

// Test state
const testingAll = ref(false)
const testingMirror = ref('')

const currentMirrorName = computed(() => {
  if (!currentMirror.value) return t('mirror.status.official')
  return currentMirror.value
})

async function loadData() {
  loading.value = true
  try {
    const [listResult, currentResult, versionResult] = await Promise.all([
      doList(),
      doCurrent().catch(() => ''),
      doVersion().catch(() => ''),
    ])
    mirrors.value = listResult
    // Sync is_current from list result
    currentMirror.value = currentResult || listResult.find(m => m.is_current)?.name || ''
    crmVersion.value = versionResult
  } catch (e: any) {
    error(t('mirror.message.loadFailed', { error: e?.message || String(e) }))
  } finally {
    loading.value = false
  }
}

async function initPage() {
  pageState.value = 'loading'
  try {
    const installed = await checkInstalled()
    if (installed) {
      pageState.value = 'main'
      await loadData()
    } else {
      pageState.value = 'guide'
    }
  } catch {
    pageState.value = 'guide'
  }
}

async function handleEnable() {
  installing.value = true
  installLogs.value = []
  installStatus.value = 'running'
  showProgress.value = true
  try {
    await doInstall()
    installStatus.value = 'success'
    success(t('mirror.message.installSuccess'))
    pageState.value = 'main'
    await loadData()
  } catch (e: any) {
    installStatus.value = 'error'
    const msg = e?.message || String(e)
    installLogs.value.push(`Error: ${msg}`)
    error(t('mirror.message.installFailed', { error: msg }))
  } finally {
    installing.value = false
  }
}

async function handleSwitch(name: string) {
  try {
    await doUse(name)
    currentMirror.value = name
    // Update is_current on all mirrors
    mirrors.value.forEach(m => (m.is_current = m.name === name))
    success(t('mirror.message.switchSuccess', { name }))
  } catch (e: any) {
    error(t('mirror.message.switchFailed', { error: e?.message || String(e) }))
  }
}

async function handleBest(mode: string) {
  bestLoading.value = mode || 'all'
  try {
    await doBest(mode || undefined)
    await loadData()
    success(t('mirror.message.bestSuccess'))
  } catch (e: any) {
    error(t('mirror.message.bestFailed', { error: e?.message || String(e) }))
  } finally {
    bestLoading.value = ''
  }
}

async function handleDefault() {
  bestLoading.value = 'default'
  try {
    await doDefault()
    currentMirror.value = ''
    mirrors.value.forEach(m => (m.is_current = false))
    await loadData()
    success(t('mirror.message.defaultSuccess'))
  } catch (e: any) {
    error(t('mirror.message.defaultFailed', { error: e?.message || String(e) }))
  } finally {
    bestLoading.value = ''
  }
}

async function handleTestAll() {
  testingAll.value = true
  try {
    const result = await doTest()
    applyLatencyResults(result.latencies)
  } catch (e: any) {
    error(t('mirror.message.testFailed', { error: e?.message || String(e) }))
  } finally {
    testingAll.value = false
  }
}

async function handleTestOne(name: string) {
  testingMirror.value = name
  try {
    const result = await doTest(name)
    applyLatencyResults(result.latencies)
  } catch (e: any) {
    error(t('mirror.message.testFailed', { error: e?.message || String(e) }))
  } finally {
    testingMirror.value = ''
  }
}

function applyLatencyResults(latencies: MirrorLatency[]) {
  for (const l of latencies) {
    latencyMap.value[l.name] = l
  }
}

function getLatencyText(name: string): string {
  const l = latencyMap.value[name]
  if (!l) return ''
  const parts: string[] = []
  if (l.network_ms !== null) {
    parts.push(`${l.network_ms}ms`)
  } else if (l.download_ms !== null) {
    // Only download available
  } else {
    // No data yet
    return ''
  }
  if (l.download_ms !== null) {
    parts.push(`${l.download_ms}ms`)
  }
  if (l.network_ms === null && l.download_ms === null) {
    return ''
  }
  // Show network / download
  const net = l.network_ms !== null ? `${l.network_ms}ms` : 'failed'
  const dl = l.download_ms !== null ? `${l.download_ms}ms` : 'failed'
  return `${net} / ${dl}`
}

function getNetworkMs(name: string): number | null {
  return latencyMap.value[name]?.network_ms ?? null
}

function getDownloadMs(name: string): number | null {
  return latencyMap.value[name]?.download_ms ?? null
}

function msColor(ms: number | null, failed: boolean): string {
  if (ms === null && !failed) return 'text-gray-400 dark:text-gray-500'
  if (ms === null && failed) return 'text-red-500 dark:text-red-400'
  if (ms! < 100) return 'text-emerald-600 dark:text-emerald-400'
  if (ms! < 300) return 'text-amber-600 dark:text-amber-400'
  return 'text-red-600 dark:text-red-400'
}

function isFailed(name: string): boolean {
  const l = latencyMap.value[name]
  if (!l) return false
  return l.network_ms === null && l.download_ms === null && l.network_ms !== undefined
}

function closeProgress() {
  showProgress.value = false
}

onMounted(async () => {
  await initPage()

  await listen<string>('plugin-install-log', event => {
    installLogs.value.push(event.payload)
  })
  await listen('plugin-install-finished', () => {
    installing.value = false
  })
})

// Re-initialize when navigating back to this page
watch(() => route.path, async (newPath) => {
  if (newPath === '/mirrors') {
    await initPage()
  }
})
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden">
    <!-- Guide state: crm not installed -->
    <div v-if="pageState === 'guide'" class="flex-1 flex items-center justify-center">
      <div class="text-center max-w-sm space-y-6">
        <div class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-sky-50 dark:bg-sky-900/30">
          <iconify-icon icon="mdi:mirror" width="32" class="text-sky-600 dark:text-sky-400"></iconify-icon>
        </div>
        <div class="space-y-2">
          <h2 class="text-xl font-bold text-gray-900 dark:text-gray-100">
            {{ t('mirror.guide.title') }}
          </h2>
          <p class="text-sm text-gray-500 dark:text-gray-400 leading-relaxed">
            {{ t('mirror.guide.description') }}
          </p>
        </div>
        <div class="space-y-3">
          <BaseButton :loading="installing" @click="handleEnable">
            <iconify-icon icon="mdi:power" width="16"></iconify-icon>
            {{ t('mirror.action.enable') }}
          </BaseButton>
          <p class="text-xs text-gray-400 dark:text-gray-500">
            {{ t('mirror.guide.command') }}
          </p>
        </div>
      </div>
    </div>

    <!-- Main state: crm installed -->
    <template v-else-if="pageState === 'main'">
      <!-- Fixed header area -->
      <div class="shrink-0 px-6 lg:px-8 pt-6 lg:pt-8 pb-4 w-full space-y-4">
        <!-- Title row -->
        <div class="flex items-center justify-between">
          <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{{ t('mirror.title') }}</h1>
          <BaseButton variant="secondary" :loading="loading" @click="loadData">
            <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
            {{ t('mirror.action.refresh') }}
          </BaseButton>
        </div>

        <!-- Status bar -->
        <div
          class="flex items-center gap-4 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 px-4 py-3"
        >
          <div class="flex items-center gap-2">
            <iconify-icon icon="mdi:check-circle" width="16" class="text-emerald-500"></iconify-icon>
            <span class="text-sm text-gray-500 dark:text-gray-400">{{ t('mirror.status.current') }}:</span>
            <span class="text-sm font-medium text-gray-900 dark:text-gray-100">{{ currentMirrorName }}</span>
          </div>
        </div>

        <!-- Quick actions -->
        <div
          class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 px-4 py-3 space-y-2"
        >
          <h2 class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
            {{ t('mirror.action.bestAll') }}
          </h2>
          <div class="flex items-center gap-2 flex-wrap">
            <button
              class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-sky-50 dark:bg-sky-900/30 text-sky-700 dark:text-sky-300 border border-sky-200 dark:border-sky-800 hover:bg-sky-100 dark:hover:bg-sky-900/50 transition-colors disabled:opacity-50"
              :disabled="!!bestLoading"
              @click="handleBest('')"
            >
              <iconify-icon
                :icon="bestLoading === 'all' ? 'mdi:loading' : 'mdi:lightning-bolt'"
                :class="{ 'animate-spin': bestLoading === 'all' }"
                width="14"
              ></iconify-icon>
              {{ t('mirror.action.bestAll') }}
            </button>
            <button
              class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-gray-50 dark:bg-gray-900 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors disabled:opacity-50"
              :disabled="!!bestLoading"
              @click="handleBest('git')"
            >
              <iconify-icon
                :icon="bestLoading === 'git' ? 'mdi:loading' : 'mdi:source-branch'"
                :class="{ 'animate-spin': bestLoading === 'git' }"
                width="14"
              ></iconify-icon>
              {{ t('mirror.action.bestGit') }}
            </button>
            <button
              class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-gray-50 dark:bg-gray-900 text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors disabled:opacity-50"
              :disabled="!!bestLoading"
              @click="handleBest('sparse')"
            >
              <iconify-icon
                :icon="bestLoading === 'sparse' ? 'mdi:loading' : 'mdi:source-merge'"
                :class="{ 'animate-spin': bestLoading === 'sparse' }"
                width="14"
              ></iconify-icon>
              {{ t('mirror.action.bestSparse') }}
            </button>
            <button
              class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-emerald-50 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800 hover:bg-emerald-100 dark:hover:bg-emerald-900/50 transition-colors disabled:opacity-50"
              :disabled="!!bestLoading"
              @click="handleBest('git-download')"
            >
              <iconify-icon
                :icon="bestLoading === 'git-download' ? 'mdi:loading' : 'mdi:download'"
                :class="{ 'animate-spin': bestLoading === 'git-download' }"
                width="14"
              ></iconify-icon>
              {{ t('mirror.action.bestGitDownload') }}
            </button>
            <button
              class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-emerald-50 dark:bg-emerald-900/30 text-emerald-700 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800 hover:bg-emerald-100 dark:hover:bg-emerald-900/50 transition-colors disabled:opacity-50"
              :disabled="!!bestLoading"
              @click="handleBest('sparse-download')"
            >
              <iconify-icon
                :icon="bestLoading === 'sparse-download' ? 'mdi:loading' : 'mdi:download'"
                :class="{ 'animate-spin': bestLoading === 'sparse-download' }"
                width="14"
              ></iconify-icon>
              {{ t('mirror.action.bestSparseDownload') }}
            </button>
            <button
              class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-300 border border-red-200 dark:border-red-800 hover:bg-red-100 dark:hover:bg-red-900/50 transition-colors disabled:opacity-50"
              :disabled="!!bestLoading"
              @click="handleDefault"
            >
              <iconify-icon
                :icon="bestLoading === 'default' ? 'mdi:loading' : 'mdi:restore'"
                :class="{ 'animate-spin': bestLoading === 'default' }"
                width="14"
              ></iconify-icon>
              {{ t('mirror.action.restoreDefault') }}
            </button>
            <button
              class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-amber-50 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800 hover:bg-amber-100 dark:hover:bg-amber-900/50 transition-colors disabled:opacity-50"
              :disabled="testingAll"
              @click="handleTestAll"
            >
              <iconify-icon
                :icon="testingAll ? 'mdi:loading' : 'mdi:speedometer'"
                :class="{ 'animate-spin': testingAll }"
                width="14"
              ></iconify-icon>
              {{ t('mirror.action.testAll') }}
            </button>
          </div>
        </div>
      </div>

      <!-- Scrollable content area -->
      <div class="flex-1 min-h-0 overflow-y-auto px-6 lg:px-8 pb-6 lg:pb-8 space-y-4">
        <!-- Mirror list -->
        <div class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
          <div class="overflow-y-auto max-h-full">
            <table class="w-full text-sm table-fixed">
              <thead class="sticky top-0 z-10">
                <tr class="bg-gray-50 dark:bg-gray-800/80">
                  <th
                    class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap max-w-0"
                  >
                    {{ t('mirror.field.name') }}
                  </th>
                  <th class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {{ t('mirror.field.type') }}
                  </th>
                  <th class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {{ t('mirror.field.index') }}
                  </th>
                  <th
                    class="text-left px-4 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap w-[120px]"
                  >
                    {{ t('mirror.field.latency') }}
                  </th>
                  <th
                    class="text-center px-2 py-3 font-semibold text-gray-500 dark:text-gray-400 whitespace-nowrap w-[80px]"
                  >
                    {{ t('mirror.field.actions') }}
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-50 dark:divide-gray-700/50">
                <tr
                  v-for="m in mirrors"
                  :key="m.name"
                  class="hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors"
                  :class="{ 'bg-sky-50/50 dark:bg-sky-900/10': m.is_current }"
                >
                  <!-- Name -->
                  <td class="px-4 py-3 max-w-0">
                    <div class="flex items-center gap-2 min-w-0">
                      <iconify-icon
                        v-if="m.is_current"
                        icon="mdi:check-circle"
                        width="16"
                        class="text-sky-500 shrink-0"
                      ></iconify-icon>
                      <span class="font-medium text-gray-900 dark:text-gray-100 truncate" :title="m.name">
                        {{ m.name }}
                      </span>
                    </div>
                  </td>

                  <!-- Type -->
                <td class="px-4 py-3">
                  <span
                    v-if="m.mirror_type === 'sparse'"
                    class="inline-flex items-center text-xs bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 px-1.5 py-0.5 rounded"
                  >
                    {{ t('mirror.tag.sparse') }}
                  </span>
                  <span
                    v-else-if="m.mirror_type === 'git'"
                    class="inline-flex items-center text-xs bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400 px-1.5 py-0.5 rounded"
                  >
                    {{ t('mirror.tag.git') }}
                  </span>
                  <span
                    v-else
                    class="inline-flex items-center text-xs bg-amber-100 text-amber-600 dark:bg-amber-900 dark:text-amber-300 px-1.5 py-0.5 rounded"
                  >
                    {{ t('mirror.tag.other') }}
                  </span>
                </td>

                  <!-- Index -->
                  <td class="px-4 py-3">
                    <span class="text-xs text-gray-500 dark:text-gray-400 font-mono truncate block" :title="m.index">
                      {{ m.index }}
                    </span>
                  </td>

                  <!-- Latency -->
                  <td class="px-4 py-3 w-[120px]">
                    <div v-if="latencyMap[m.name]" class="space-y-0.5">
                      <div class="flex items-center gap-1 text-xs">
                        <span class="text-gray-400 dark:text-gray-500 shrink-0">Net</span>
                        <span
                          class="font-mono"
                          :class="
                            msColor(
                              getNetworkMs(m.name) ?? null,
                              latencyMap[m.name]?.network_ms === null && latencyMap[m.name] !== undefined
                            )
                          "
                        >
                          {{ getNetworkMs(m.name) !== null ? `${getNetworkMs(m.name)}ms` : 'failed' }}
                        </span>
                      </div>
                      <div class="flex items-center gap-1 text-xs">
                        <span class="text-gray-400 dark:text-gray-500 shrink-0">DL</span>
                        <span
                          class="font-mono"
                          :class="
                            msColor(
                              getDownloadMs(m.name) ?? null,
                              latencyMap[m.name]?.download_ms === null && latencyMap[m.name] !== undefined
                            )
                          "
                        >
                          {{ getDownloadMs(m.name) !== null ? `${getDownloadMs(m.name)}ms` : 'failed' }}
                        </span>
                      </div>
                    </div>
                    <span v-else class="text-xs text-gray-300 dark:text-gray-600">—</span>
                  </td>

                  <!-- Actions -->
                  <td class="px-2 py-3 w-[80px]">
                    <div class="flex items-center justify-center gap-1">
                      <button
                        v-if="m.name !== currentMirror"
                        class="inline-flex items-center justify-center w-7 h-7 rounded-md text-sky-600 dark:text-sky-400 hover:bg-sky-50 dark:hover:bg-sky-900/30 transition-colors"
                        :title="t('mirror.action.switchTo')"
                        @click="handleSwitch(m.name)"
                      >
                        <iconify-icon icon="mdi:swap-horizontal" width="14"></iconify-icon>
                      </button>
                      <button
                        class="inline-flex items-center justify-center w-7 h-7 rounded-md text-amber-600 dark:text-amber-400 hover:bg-amber-50 dark:hover:bg-amber-900/30 transition-colors disabled:opacity-50"
                        :title="t('mirror.action.testMirror')"
                        :disabled="testingAll || testingMirror === m.name"
                        @click="handleTestOne(m.name)"
                      >
                        <iconify-icon
                          :icon="testingMirror === m.name ? 'mdi:loading' : 'mdi:speedometer'"
                          :class="{ 'animate-spin': testingMirror === m.name }"
                          width="14"
                        ></iconify-icon>
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-if="mirrors.length === 0 && !loading" class="py-8">
            <EmptyState :message="t('mirror.status.noMirrors')" />
          </div>
        </div>
      </div>
    </template>

    <!-- Loading state -->
    <div v-else class="flex-1 flex items-center justify-center">
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

    <!-- Install progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="t('mirror.guide.title')"
      :status="installStatus"
      :status-text="
        installStatus === 'running'
          ? t('mirror.status.installing')
          : installStatus === 'success'
          ? t('mirror.message.installSuccess')
          : t('mirror.status.installFailed')
      "
      :lines="installLogs"
      @close="closeProgress"
    />
  </div>
</template>

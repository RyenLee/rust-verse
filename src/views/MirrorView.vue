<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { onMounted, ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import BaseButton from '../components/BaseButton.vue'
import EmptyState from '../components/EmptyState.vue'
import LatencyBar from '../components/LatencyBar.vue'
import ListItem from '../components/ListItem.vue'
import PageLayout from '../components/PageLayout.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import SectionTitle from '../components/SectionTitle.vue'
import StatusBadge from '../components/StatusBadge.vue'
import { useMirror, type MirrorInfo, type MirrorLatency } from '../composables/useMirror'
import { useResponsiveListHeight } from '../composables/useResponsiveListHeight'
import { useToast } from '../composables/useToast'
import { useBackgroundTask } from '../composables/useBackgroundTask'
import { useTerminalReinit } from '../composables/useTerminalReinit'

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
const bgTask = useBackgroundTask()
const { reinitTerminal } = useTerminalReinit()

async function reinitTerminalSilent() {
  try {
    await reinitTerminal()
  } catch {
    // Terminal reinit is best-effort
  }
}

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

// Responsive list height: subtract all fixed elements above the mirror list
// nav(56) + pageLayoutHeader(56) + currentMirror(70) + quickActions(90) + margins(60) + buffer(80)
const { listHeight, listContainerRef } = useResponsiveListHeight({
  aboveList: 220,
  buffer: 80,
})

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
      doList().catch(() => []),
      doCurrent().catch(() => ''),
      doVersion().catch(() => ''),
    ])
    mirrors.value = listResult
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
  if (!(await bgTask.guardStart())) {
    return
  }
  installing.value = true
  installLogs.value = []
  installStatus.value = 'running'
  showProgress.value = true
  bgTask.startTask(t('mirror.guide.title'))
  try {
    await doInstall()
    installStatus.value = 'success'
    bgTask.finishTask('completed')
    success(t('mirror.message.installSuccess'))
    pageState.value = 'main'
    await loadData()
  } catch (e: any) {
    installStatus.value = 'error'
    const msg = e?.message || String(e)
    installLogs.value.push(`Error: ${msg}`)
    bgTask.finishTask('failed')
    error(t('mirror.message.installFailed', { error: msg }))
  } finally {
    installing.value = false
  }
}

async function handleSwitch(name: string) {
  try {
    await doUse(name)
    currentMirror.value = name
    mirrors.value.forEach(m => (m.is_current = m.name === name))
    reinitTerminalSilent()
    success(t('mirror.message.switchSuccess', { name }))
  } catch (e: any) {
    if (isBinaryNotFoundError(e)) {
      pageState.value = 'guide'
      error('crm 未安装，请先点击引导页安装')
    } else {
      error(t('mirror.message.switchFailed', { error: e?.message || String(e) }))
    }
  }
}

async function handleBest(mode: string) {
  bestLoading.value = mode || 'all'
  try {
    await doBest(mode || undefined)
    await loadData()
    reinitTerminalSilent()
    success(t('mirror.message.bestSuccess'))
  } catch (e: any) {
    if (isBinaryNotFoundError(e)) {
      pageState.value = 'guide'
      error('crm 未安装，请先点击引导页安装')
    } else {
      error(t('mirror.message.bestFailed', { error: e?.message || String(e) }))
    }
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
    reinitTerminalSilent()
    success(t('mirror.message.defaultSuccess'))
  } catch (e: any) {
    if (isBinaryNotFoundError(e)) {
      pageState.value = 'guide'
      error('crm 未安装，请先点击引导页安装')
    } else {
      error(t('mirror.message.defaultFailed', { error: e?.message || String(e) }))
    }
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
    if (isBinaryNotFoundError(e)) {
      pageState.value = 'guide'
      error('crm 未安装，请先点击引导页安装')
    } else {
      error(t('mirror.message.testFailed', { error: e?.message || String(e) }))
    }
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
    if (isBinaryNotFoundError(e)) {
      pageState.value = 'guide'
      error('crm 未安装，请先点击引导页安装')
    } else {
      error(t('mirror.message.testFailed', { error: e?.message || String(e) }))
    }
  } finally {
    testingMirror.value = ''
  }
}

function applyLatencyResults(latencies: MirrorLatency[]) {
  for (const l of latencies) {
    latencyMap.value[l.name] = l
  }
}

function getNetworkMs(name: string): number | null {
  return latencyMap.value[name]?.network_ms ?? null
}

function getDownloadMs(name: string): number | null {
  return latencyMap.value[name]?.download_ms ?? null
}

function isBinaryNotFoundError(e: any): boolean {
  const msg = e?.message || ''
  const kind = e?.kind || ''
  return kind === 'binary_not_found' || msg.includes('not found') || msg.includes('program not found')
}

function closeProgress() {
  showProgress.value = false
}

async function cancelMirrorOp() {
  await bgTask.requestCancel()
  installStatus.value = 'error'
  installLogs.value.push('操作已取消')
}

function minimizeMirrorOp() {
  bgTask.minimize(
    () => {
      showProgress.value = false
    },
    () => {
      showProgress.value = true
    }
  )
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

watch(
  () => route.path,
  async newPath => {
    if (newPath === '/mirrors') {
      await initPage()
    }
  }
)
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
    <PageLayout
      v-else-if="pageState === 'main'"
      :group="t('nav.group.config')"
      :title="t('mirror.title')"
      :description="t('mirror.description')"
    >
      <template #actions>
        <BaseButton variant="secondary" :loading="loading" @click="loadData">
          <iconify-icon icon="mdi:refresh" width="16"></iconify-icon>
          {{ t('mirror.action.refresh') }}
        </BaseButton>
      </template>

      <!-- Current mirror -->
      <SectionTitle :title="t('mirror.status.current')" />
      <div
        class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 px-4 py-3 flex items-center justify-between"
      >
        <div class="flex items-center gap-2">
          <iconify-icon icon="mdi:check-circle" width="16" class="text-emerald-500"></iconify-icon>
          <span class="text-sm font-medium text-gray-900 dark:text-gray-100">{{ currentMirrorName }}</span>
        </div>
        <button
          class="inline-flex items-center gap-1 text-xs text-red-500 dark:text-red-400 hover:text-red-600 dark:hover:text-red-300 transition-colors disabled:opacity-50"
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
      </div>

      <!-- Quick actions -->
      <SectionTitle :title="t('mirror.action.bestAll')" class="mt-6" />
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
          class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-purple-50 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 border border-purple-200 dark:border-purple-800 hover:bg-purple-100 dark:hover:bg-purple-900/50 transition-colors disabled:opacity-50"
          :disabled="!!bestLoading"
          @click="handleBest('sparse')"
        >
          <iconify-icon
            :icon="bestLoading === 'sparse' ? 'mdi:loading' : 'mdi:cube-outline'"
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
          class="inline-flex items-center gap-1.5 h-8 px-3 rounded-lg text-xs font-medium bg-orange-50 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300 border border-orange-200 dark:border-orange-800 hover:bg-orange-100 dark:hover:bg-orange-900/50 transition-colors disabled:opacity-50"
          :disabled="!!bestLoading"
          @click="handleBest('sparse-download')"
        >
          <iconify-icon
            :icon="bestLoading === 'sparse-download' ? 'mdi:loading' : 'mdi:cloud-download'"
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

      <!-- Mirror list -->
      <SectionTitle :title="t('mirror.field.name')" :count="mirrors.length" class="mt-6" />
      <div
        ref="listContainerRef"
        class="overflow-y-auto scroll-container mt-3 space-y-2 rounded-lg"
        :style="{ maxHeight: listHeight }"
      >
        <ListItem
          v-for="m in mirrors"
          :key="m.name"
          :title="m.is_current ? `${m.name} ✅` : m.name"
          :active="m.is_current"
          :description="m.index"
        >
          <template #badges>
            <StatusBadge
              :type="m.mirror_type === 'sparse' ? 'active' : m.mirror_type === 'git' ? 'installed' : 'uninstalled'"
              :label="
                m.mirror_type === 'sparse'
                  ? t('mirror.tag.sparse')
                  : m.mirror_type === 'git'
                  ? t('mirror.tag.git')
                  : t('mirror.tag.other')
              "
            />
          </template>
          <template #actions>
            <div class="flex items-center gap-3">
              <div v-if="latencyMap[m.name]" class="flex items-center gap-3">
                <LatencyBar :value="getNetworkMs(m.name)" label="Net" />
                <LatencyBar :value="getDownloadMs(m.name)" label="DL" />
              </div>
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
          </template>
        </ListItem>

        <EmptyState v-if="mirrors.length === 0 && !loading" :message="t('mirror.status.noMirrors')" />
      </div>
    </PageLayout>

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
      @cancel="cancelMirrorOp"
      @minimize="minimizeMirrorOp"
    />
  </div>
</template>

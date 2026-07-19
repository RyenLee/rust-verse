<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useStore } from '../store'
import BaseButton from '../components/BaseButton.vue'
import PageLayout from '../components/PageLayout.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useAppUpdater } from '../composables/useAppUpdater'
import { useBackgroundTask } from '../composables/useBackgroundTask'

const { t } = useI18n()
const store = useStore()
const bgTask = useBackgroundTask()
const {
  checking: appChecking,
  update: appUpdate,
  downloadPhase,
  downloadedBytes,
  totalBytes,
  downloadError,
  checkError,
  checkForUpdate,
  downloadAndInstall,
  reset: resetAppUpdater,
} = useAppUpdater()

const showProgress = ref(false)
const showRestartButton = ref(false)
const restarting = ref(false)

const progressLines = computed(() => {
  const lines: string[] = []
  const total = totalBytes.value
  const dl = downloadedBytes.value

  const size =
    total != null ? (total > 1048576 ? `${(total / 1048576).toFixed(1)} MB` : `${(total / 1024).toFixed(1)} KB`) : null
  const dlSize = dl > 1048576 ? `${(dl / 1048576).toFixed(1)} MB` : `${(dl / 1024).toFixed(1)} KB`

  if (downloadPhase.value === 'downloading') {
    const pct = total && total > 0 ? Math.round((dl / total) * 100) : 0
    const info = size
      ? `${t('updates.progress.downloadProgress')}: ${dlSize} / ${size} (${pct}%)`
      : `${t('updates.progress.downloadProgress')}: ${dlSize} (${pct}%)`
    lines.push(info)
  }
  if (downloadPhase.value === 'installing') {
    lines.push(t('updates.status.appInstalling'))
  }
  if (downloadPhase.value === 'success') {
    lines.push(t('updates.progress.installComplete'))
  }
  if (downloadPhase.value === 'error' && downloadError.value) {
    lines.push(`${t('updates.progress.downloadFailed')}: ${downloadError.value}`)
  }
  return lines
})

async function handleCheckUpdate() {
  await checkForUpdate()
}

async function handleDownloadAndInstall() {
  if (!(await bgTask.guardStart())) {
    return
  }
  showProgress.value = true
  bgTask.startTask(t('updates.progress.title'))
  await downloadAndInstall()
  if (downloadPhase.value === 'success') {
    bgTask.finishTask('completed')
  } else if (downloadPhase.value === 'error') {
    bgTask.finishTask('failed')
  }
}

function closeProgress() {
  showProgress.value = false
  if (downloadPhase.value === 'success' || downloadPhase.value === 'error') {
    resetAppUpdater()
  }
}

async function cancelAppUpdateOp() {
  await bgTask.requestCancel()
}

async function handleRestart() {
  restarting.value = true
  try {
    await invoke('restart_application')
  } catch (e) {
    console.error('[AppUpdate] Restart failed:', e)
    restarting.value = false
  }
}

function minimizeAppUpdateOp() {
  bgTask.minimize(
    () => {
      showProgress.value = false
    },
    () => {
      showProgress.value = true
    }
  )
}

// Network diagnostic
interface DiagResult {
  success: boolean
  dns: string
  tcp: string
  http: string
  http_status: number | null
  http_body: string | null
  elapsed_ms: number
  conclusion: string
}

const diagRunning = ref(false)
const diagResult = ref<DiagResult | null>(null)
const diagError = ref<string | null>(null)

async function runDiag() {
  diagRunning.value = true
  diagError.value = null
  diagResult.value = null
  try {
    diagResult.value = await invoke<DiagResult>('diag_network')
  } catch (e: any) {
    diagError.value = e?.message || String(e)
  } finally {
    diagRunning.value = false
  }
}

watch(downloadPhase, async phase => {
  if (phase === 'downloading' || phase === 'installing') {
    showProgress.value = true
    showRestartButton.value = false
  }
  if (phase === 'success') {
    showRestartButton.value = true
    await store.loadAppMeta()
  }
  if (phase === 'error') {
    showRestartButton.value = false
  }
})
</script>

<template>
  <PageLayout :group="t('nav.group.system')" :title="t('about.title')" :description="t('about.description')">
    <div class="space-y-6">
      <!-- Version info card -->
      <div class="bg-gray-50 dark:bg-gray-800/50 rounded-lg p-5 space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <iconify-icon icon="mdi:package-variant-closed" width="24" class="text-indigo-500"></iconify-icon>
            <div>
              <p class="text-base font-medium text-gray-900 dark:text-gray-100">RustVerse</p>
              <p v-if="appUpdate" class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
                {{ appUpdate.currentVersion }} →
                <span class="text-indigo-600 dark:text-indigo-400 font-semibold">{{ appUpdate.version }}</span>
              </p>
              <p v-else class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
                v{{ store.appVersion || '1.0.0' }} • {{ t('updates.status.appUpToDate') }}
              </p>
            </div>
          </div>

          <!-- Update available -->
          <div v-if="appUpdate">
            <BaseButton variant="secondary" @click="handleDownloadAndInstall">
              {{ t('updates.action.installNow') }}
            </BaseButton>
          </div>

          <!-- Check for updates -->
          <div v-else>
            <BaseButton variant="secondary" :loading="appChecking" @click="handleCheckUpdate">
              {{ appChecking ? t('updates.status.appChecking') : t('updates.action.checkAppUpdate') }}
            </BaseButton>
          </div>
        </div>

        <!-- Update available banner -->
        <div
          v-if="appUpdate"
          class="flex items-center gap-2 px-3 py-2.5 rounded-lg bg-indigo-50 dark:bg-indigo-900/20 border border-indigo-200 dark:border-indigo-800"
        >
          <iconify-icon
            icon="mdi:information-outline"
            width="18"
            class="text-indigo-600 dark:text-indigo-400 shrink-0"
          ></iconify-icon>
          <p class="text-sm text-indigo-700 dark:text-indigo-300">
            {{ t('updates.status.appUpdateAvailable', { version: appUpdate.version }) }}
          </p>
        </div>
      </div>

      <!-- Network error banner -->
      <div
        v-if="checkError"
        class="flex items-center gap-2 px-3 py-2.5 rounded-lg bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800"
      >
        <iconify-icon
          icon="mdi:alert-outline"
          width="18"
          class="text-amber-600 dark:text-amber-400 shrink-0"
        ></iconify-icon>
        <p class="text-sm text-amber-700 dark:text-amber-300">{{ checkError }}</p>
      </div>

      <!-- Release notes -->
      <div v-if="appUpdate?.body" class="bg-gray-50 dark:bg-gray-800/50 rounded-lg p-5">
        <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">{{ t('common.label.releaseNotes') }}</h3>
        <div
          class="text-sm text-gray-600 dark:text-gray-400 leading-relaxed max-h-80 overflow-y-auto prose prose-sm dark:prose-invert"
        >
          <div v-html="appUpdate.body"></div>
        </div>
      </div>

      <!-- Network diagnostic panel -->
      <div class="bg-gray-50 dark:bg-gray-800/50 rounded-lg p-5 space-y-3">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <iconify-icon icon="mdi:network-outline" width="18" class="text-gray-500"></iconify-icon>
            <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ t('updates.networkDiag.title') }}</h3>
          </div>
          <BaseButton variant="secondary" :loading="diagRunning" @click="runDiag">
            {{ diagRunning ? t('updates.networkDiag.running') : t('updates.networkDiag.run') }}
          </BaseButton>
        </div>

        <!-- Diagnostic output -->
        <div
          v-if="diagResult"
          class="space-y-2 text-xs font-mono bg-gray-100 dark:bg-gray-900 rounded p-3 max-h-64 overflow-y-auto"
        >
          <p class="text-gray-600 dark:text-gray-400">
            {{ t('updates.networkDiag.elapsed', { ms: diagResult.elapsed_ms }) }}
          </p>
          <p
            :class="
              diagResult.dns.startsWith('OK') ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'
            "
          >
            DNS: {{ diagResult.dns }}
          </p>
          <p
            :class="
              diagResult.tcp.startsWith('OK') ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'
            "
          >
            TCP: {{ diagResult.tcp }}
          </p>
          <p
            :class="
              diagResult.http.startsWith('OK') ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'
            "
          >
            HTTP: {{ diagResult.http }}
          </p>

          <!-- Conclusion with actionable suggestions -->
          <div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
            <p
              class="text-sm leading-relaxed whitespace-normal font-sans"
              :class="diagResult.success ? 'text-green-700 dark:text-green-300' : 'text-amber-700 dark:text-amber-300'"
            >
              <iconify-icon
                :icon="diagResult.success ? 'mdi:check-circle-outline' : 'mdi:alert-outline'"
                width="16"
                class="inline-block align-text-bottom mr-1"
              ></iconify-icon>
              {{ diagResult.conclusion }}
            </p>
          </div>
        </div>
        <p v-if="diagError" class="text-sm text-red-600 dark:text-red-400">{{ diagError }}</p>
      </div>
    </div>

    <!-- Download progress dialog -->
    <ProgressDialog
      :visible="showProgress"
      :title="t('updates.progress.title')"
      :status="
        downloadPhase === 'downloading' || downloadPhase === 'installing'
          ? 'running'
          : downloadPhase === 'success'
          ? 'success'
          : 'error'
      "
      :status-text="
        downloadPhase === 'downloading'
          ? t('updates.status.appDownloading')
          : downloadPhase === 'installing'
          ? t('updates.status.appInstalling')
          : downloadPhase === 'success'
          ? t('updates.status.appUpdateReady')
          : t('updates.status.appUpdateFailed')
      "
      :lines="progressLines"
      @close="closeProgress"
      @cancel="cancelAppUpdateOp"
      @minimize="minimizeAppUpdateOp"
    />

    <!-- Restart button overlay when update is ready -->
    <div v-if="showRestartButton" class="fixed bottom-6 right-6 z-50 flex flex-col gap-3">
      <div
        class="bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-800 rounded-lg px-4 py-3 shadow-lg"
      >
        <p class="text-sm text-green-700 dark:text-green-300 mb-2">
          {{ t('updates.status.appUpdateReady') }}
        </p>
        <BaseButton variant="primary" :loading="restarting" @click="handleRestart">
          {{ t('updates.action.restartNow') || 'Restart Now' }}
        </BaseButton>
      </div>
    </div>
  </PageLayout>
</template>

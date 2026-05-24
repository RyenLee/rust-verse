<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-shell'
import { invoke } from '@tauri-apps/api/core'
import { useStore } from '../store'
import BaseButton from '../components/BaseButton.vue'
import PageLayout from '../components/PageLayout.vue'
import ProgressDialog from '../components/ProgressDialog.vue'
import { useAppUpdater } from '../composables/useAppUpdater'

const { t } = useI18n()
const store = useStore()
const PROJECT_URL = 'https://github.com/RyenLee/rust-verse'
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
  showProgress.value = true
  await downloadAndInstall()
}

function closeProgress() {
  showProgress.value = false
  if (downloadPhase.value === 'success' || downloadPhase.value === 'error') {
    resetAppUpdater()
  }
}

function openHomepage() {
  open(PROJECT_URL)
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
  }
  if (phase === 'success') {
    // Reload app metadata so the version display reflects the newly installed version.
    // The MSI installer has already updated the config; refresh from backend.
    await store.loadAppMeta()
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

      <!-- Project links -->
      <a
        class="flex items-center gap-2 px-3 py-2.5 rounded-lg bg-gray-100 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-sm text-gray-600 dark:text-gray-400 hover:text-indigo-600 dark:hover:text-indigo-400 hover:border-indigo-300 dark:hover:border-indigo-700 transition-colors cursor-pointer"
        @click.prevent="openHomepage"
      >
        <iconify-icon icon="mdi:github" width="18" class="shrink-0"></iconify-icon>
        <span>{{ t('about.homepage') }}</span>
        <iconify-icon icon="mdi:open-in-new" width="14" class="ml-auto shrink-0"></iconify-icon>
      </a>

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
        <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">Release Notes</h3>
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
            <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">Network Diagnostic</h3>
          </div>
          <BaseButton variant="secondary" :loading="diagRunning" @click="runDiag">
            {{ diagRunning ? 'Running...' : 'Run Diagnostic' }}
          </BaseButton>
        </div>

        <!-- Diagnostic output -->
        <div
          v-if="diagResult"
          class="space-y-2 text-xs font-mono bg-gray-100 dark:bg-gray-900 rounded p-3 max-h-48 overflow-y-auto"
        >
          <p class="text-gray-600 dark:text-gray-400">Elapsed: {{ diagResult.elapsed_ms }}ms</p>
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
          <p v-if="diagResult.http_body" class="text-gray-500 dark:text-gray-500 break-all">
            Body: {{ diagResult.http_body }}
          </p>
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
    />
  </PageLayout>
</template>

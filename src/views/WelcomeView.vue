<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useRustup, type EnvCheck } from '@/composables/useRustup'
import { useAppStore } from '@/composables/useAppStore'
import { useStore } from '@/store'
import { appLog } from '@/composables/useLogger'

// Safely initialize i18n
let t: ReturnType<typeof useI18n>['t']
try {
  const i18n = useI18n()
  t = i18n.t
} catch {
  t = ((key: string) => key) as any
}

// Safely initialize stores and composables
let checkEnv: ReturnType<typeof useRustup>['checkEnv']
let refreshProcessPath: ReturnType<typeof useRustup>['refreshProcessPath']
let initTheme: ReturnType<typeof useAppStore>['initTheme']
let store: ReturnType<typeof useStore>

try {
  const rustup = useRustup()
  checkEnv = rustup.checkEnv
  refreshProcessPath = rustup.refreshProcessPath
} catch (e) {
  throw e
}

try {
  initTheme = useAppStore().initTheme
} catch {
  // ignore
}

try {
  store = useStore()
} catch {
  store = { appVersion: 'unknown', appName: 'RustVerse' } as any
}

type Phase = 'idle' | 'detecting' | 'not-configured' | 'installing' | 'install-failed' | 'configured'
const phase = ref<Phase>('idle')
const envCheck = ref<EnvCheck | null>(null)
const detectError = ref('')
const installError = ref('')
const installLogs = ref<string[]>([])
const installProgress = ref(0)
const enterRequested = ref(false)
const detectLogs = ref<string[]>([])

// Emit for parent to transition to main
const emit = defineEmits<{
  (e: 'enter'): void
}>()

function simulateProgress() {
  installProgress.value = 0
  const interval = setInterval(() => {
    // Stop simulation once real progress kicks in (>= 5% from backend)
    if (installProgress.value >= 5) {
      clearInterval(interval)
      return
    }
    if (installProgress.value >= 90) {
      clearInterval(interval)
      return
    }
    installProgress.value += Math.random() * 3
    if (installProgress.value > 5) installProgress.value = 5
  }, 500)
  return interval
}

async function handleDetect() {
  phase.value = 'detecting'
  detectError.value = ''
  detectLogs.value = []

  // Listen for real-time check progress events
  const unlisten: UnlistenFn = await listen<string>('env-check-log', event => {
    detectLogs.value.push(event.payload)
    appLog.info('welcome-detect', event.payload)
  })

  try {
    await refreshProcessPath()
    envCheck.value = await checkEnv()

    await new Promise(r => setTimeout(r, 400)) // brief delay for visual feedback

    // Both rustup AND cargo must be available
    if (envCheck.value.rustup_installed && envCheck.value.cargo_installed) {
      phase.value = 'configured'
      appLog.info('welcome-detect', 'Environment check passed: rustup and cargo available')
    } else {
      phase.value = 'not-configured'
      appLog.info('welcome-detect', `Environment check: rustup=${envCheck.value.rustup_installed}, cargo=${envCheck.value.cargo_installed}`)
    }
  } catch (e: any) {
    detectError.value = e?.message || e?.toString?.() || String(e)
    phase.value = 'idle'
    appLog.error('welcome-detect', `Detection failed: ${detectError.value}`)
  } finally {
    unlisten()
  }
}

async function handleInstall() {
  phase.value = 'installing'
  installError.value = ''
  installLogs.value = []
  installProgress.value = 0

  const progressInterval = simulateProgress()

  // Listen for Tauri events
  const unlistenLog = await listen<string>('rustup-install-log', event => {
    installLogs.value.push(event.payload)
    appLog.info('welcome-install', event.payload)
    // Parse real download progress from log messages like "Downloading... 45% (4.2 MB)"
    const match = event.payload.match(/Downloading\.\.\.\s+(\d+)%/)
    if (match) {
      const pct = parseInt(match[1], 10)
      // Map download progress to 0-80% range (installation takes the remaining 20%)
      installProgress.value = Math.min(pct * 0.8, 80)
    }
  })
  const unlistenDone = await listen<void>('rustup-install-finished', () => {
    installProgress.value = 100
    appLog.info('welcome-install', 'Installation process finished')
  })

  try {
    await invoke('install_rustup')
    clearInterval(progressInterval)
    installProgress.value = 100
    appLog.info('welcome-install', 'install_rustup command completed successfully')

    // Re-check environment after install
    await refreshProcessPath()
    envCheck.value = await checkEnv()

    if (envCheck.value.rustup_installed && envCheck.value.cargo_installed) {
      phase.value = 'configured'
      appLog.info('welcome-install', 'Environment check passed: rustup and cargo available')
    } else {
      installError.value = t('app.installCompleteButNotFound')
      phase.value = 'install-failed'
      appLog.warn('welcome-install', 'Install completed but rustup/cargo not found in environment')
    }
  } catch (e: any) {
    clearInterval(progressInterval)
    const msg = e?.message || e?.toString?.() || String(e)
    installError.value = msg
    phase.value = 'install-failed'
    appLog.error('welcome-install', `Install failed: ${msg}`)
  } finally {
    unlistenLog()
    unlistenDone()
  }
}

function handleEnter() {
  enterRequested.value = true
  emit('enter')
}

function handleRetry() {
  // Always re-detect first. After an uninstall, residual files may cause
  // install_rustup to report "already installed" even though the toolchain
  // is broken. Re-detecting refreshes envCheck with the actual state.
  handleDetect()
}

// Check if the error is a download failure (to show manual guide)
const isDownloadError = computed(() => {
  return installError.value.toLowerCase().includes('download failed')
})

// Extract download URL from error message
const manualDownloadUrl = computed(() => {
  const match = installError.value.match(/download from\s+(https?:\/\/\S+)/i)
  return match ? match[1] : 'https://rustup.rs'
})

// Extract save path from error message
const manualSavePath = computed(() => {
  const match = installError.value.match(/save as\s+(\S+)/i)
  return match ? match[1] : '<app_dir>/data/rustup-init.exe'
})

function resetToIdle() {
  detectError.value = ''
  detectLogs.value = []
  phase.value = 'idle'
}

onMounted(() => {
  initTheme().catch(() => {})
})
</script>

<template>
  <div
    class="fixed inset-0 z-[998] flex items-center justify-center bg-white dark:bg-gray-950 select-none transition-colors duration-500"
  >
    <div class="flex flex-col items-center gap-8 px-6 max-w-sm w-full">
      <!-- Logo -->
      <div class="flex flex-col items-center gap-5">
        <div class="w-20 h-20 rounded-2xl bg-orange-50 dark:bg-orange-950/40 flex items-center justify-center">
          <iconify-icon icon="mdi:language-rust" width="48" class="text-orange-500"></iconify-icon>
        </div>
        <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100 tracking-tight">
          {{ store.appName || 'RustVerse' }}
        </h1>
        <p class="text-sm text-center text-gray-500 dark:text-gray-400 leading-relaxed">
          {{ t('welcome.subtitle') }}
        </p>
      </div>

      <!-- Idle: Show Detect button -->
      <Transition name="fade-up" mode="out-in">
        <button
          v-if="phase === 'idle'"
          key="detect"
          class="w-full py-3 px-6 rounded-xl bg-orange-600 hover:bg-orange-500 text-white font-semibold text-sm transition-all duration-200 shadow-sm hover:shadow-md active:scale-[0.98] cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-orange-400 inline-flex items-center justify-center gap-1.5"
          @click="handleDetect"
        >
          <iconify-icon icon="mdi:magnify" width="18"></iconify-icon>
          {{ t('welcome.detect') }}
        </button>
      </Transition>

      <!-- Detecting -->
      <Transition name="fade-up" mode="out-in">
        <div v-if="phase === 'detecting'" key="detecting" class="flex flex-col items-center gap-4 w-full">
          <div class="w-8 h-8 border-2 border-orange-500 border-t-transparent rounded-full animate-spin" />
          <p class="text-sm text-gray-500 dark:text-gray-400">{{ t('app.detecting') }}</p>

          <!-- Real-time check logs -->
          <div
            v-if="detectLogs.length > 0"
            class="w-full max-h-32 overflow-y-auto bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5"
          >
            <p v-for="(line, i) in detectLogs" :key="i" class="break-all">{{ line }}</p>
          </div>
        </div>
      </Transition>

      <!-- Not Configured: Show Install button -->
      <Transition name="fade-up" mode="out-in">
        <div v-if="phase === 'not-configured'" key="not-configured" class="flex flex-col items-center gap-5 w-full">
          <div
            class="flex items-center gap-2 text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/30 px-4 py-2 rounded-lg text-sm"
          >
            <iconify-icon icon="mdi:alert-circle-outline" width="20"></iconify-icon>

            <!-- Detailed reason -->
            <span v-if="!envCheck?.rustup_installed && !envCheck?.cargo_installed" class="min-w-0">
              Rust toolchain not found (rustup + cargo)
            </span>
            <span v-else-if="!envCheck?.rustup_installed" class="min-w-0">rustup not found</span>
            <span v-else-if="!envCheck?.cargo_installed" class="min-w-0"
              >cargo not found — rustup is installed, re-running installer to repair...</span
            >
          </div>

          <!-- Show logs from the failed detection -->
          <div
            v-if="detectLogs.length > 0"
            class="w-full max-h-32 overflow-y-auto bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5"
          >
            <p v-for="(line, i) in detectLogs" :key="i" class="break-all">{{ line }}</p>
          </div>

          <p class="text-xs text-gray-400 dark:text-gray-500 text-center leading-relaxed">
            {{ t('app.rustNotFoundDesc', { rustup: 'rustup', cargo: 'cargo' }) }}
          </p>
          <button
            class="w-full py-3 px-6 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-sm transition-all duration-200 shadow-sm hover:shadow-md active:scale-[0.98] cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-400 inline-flex items-center justify-center gap-1.5"
            @click="handleInstall"
          >
            <iconify-icon icon="mdi:download" width="18"></iconify-icon>
            {{ t('app.installRustup') }}
          </button>
          <button
            class="text-xs text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors cursor-pointer"
            @click="handleDetect"
          >
            {{ t('app.recheckEnv') }}
          </button>
        </div>
      </Transition>

      <!-- Installing -->
      <Transition name="fade-up" mode="out-in">
        <div v-if="phase === 'installing'" key="installing" class="flex flex-col items-center gap-5 w-full">
          <div class="w-full">
            <div class="flex items-center justify-between mb-2">
              <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('app.installing') }}</span>
              <span class="text-xs font-mono text-gray-500">{{ Math.round(installProgress) }}%</span>
            </div>
            <div class="h-2 bg-gray-200 dark:bg-gray-800 rounded-full overflow-hidden">
              <div
                class="h-full bg-emerald-500 rounded-full transition-all duration-500 ease-out"
                :style="{ width: installProgress + '%' }"
              />
            </div>
          </div>
          <div
            v-if="installLogs.length > 0"
            class="w-full max-h-40 overflow-y-auto bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5"
          >
            <p v-for="(line, i) in installLogs" :key="i" class="break-all">{{ line }}</p>
          </div>
        </div>
      </Transition>

      <!-- Install Failed -->
      <Transition name="fade-up" mode="out-in">
        <div v-if="phase === 'install-failed'" key="install-failed" class="flex flex-col items-center gap-4 w-full">
          <div
            class="flex items-start gap-2 text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 px-4 py-2 rounded-lg text-sm w-full"
          >
            <iconify-icon icon="mdi:close-circle-outline" width="20" class="shrink-0 mt-0.5"></iconify-icon>
            <span class="break-all min-w-0">{{ installError }}</span>
          </div>

          <!-- Manual placement guide when download fails -->
          <div
            v-if="isDownloadError"
            class="w-full bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-800 rounded-lg p-3 text-xs text-amber-800 dark:text-amber-200 space-y-1.5"
          >
            <p class="font-semibold text-sm">{{ t('app.manualGuideTitle') }}</p>
            <p>{{ t('app.manualGuideStep1') }}</p>
            <p class="font-mono bg-white dark:bg-gray-900 px-2 py-1 rounded text-xs break-all">
              {{ manualDownloadUrl }}
            </p>
            <p>{{ t('app.manualGuideStep2') }}</p>
            <p class="font-mono bg-white dark:bg-gray-900 px-2 py-1 rounded text-xs break-all">
              {{ manualSavePath }}
            </p>
            <p>{{ t('app.manualGuideStep3') }}</p>
          </div>

          <button
            class="w-full py-3 px-6 rounded-xl bg-orange-600 hover:bg-orange-500 text-white font-semibold text-sm transition-all duration-200 shadow-sm hover:shadow-md active:scale-[0.98] cursor-pointer inline-flex items-center justify-center gap-1.5"
            @click="handleRetry"
          >
            <iconify-icon icon="mdi:refresh" width="18"></iconify-icon>
            {{ t('welcome.retry') }}
          </button>
        </div>
      </Transition>

      <!-- Configured: Show Enter button -->
      <Transition name="fade-up" mode="out-in">
        <div
          v-if="phase === 'configured' || enterRequested"
          key="configured"
          class="flex flex-col items-center gap-3 w-full"
        >
          <div
            class="flex items-center gap-2 text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/30 px-4 py-2 rounded-lg text-sm"
          >
            <iconify-icon icon="mdi:check-circle-outline" width="20" class="shrink-0"></iconify-icon>
            <span class="min-w-0">rustup + cargo {{ t('welcome.ready') }}</span>
          </div>

          <!-- Show detection logs even on success -->
          <div
            v-if="detectLogs.length > 0"
            class="w-full max-h-32 overflow-y-auto bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5"
          >
            <p v-for="(line, i) in detectLogs" :key="i" class="break-all">{{ line }}</p>
          </div>

          <button
            class="w-full py-3 px-6 rounded-xl bg-orange-600 hover:bg-orange-500 text-white font-semibold text-sm transition-all duration-200 shadow-sm hover:shadow-md active:scale-[0.98] cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-orange-400 inline-flex items-center justify-center gap-1.5"
            @click="handleEnter"
          >
            <iconify-icon icon="mdi:arrow-right-circle-outline" width="18"></iconify-icon>
            {{ t('welcome.enter') }}
          </button>
        </div>
      </Transition>

      <!-- Generic detect error -->
      <Transition name="fade-up" mode="out-in">
        <div v-if="detectError" key="detect-error" class="flex flex-col items-center gap-4 w-full">
          <div
            class="flex items-center gap-2 text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 px-4 py-2 rounded-lg text-sm w-full"
          >
            <iconify-icon icon="mdi:alert-octagon-outline" width="20" class="shrink-0"></iconify-icon>
            <span class="break-all min-w-0">{{ detectError }}</span>
          </div>
          <button
            class="w-full py-3 px-6 rounded-xl bg-orange-600 hover:bg-orange-500 text-white font-semibold text-sm transition-all duration-200 shadow-sm hover:shadow-md active:scale-[0.98] cursor-pointer"
            @click="resetToIdle"
          >
            {{ t('welcome.retry') }}
          </button>
        </div>
      </Transition>

      <!-- Version -->
      <p class="text-xs text-gray-400 dark:text-gray-500 mt-8">RustVerse v{{ store.appVersion || '1.0.0' }}</p>
    </div>
  </div>
</template>

<style scoped>
/* fade-up transition */
.fade-up-enter-active {
  transition: all 0.3s ease-out;
}
.fade-up-leave-active {
  transition: all 0.2s ease-in;
}
.fade-up-enter-from {
  opacity: 0;
  transform: translateY(12px);
}
.fade-up-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>

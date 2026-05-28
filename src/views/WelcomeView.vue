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
      appLog.info(
        'welcome-detect',
        `Environment check: rustup=${envCheck.value.rustup_installed}, cargo=${envCheck.value.cargo_installed}`
      )
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

// Check if cargo is blocked by Windows security (os error 448)
const isBlockedBySecurity = computed(() => {
  const cargoErr = envCheck.value?.cargo_error || ''
  return (
    cargoErr.includes('os error 448') ||
    cargoErr.includes('448') ||
    cargoErr.includes('untrusted mount point') ||
    cargoErr.includes('不受信任')
  )
})

// Check if the error is a download failure (to show manual guide)

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
    class="fixed inset-0 z-[998] flex items-center justify-center bg-gradient-to-br from-white to-gray-50 dark:from-gray-950 dark:to-gray-900 select-none transition-colors duration-500"
  >
    <div class="flex flex-col items-center gap-10 px-6 max-w-md w-full">
      <!-- Header: Logo + Brand -->
      <div class="flex flex-col items-center gap-6">
        <!-- Logo Image -->
        <div class="relative">
          <img
            src="/icon.png"
            alt="RustVerse"
            class="w-24 h-24 rounded-3xl shadow-lg shadow-gray-500/20 dark:shadow-gray-900/30"
          />
        </div>
        <!-- Brand Text -->
        <div class="flex flex-col items-center gap-2 text-center">
          <h1 class="text-3xl font-bold text-gray-900 dark:text-gray-100 tracking-tight">
            {{ store.appName || 'RustVerse' }}
          </h1>
          <p class="text-sm text-gray-500 dark:text-gray-400 leading-relaxed max-w-xs">
            {{ t('welcome.subtitle') }}
          </p>
        </div>
      </div>

      <!-- Content Area: State-based UI -->
      <div class="w-full">
        <Transition name="fade-up" mode="out-in">
          <!-- Idle: Show Detect button -->
          <button
            v-if="phase === 'idle'"
            key="detect"
            class="w-full py-3.5 px-6 rounded-xl bg-gradient-to-r from-orange-600 to-amber-500 hover:from-orange-500 hover:to-amber-400 text-white font-semibold text-sm transition-all duration-200 shadow-md hover:shadow-lg active:scale-[0.98] cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-orange-400 inline-flex items-center justify-center gap-2"
            @click="handleDetect"
          >
            <iconify-icon icon="mdi:magnify" width="20"></iconify-icon>
            {{ t('welcome.detect') }}
          </button>
        </Transition>

        <Transition name="fade-up" mode="out-in">
          <!-- Detecting -->
          <div v-if="phase === 'detecting'" key="detecting" class="flex flex-col items-center gap-4 w-full">
            <div class="flex flex-col items-center gap-3">
              <div class="w-8 h-8 border-3 border-orange-500 border-t-transparent rounded-full animate-spin" />
              <p class="text-sm text-gray-500 dark:text-gray-400">{{ t('app.detecting') }}</p>
            </div>

            <!-- Real-time check logs -->
            <div
              v-if="detectLogs.length > 0"
              class="w-full max-h-32 overflow-y-auto bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5"
            >
              <p v-for="(line, i) in detectLogs" :key="i" class="break-all">{{ line }}</p>
            </div>
          </div>
        </Transition>

        <Transition name="fade-up" mode="out-in">
          <!-- Not Configured: Show Install button -->
          <div v-if="phase === 'not-configured'" key="not-configured" class="flex flex-col items-center gap-5 w-full">
            <!-- Windows Security blocking (os error 448) -->
            <div
              v-if="isBlockedBySecurity"
              class="w-full bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 rounded-xl p-4 space-y-3"
            >
              <div class="flex items-start gap-2 text-red-600 dark:text-red-400">
                <iconify-icon icon="mdi:shield-lock" width="20" class="mt-0.5 flex-shrink-0"></iconify-icon>
                <div class="text-sm leading-relaxed">
                  <strong>Windows 安全策略阻止了 Rust 工具链访问</strong>
                  <p class="mt-1 text-xs text-red-500 dark:text-red-400">
                    检测到错误：无法遍历该路径，因为它包含不受信任的装入点 (os error 448)
                  </p>
                  <p class="mt-1 text-xs text-red-500/80 dark:text-red-400/80">
                    这通常是因为 Windows Defender / 受控文件夹访问 阻止了应用访问 Rust 工具链目录
                  </p>
                </div>
              </div>
              <div class="text-xs text-gray-600 dark:text-gray-400 space-y-1">
                <p><strong>解决方法：</strong></p>
                <p>1. 打开「Windows 安全中心」→「病毒和威胁防护」→「管理设置」→「受控文件夹访问」</p>
                <p>2. 关闭「受控文件夹访问」或将 RustVerse 添加到「允许的应用」</p>
                <p>3. 或者：将工具链目录添加到 Windows Defender 排除项</p>
              </div>
              <button
                class="w-full py-2 px-4 rounded-lg bg-red-600 hover:bg-red-500 text-white text-sm font-medium transition-colors cursor-pointer"
                @click="handleDetect"
              >
                修复后重新检测
              </button>
            </div>

            <!-- Normal not-configured state -->
            <template v-else>
              <div
                class="flex items-center gap-2 text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/30 px-4 py-3 rounded-lg text-sm w-full"
              >
                <iconify-icon icon="mdi:alert-circle-outline" width="20" class="shrink-0"></iconify-icon>

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
                class="w-full max-h-32 overflow-y-auto bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5"
              >
                <p v-for="(line, i) in detectLogs" :key="i" class="break-all">{{ line }}</p>
              </div>

              <p class="text-xs text-gray-400 dark:text-gray-500 text-center leading-relaxed">
                {{ t('app.rustNotFoundDesc', { rustup: 'rustup', cargo: 'cargo' }) }}
              </p>
              <button
                class="w-full py-3 px-6 rounded-xl bg-gradient-to-r from-emerald-600 to-green-500 hover:from-emerald-500 hover:to-green-400 text-white font-semibold text-sm transition-all duration-200 shadow-md hover:shadow-lg active:scale-[0.98] cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-400 inline-flex items-center justify-center gap-2"
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
            </template>
          </div>
        </Transition>

        <Transition name="fade-up" mode="out-in">
          <!-- Installing -->
          <div v-if="phase === 'installing'" key="installing" class="flex flex-col items-center gap-5 w-full">
            <div class="w-full">
              <div class="flex items-center justify-between mb-2">
                <span class="text-xs text-gray-500 dark:text-gray-400">{{ t('app.installing') }}</span>
                <span class="text-xs font-mono text-gray-500">{{ Math.round(installProgress) }}%</span>
              </div>
              <div class="h-2.5 bg-gray-200 dark:bg-gray-800 rounded-full overflow-hidden">
                <div
                  class="h-full bg-gradient-to-r from-emerald-500 to-green-400 rounded-full transition-all duration-500 ease-out"
                  :style="{ width: installProgress + '%' }"
                />
              </div>
            </div>
            <div
              v-if="installLogs.length > 0"
              class="w-full max-h-40 overflow-y-auto bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5"
            >
              <p v-for="(line, i) in installLogs" :key="i" class="break-all">{{ line }}</p>
            </div>
          </div>
        </Transition>

        <Transition name="fade-up" mode="out-in">
          <!-- Install Failed -->
          <div v-if="phase === 'install-failed'" key="install-failed" class="flex flex-col items-center gap-4 w-full">
            <div
              class="flex items-start gap-2 text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 px-4 py-3 rounded-lg text-sm w-full"
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
              class="w-full py-3 px-6 rounded-xl bg-gradient-to-r from-orange-600 to-amber-500 hover:from-orange-500 hover:to-amber-400 text-white font-semibold text-sm transition-all duration-200 shadow-md hover:shadow-lg active:scale-[0.98] cursor-pointer inline-flex items-center justify-center gap-2"
              @click="handleRetry"
            >
              <iconify-icon icon="mdi:refresh" width="18"></iconify-icon>
              {{ t('welcome.retry') }}
            </button>
          </div>
        </Transition>

        <Transition name="fade-up" mode="out-in">
          <!-- Configured: Show Enter button -->
          <div
            v-if="phase === 'configured' || enterRequested"
            key="configured"
            class="flex flex-col items-center gap-4 w-full"
          >
            <div
              class="flex items-center gap-2 text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-950/30 px-4 py-3 rounded-lg text-sm w-full"
            >
              <iconify-icon icon="mdi:check-circle-outline" width="20" class="shrink-0"></iconify-icon>
              <span class="min-w-0">rustup + cargo {{ t('welcome.ready') }}</span>
            </div>

            <!-- Show detection logs even on success -->
            <div
              v-if="detectLogs.length > 0"
              class="w-full max-h-32 overflow-y-auto bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-800 rounded-lg p-3 text-xs font-mono text-gray-600 dark:text-gray-400 space-y-0.5"
            >
              <p v-for="(line, i) in detectLogs" :key="i" class="break-all">{{ line }}</p>
            </div>

            <button
              class="w-full py-3 px-6 rounded-xl bg-gradient-to-r from-orange-600 to-amber-500 hover:from-orange-500 hover:to-amber-400 text-white font-semibold text-sm transition-all duration-200 shadow-md hover:shadow-lg active:scale-[0.98] cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-orange-400 inline-flex items-center justify-center gap-2"
              @click="handleEnter"
            >
              <iconify-icon icon="mdi:arrow-right-circle-outline" width="18"></iconify-icon>
              {{ t('welcome.enter') }}
            </button>
          </div>
        </Transition>

        <Transition name="fade-up" mode="out-in">
          <!-- Generic detect error -->
          <div v-if="detectError" key="detect-error" class="flex flex-col items-center gap-4 w-full">
            <div
              class="flex items-center gap-2 text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950/30 px-4 py-3 rounded-lg text-sm w-full"
            >
              <iconify-icon icon="mdi:alert-octagon-outline" width="20" class="shrink-0"></iconify-icon>
              <span class="break-all min-w-0">{{ detectError }}</span>
            </div>
            <button
              class="w-full py-3 px-6 rounded-xl bg-gradient-to-r from-orange-600 to-amber-500 hover:from-orange-500 hover:to-amber-400 text-white font-semibold text-sm transition-all duration-200 shadow-md hover:shadow-lg active:scale-[0.98] cursor-pointer"
              @click="resetToIdle"
            >
              {{ t('welcome.retry') }}
            </button>
          </div>
        </Transition>
      </div>

      <!-- Footer: Version -->
      <p class="text-xs text-gray-400 dark:text-gray-500 mt-4">RustVerse v{{ store.appVersion || '1.0.0' }}</p>
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

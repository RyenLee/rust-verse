<script setup lang="ts">
import { onMounted, onBeforeUnmount, onErrorCaptured, ref, computed, nextTick, provide } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Toast from './components/Toast.vue'
import ProgressDialog from './components/ProgressDialog.vue'
import BackgroundTaskOverlay from './components/BackgroundTaskOverlay.vue'
import { useBackgroundTask } from './composables/useBackgroundTask'
import ConfirmDialog from './components/ConfirmDialog.vue'
import SplashScreen from './components/SplashScreen.vue'
import TopBar from './components/TopBar.vue'
import WelcomeView from './views/WelcomeView.vue'
import { useAppStore } from './composables/useAppStore'
import { useStore } from './store'
import { useRustup, type EnvCheck } from './composables/useRustup'
import { useToast } from './composables/useToast'
import { useDataRefresh } from './composables/useDataRefresh'
import { appLog } from './composables/useLogger'
import { withTimeout } from './composables/useWithTimeout'
import { initLocale } from './locales'
import { useAppUpdater } from './composables/useAppUpdater'

const { t } = useI18n()
const { checkEnv, refreshProcessPath, uninstallRustup } = useRustup()
const { onEnvVarChange } = useDataRefresh()
const store = useStore()

// === Use global update state ===
const { update, checkForUpdate } = useAppUpdater()
const updateAvailableInfo = computed(() => update.value)
provide('updateAvailableInfo', updateAvailableInfo)

// P0: keep-alive 策略优化 — 仅对高频切换页面启用缓存，避免所有页面常驻内存
const keepAliveNames = ['ToolchainListView', 'DashboardView']

function dismissUpdateNotification() {
  // Keep update data for page, just don't show badge (can be extended if needed)
}
provide('dismissUpdateNotification', dismissUpdateNotification)

// App phase: splash → welcome → main
type AppPhase = 'splash' | 'welcome' | 'main'
const phase = ref<AppPhase>('splash')
const envCheck = ref<EnvCheck | null>(null)
const uninstalling = ref(false)
const route = useRoute()
const router = useRouter()

// Sidebar collapsed state (persisted)
const sidebarCollapsed = ref(false)
const bgTask = useBackgroundTask()
try {
  const saved = localStorage.getItem('sidebar-collapsed')
  if (saved !== null) sidebarCollapsed.value = saved === 'true'
} catch {
  /* ignore */
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value
  try {
    localStorage.setItem('sidebar-collapsed', String(sidebarCollapsed.value))
  } catch {
    /* ignore */
  }
}

// Keyboard shortcut: Ctrl+B to toggle sidebar
function handleKeydown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === 'b') {
    e.preventDefault()
    toggleSidebar()
  }
}

// Splash screen state
const splashProgress = ref(0)
const splashStatusText = ref('')
const startupErrors = ref<string[]>([])

function logStartup(step: string, msg: string) {
  try {
    appLog.info('App', `${step}: ${msg}`)
  } catch {
    /* ignore */
  }
}

function logStartupError(step: string, msg: string) {
  startupErrors.value.push(`[${step}] ${msg}`)
  try {
    appLog.error('App', `${step} ERROR: ${msg}`)
  } catch {
    /* ignore */
  }
}

// Uninstall progress dialog state
const showUninstallConfirm = ref(false)
const showUninstallProgress = ref(false)
const uninstallProgressStatus = ref<'running' | 'success' | 'error'>('running')
const uninstallProgressLines = ref<string[]>([])
const uninstallProgressStatusText = ref('')
const { isDark, initTheme, toggleTheme, restoreWindowBounds, setupWindowBoundsListener } = useAppStore()

// === Error boundary for WelcomeView ===
const welcomeError = ref<string | null>(null)
const welcomeErrorStack = ref<string | null>(null)

function clearWelcomeError() {
  welcomeError.value = null
  welcomeErrorStack.value = null
}

function skipToMain() {
  phase.value = 'main'
  router.push('/')
}

onErrorCaptured((err: unknown, instance, info) => {
  const msg = err instanceof Error ? err.message : String(err)
  const stack = err instanceof Error ? err.stack : null
  logStartupError('welcome-render', `${msg} [info: ${info}]`)

  if (phase.value === 'welcome') {
    welcomeError.value = msg
    welcomeErrorStack.value = stack
  }

  return false
})

// ===== Navigation config with groups =====
interface NavItem {
  path: string
  label: string
  icon: string
}

interface NavGroup {
  key: string
  label: string
  icon: string
  color: string
  items: NavItem[]
}

const navGroups = computed<NavGroup[]>(() => [
  {
    key: 'overview',
    label: t('nav.group.overview'),
    icon: 'mdi:view-dashboard-outline',
    color: 'orange',
    items: [{ path: '/', label: t('nav.dashboard'), icon: 'mdi:view-dashboard-outline' }],
  },
  {
    key: 'toolchain',
    label: t('nav.group.toolchain'),
    icon: 'mdi:cog-outline',
    color: 'sky',
    items: [
      { path: '/toolchains', label: t('nav.toolchains'), icon: 'mdi:cog-outline' },
      { path: '/history-versions', label: t('nav.historyVersions'), icon: 'mdi:history' },
      { path: '/components', label: t('nav.components'), icon: 'mdi:puzzle-outline' },
      { path: '/targets', label: t('nav.targets'), icon: 'mdi:target' },
      { path: '/rustup-mirror', label: t('nav.rustupMirror'), icon: 'mdi:lightning-bolt' },
    ],
  },
  {
    key: 'config',
    label: t('nav.group.config'),
    icon: 'mdi:tune-vertical',
    color: 'violet',
    items: [
      { path: '/env-vars', label: t('nav.envVars'), icon: 'mdi:variable' },
      { path: '/mirrors', label: t('nav.mirrors'), icon: 'mdi:mirror' },
      { path: '/overrides', label: t('nav.overrides'), icon: 'mdi:folder-marker-outline' },
    ],
  },
  {
    key: 'extend',
    label: t('nav.group.extend'),
    icon: 'mdi:rocket-launch-outline',
    color: 'emerald',
    items: [
      { path: '/plugins', label: t('nav.plugins'), icon: 'mdi:power-plug-outline' },
    ],
  },
  {
    key: 'system',
    label: t('nav.group.system'),
    icon: 'mdi:desktop-tower-monitor',
    color: 'slate',
    items: [
      { path: '/notifications', label: t('nav.notifications'), icon: 'mdi:bell-outline' },
      { path: '/settings', label: t('nav.settings'), icon: 'mdi:cog-outline' },
      { path: '/about', label: t('nav.appUpdate'), icon: 'mdi:update' },
    ],
  },
])

async function recheckEnv() {
  try {
    try {
      await refreshProcessPath()
    } catch {
      // Non-critical
    }
    envCheck.value = await checkEnv()
  } catch {
    envCheck.value = {
      rustup_installed: false,
      cargo_installed: false,
      rustup_error: null,
      cargo_error: null,
      cargo_home: null,
      rustup_home: null,
    }
  }
}

function advanceSplash(stepIndex: number) {
  const steps = [
    { progress: 25, text: 'Checking locale...' },
    { progress: 50, text: 'Loading theme...' },
    { progress: 75, text: 'Refreshing environment...' },
    { progress: 100, text: 'Starting...' },
  ]
  const step = steps[stepIndex]
  if (step) {
    splashProgress.value = step.progress
    splashStatusText.value = step.text
  }
}

async function handleUninstallRustup() {
  if (!(await bgTask.guardStart())) {
    return
  }
  uninstalling.value = true
  showUninstallProgress.value = true
  uninstallProgressStatus.value = 'running'
  uninstallProgressLines.value = [t('app.uninstallStepChecking')]
  uninstallProgressStatusText.value = ''
  bgTask.startTask(t('app.uninstallRustup'))

  try {
    uninstallProgressLines.value.push(t('app.uninstallStepRemoving'))
    uninstallProgressStatusText.value = t('app.uninstallStepRemoving')
    const output = await uninstallRustup()

    if (output) {
      output
        .split('\n')
        .filter(Boolean)
        .forEach(line => {
          uninstallProgressLines.value.push(line)
        })
    }
    uninstallProgressLines.value.push(t('app.uninstallStepDone'))
    uninstallProgressStatus.value = 'success'
    uninstallProgressStatusText.value = t('app.uninstallSuccess')
    bgTask.finishTask('completed')

    setTimeout(async () => {
      showUninstallProgress.value = false
      await recheckEnv()
      router.push('/')
      phase.value = 'welcome'
    }, 2000)
  } catch (e: any) {
    const msg = e?.message || e?.toString?.() || String(e)
    const errorLines = msg
      .replace(/^command execution failed:\s*/, '')
      .split('\n')
      .filter(Boolean)
    errorLines.forEach(line => {
      uninstallProgressLines.value.push(line)
    })

    uninstallProgressStatus.value = 'error'
    bgTask.finishTask('failed')
    if (msg.includes('rustup is not installed')) {
      uninstallProgressStatusText.value = t('app.uninstallNotInstalled')
    } else if (
      msg.includes('os error 32') ||
      msg.includes('os error 5') ||
      msg.includes('another program') ||
      msg.includes('being used') ||
      msg.includes('拒绝访问')
    ) {
      uninstallProgressStatusText.value = t('app.uninstallFileLocked')
    } else {
      uninstallProgressStatusText.value = t('app.uninstallFailed', { error: errorLines[0] || msg })
    }
  } finally {
    uninstalling.value = false
  }

  await recheckEnv()
}

function confirmUninstall() {
  showUninstallConfirm.value = false
  handleUninstallRustup()
}

function closeUninstallProgress() {
  showUninstallProgress.value = false
}

async function cancelUninstallOp() {
  await bgTask.requestCancel()
  uninstallProgressStatus.value = 'error'
  uninstallProgressLines.value.push(t('common.message.operationCancelled'))
}

function minimizeUninstallOp() {
  bgTask.minimize(
    () => { showUninstallProgress.value = false },
    () => { showUninstallProgress.value = true }
  )
}

// Expose uninstall trigger for child views (e.g. DashboardView)
function triggerUninstall() {
  showUninstallConfirm.value = true
}
provide('triggerUninstall', triggerUninstall)

window.addEventListener('unhandledrejection', event => {
  startupErrors.value.push(`[unhandled] ${event.reason}`)
  event.preventDefault()
})

onMounted(async () => {
  const t0 = performance.now()
  logStartup('step0', 'onMounted started')
  startupErrors.value = []

  // Register keyboard shortcuts
  document.addEventListener('keydown', handleKeydown)

  advanceSplash(0)

  try {
    const tLocale = performance.now()
    logStartup('step0', 'Starting initLocale (timeout 8s)...')
    const localeResult = await withTimeout(initLocale(), 8000)
    if (!localeResult.ok) {
      logStartupError('step0', 'initLocale timed out after 8s - continuing with defaults')
    } else {
      logStartup('step0', `initLocale completed in ${Math.round(performance.now() - tLocale)}ms`)
    }

    advanceSplash(1)

    const tTheme = performance.now()
    logStartup('step1', 'Starting initTheme (timeout 5s)...')
    const themeResult = await withTimeout(initTheme(), 5000)
    if (!themeResult.ok) {
      logStartupError('step1', 'initTheme timed out after 5s - continuing with defaults')
    } else {
      logStartup('step1', `initTheme completed in ${Math.round(performance.now() - tTheme)}ms`)
    }
    advanceSplash(2)

    const tMeta = performance.now()
    logStartup('step1.5', 'Starting loadAppMeta (timeout 5s)...')
    const metaResult = await withTimeout(store.loadAppMeta(), 5000)
    if (!metaResult.ok) {
      logStartupError('step1.5', 'loadAppMeta timed out after 5s - using defaults')
    } else {
      logStartup(
        'step1.5',
        `loadAppMeta completed in ${Math.round(performance.now() - tMeta)}ms: name=${store.appName} version=${
          store.appVersion
        }`
      )
    }

    const tPath = performance.now()
    logStartup('step2', 'Starting refreshProcessPath...')
    const pathResult = await withTimeout(refreshProcessPath(), 5000)
    if (!pathResult.ok) {
      logStartupError('step2', 'refreshProcessPath timed out after 5s')
    } else {
      logStartup('step2', `refreshProcessPath completed in ${Math.round(performance.now() - tPath)}ms`)
    }

    const tBounds = performance.now()
    logStartup('step3', 'Starting restoreWindowBounds...')
    const boundsResult = await withTimeout(restoreWindowBounds(), 5000)
    if (!boundsResult.ok) {
      logStartupError('step3', 'restoreWindowBounds timed out after 5s')
    } else {
      logStartup('step3', `restoreWindowBounds completed in ${Math.round(performance.now() - tBounds)}ms`)
    }
    advanceSplash(3)
    setupWindowBoundsListener()
  } catch (e) {
    logStartupError('top-level', `onMounted catch block triggered: ${e}`)
  }

  await new Promise(resolve => setTimeout(resolve, 300))
  logStartup(
    'done',
    `Startup completed in ${Math.round(performance.now() - t0)}ms, errors=${startupErrors.value.length}`
  )

  phase.value = 'welcome'
  await nextTick()
  logStartup('done', `Phase switched to welcome. WelcomeView mounted.`)

  recheckEnv().then(() => {
    logStartup('deferred', 'Environment check completed in background')
  })
})

function handleWelcomeEnter() {
  logStartup('enter', 'User clicked Enter, switching to main')
  phase.value = 'main'
  router.push('/')

  // Schedule auto-update check after 1 minute
  scheduleAutoUpdateCheck()
}

// === Auto-update check (1 minute after app starts) ===
let autoUpdateTimer: ReturnType<typeof setTimeout> | null = null

async function scheduleAutoUpdateCheck() {
  // Clear any existing timer
  if (autoUpdateTimer) {
    clearTimeout(autoUpdateTimer)
    autoUpdateTimer = null
  }

  // Wait 1 minute, then check for updates
  autoUpdateTimer = setTimeout(async () => {
    logStartup('auto-update', 'Starting auto-update check...')
    try {
      const result = await checkForUpdate()
      if (result) {
        logStartup('auto-update', `Update available: ${result.version}`)
      } else {
        logStartup('auto-update', 'App is up to date')
      }
    } catch (e: any) {
      logStartup('auto-update', `Update check failed: ${e?.message || e}`)
    }
  }, 60_000)
}

const stopEnvVarWatch = onEnvVarChange(() => {
  recheckEnv()
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown)
  stopEnvVarWatch()
  if (autoUpdateTimer) {
    clearTimeout(autoUpdateTimer)
    autoUpdateTimer = null
  }
})
</script>

<template>
  <!-- Splash screen phase -->
  <SplashScreen
    v-if="phase === 'splash'"
    :progress="splashProgress"
    :status-text="splashStatusText"
    :startup-errors="startupErrors"
  />

  <!-- Welcome phase -->
  <WelcomeView v-if="phase === 'welcome' && !welcomeError" @enter="handleWelcomeEnter" />

  <!-- WelcomeView crash fallback -->
  <div
    v-if="phase === 'welcome' && welcomeError"
    class="fixed inset-0 z-[998] flex items-center justify-center bg-gray-50 dark:bg-gray-950"
  >
    <div class="flex flex-col items-center gap-4 px-6 max-w-md w-full text-center">
      <iconify-icon icon="mdi:alert-octagon" width="48" class="text-red-500"></iconify-icon>
      <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{{ t('common.dialog.welcomePageError') }}</h2>
      <p class="text-sm text-red-600 dark:text-red-400 break-all font-mono">{{ welcomeError }}</p>
      <pre
        v-if="welcomeErrorStack"
        class="text-xs text-gray-500 dark:text-gray-400 text-left w-full overflow-x-auto max-h-40 p-3 bg-gray-100 dark:bg-gray-800 rounded-lg"
        >{{ welcomeErrorStack }}</pre
      >
      <button
        class="px-6 py-2 rounded-lg bg-orange-600 hover:bg-orange-500 text-white font-medium text-sm transition-colors cursor-pointer"
        @click="clearWelcomeError"
      >
        {{ t('common.action.retry') }}
      </button>
      <button
        class="px-6 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white font-medium text-sm transition-colors cursor-pointer"
        @click="skipToMain"
      >
        {{ t('common.action.skipToMain') }}
      </button>
    </div>
  </div>

  <!-- Main app layout -->
  <Transition name="main-appear">
    <div v-if="phase === 'main'" class="flex h-screen overflow-hidden">
      <!-- Sidebar -->
      <nav
        class="sidebar bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-800 flex flex-col shrink-0 transition-[width] duration-200 ease-[cubic-bezier(0.4,0,0.2,1)]"
        :class="sidebarCollapsed ? 'w-16' : 'w-60'"
      >
        <!-- Brand -->
        <div class="px-5 py-4 flex items-center gap-3" :class="sidebarCollapsed && 'justify-center px-0'">
          <img
            src="/icon.png"
            alt="RustVerse"
            class="w-6 h-6 shrink-0 rounded-md"
          />
          <h1
            v-if="!sidebarCollapsed"
            class="text-base font-semibold text-gray-900 dark:text-gray-100 tracking-tight truncate"
          >
            {{ store.appName || 'RustVerse' }}
          </h1>
        </div>

        <!-- Navigation with groups -->
        <div class="flex-1 overflow-y-auto px-3 py-1 space-y-4">
          <div v-for="group in navGroups" :key="group.key" :class="`nav-group-${group.color}`">
            <!-- Group label (hidden when collapsed) -->
            <div
              v-if="!sidebarCollapsed"
              class="px-3 pt-2 pb-1 text-[11px] font-semibold uppercase tracking-wider text-gray-400 dark:text-gray-500"
            >
              {{ group.label }}
            </div>

            <!-- Nav items -->
            <div class="space-y-0.5">
              <router-link
                v-for="item in group.items"
                :key="item.path"
                :to="item.path"
                :class="[
                  'sidebar-nav-item flex items-center gap-3 px-3 py-2 rounded-lg text-[13px] font-medium transition-all duration-150',
                  sidebarCollapsed && 'justify-center px-0',
                  route.path === item.path
                    ? 'active'
                    : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-800/60',
                ]"
                :title="sidebarCollapsed ? item.label : undefined"
              >
                <iconify-icon
                  :icon="item.icon"
                  width="18"
                  :class="[
                    'shrink-0 transition-colors duration-150',
                    route.path === item.path ? '' : 'text-gray-400 dark:text-gray-500',
                  ]"
                ></iconify-icon>
                <span v-if="!sidebarCollapsed" class="truncate">{{ item.label }}</span>
              </router-link>
            </div>
          </div>
        </div>
      </nav>

      <!-- Main area: TopBar + Content -->
      <div class="flex-1 flex flex-col bg-white dark:bg-gray-950 overflow-hidden">
        <!-- Global TopBar -->
        <TopBar @toggle-sidebar="toggleSidebar" />

        <!-- Page content -->
        <main class="flex-1 overflow-hidden">
          <router-view v-slot="{ Component, route: r }">
            <Transition name="route" mode="out-in">
              <keep-alive :include="keepAliveNames">
                <component :is="Component" :key="r.fullPath" />
              </keep-alive>
            </Transition>
          </router-view>
        </main>
      </div>

      <Toast />
      <BackgroundTaskOverlay />
    </div>
  </Transition>

  <ConfirmDialog
    v-if="phase === 'main'"
    :visible="showUninstallConfirm"
    :title="t('app.uninstallRustup')"
    :message="t('app.uninstallConfirmMsg')"
    :confirm-label="t('app.uninstallRustup')"
    :danger="true"
    @confirm="confirmUninstall"
    @cancel="showUninstallConfirm = false"
  />
  <ProgressDialog
    v-if="phase === 'main'"
    :visible="showUninstallProgress"
    :title="t('app.uninstallRustup')"
    :status="uninstallProgressStatus"
    :status-text="uninstallProgressStatusText"
    :lines="uninstallProgressLines"
    @close="closeUninstallProgress"
    @cancel="cancelUninstallOp"
    @minimize="minimizeUninstallOp"
  />
</template>

<style scoped>
/* Route transition */
.route-enter-active {
  transition: opacity 0.18s ease-out, transform 0.18s ease-out;
}
.route-leave-active {
  transition: opacity 0.12s ease-in, transform 0.12s ease-in;
}
.route-enter-from {
  opacity: 0;
  transform: translateX(8px);
}
.route-leave-to {
  opacity: 0;
  transform: translateX(-8px);
}

/* Main app appear animation */
.main-appear-enter-active {
  transition: opacity 0.35s ease-out, transform 0.35s ease-out;
}
.main-appear-enter-from {
  opacity: 0;
  transform: scale(0.97);
}
</style>

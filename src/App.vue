<script setup lang="ts">
import { onMounted, onBeforeUnmount, onErrorCaptured, ref, computed, nextTick, provide } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Toast from './components/Toast.vue'
import ProgressDialog from './components/ProgressDialog.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import SplashScreen from './components/SplashScreen.vue'
import WelcomeView from './views/WelcomeView.vue'
import { useAppStore } from './composables/useAppStore'
import { useStore } from './store'
import { useRustup, type EnvCheck } from './composables/useRustup'
import { useToast } from './composables/useToast'
import { useDataRefresh } from './composables/useDataRefresh'
import { appLog } from './composables/useLogger'
import { withTimeout } from './composables/useWithTimeout'
import { initLocale, setLocale, getLocale, getAvailableLocales, type LocaleInfo } from './locales'

const { t } = useI18n()
const { checkEnv, refreshProcessPath, uninstallRustup } = useRustup()
const { onEnvVarChange } = useDataRefresh()
const store = useStore()

// App phase: splash → welcome → main
type AppPhase = 'splash' | 'welcome' | 'main'
const phase = ref<AppPhase>('splash')
const envCheck = ref<EnvCheck | null>(null)
const uninstalling = ref(false)
const route = useRoute()
const router = useRouter()

// Splash screen state
const splashProgress = ref(0)
const splashStatusText = ref('')
const startupErrors = ref<string[]>([])

function logStartup(step: string, msg: string) {
  const ts = new Date().toISOString().split('T')[1].slice(0, 12)
  console.log(`[${ts}] [startup] ${step}: ${msg}`)
  try {
    appLog.info('App', `${step}: ${msg}`)
  } catch {
    /* ignore */
  }
}

function logStartupError(step: string, msg: string) {
  const ts = new Date().toISOString().split('T')[1].slice(0, 12)
  console.error(`[${ts}] [startup] ${step} ERROR: ${msg}`)
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
  console.error('[ErrorBoundary] Caught error:', msg, stack, 'info:', info)
  logStartupError('welcome-render', `${msg} [info: ${info}]`)

  // Only show error UI if WelcomeView is active
  if (phase.value === 'welcome') {
    welcomeError.value = msg
    welcomeErrorStack.value = stack
  }

  // Prevent propagation to Vue's default error handler (which would unmount the app)
  return false
})
// === End error boundary ===

const currentLocale = ref(getLocale())
const availableLocales = ref<LocaleInfo[]>(getAvailableLocales())
const localeDropdownOpen = ref(false)
const localeDropdownRef = ref<HTMLElement | null>(null)

function handleClickOutside(e: MouseEvent) {
  if (localeDropdownRef.value && !localeDropdownRef.value.contains(e.target as Node)) {
    localeDropdownOpen.value = false
  }
}

const navItems = computed(() => [
  { path: '/', label: t('nav.dashboard'), icon: 'mdi:view-dashboard-outline' },
  { path: '/env-vars', label: t('nav.envVars'), icon: 'mdi:variable' },
  { path: '/toolchains', label: t('nav.toolchains'), icon: 'mdi:cog-outline' },
  { path: '/components', label: t('nav.components'), icon: 'mdi:puzzle-outline' },
  { path: '/targets', label: t('nav.targets'), icon: 'mdi:target' },
  { path: '/overrides', label: t('nav.overrides'), icon: 'mdi:folder-marker-outline' },
  { path: '/updates', label: t('nav.updates'), icon: 'mdi:cloud-download-outline' },
  { path: '/plugins', label: t('nav.plugins'), icon: 'mdi:power-plug-outline' },
  { path: '/help', label: t('nav.help'), icon: 'mdi:help-circle-outline' },
])

const currentLocaleInfo = computed(() => availableLocales.value.find(l => l.code === currentLocale.value))

function selectLocale(code: string) {
  currentLocale.value = code
  setLocale(code)
  localeDropdownOpen.value = false
}

async function recheckEnv() {
  try {
    try {
      await refreshProcessPath()
    } catch {
      // Non-critical
    }
    envCheck.value = await checkEnv()
  } catch (e) {
    console.error('Failed to check environment:', e)
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
  uninstalling.value = true
  showUninstallProgress.value = true
  uninstallProgressStatus.value = 'running'
  uninstallProgressLines.value = [t('app.uninstallStepChecking')]
  uninstallProgressStatusText.value = ''

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

    // After a brief pause, return to welcome page
    setTimeout(() => {
      showUninstallProgress.value = false
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

// Expose uninstall trigger for child views (e.g. DashboardView)
function triggerUninstall() {
  showUninstallConfirm.value = true
}
provide('triggerUninstall', triggerUninstall)

window.addEventListener('unhandledrejection', event => {
  console.error('[unhandledrejection]', event.reason)
  startupErrors.value.push(`[unhandled] ${event.reason}`)
  event.preventDefault()
})

onMounted(async () => {
  const t0 = performance.now()
  console.log('=== RustVerse frontend onMounted ===')
  logStartup('step0', 'onMounted started')
  startupErrors.value = []

  advanceSplash(0)

  try {
    // Step 0: Init locale
    const tLocale = performance.now()
    logStartup('step0', 'Starting initLocale (timeout 8s)...')
    const localeResult = await withTimeout(initLocale(), 8000)
    if (!localeResult.ok) {
      logStartupError('step0', 'initLocale timed out after 8s - continuing with defaults')
    } else {
      logStartup('step0', `initLocale completed in ${Math.round(performance.now() - tLocale)}ms`)
    }

    currentLocale.value = getLocale()
    availableLocales.value = getAvailableLocales()
    advanceSplash(1)

    // Step 1: Init theme
    const tTheme = performance.now()
    logStartup('step1', 'Starting initTheme (timeout 5s)...')
    const themeResult = await withTimeout(initTheme(), 5000)
    if (!themeResult.ok) {
      logStartupError('step1', 'initTheme timed out after 5s - continuing with defaults')
    } else {
      logStartup('step1', `initTheme completed in ${Math.round(performance.now() - tTheme)}ms`)
    }
    advanceSplash(2)

    // Step 1.5: Load app metadata from config.toml [app]
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

    // Step 2: Refresh PATH (non-critical)
    const tPath = performance.now()
    logStartup('step2', 'Starting refreshProcessPath...')
    const pathResult = await withTimeout(refreshProcessPath(), 5000)
    if (!pathResult.ok) {
      logStartupError('step2', 'refreshProcessPath timed out after 5s')
    } else {
      logStartup('step2', `refreshProcessPath completed in ${Math.round(performance.now() - tPath)}ms`)
    }

    // Step 3: Restore window bounds
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

  // Transition to welcome phase
  console.log('[App] About to switch phase to welcome, document.readyState:', document.readyState)
  phase.value = 'welcome'
  await nextTick()
  logStartup(
    'done',
    `Phase switched to welcome. WelcomeView mounted, document.body has ${document.body.children.length} children`
  )
  console.log('[App] Phase switched to welcome, children:', document.body.children.length)

  // Deferred: check Rust environment in background
  recheckEnv().then(() => {
    logStartup('deferred', 'Environment check completed in background')
  })
})

function handleWelcomeEnter() {
  logStartup('enter', 'User clicked Enter, switching to main')
  phase.value = 'main'
  router.push('/')
}

// Watch for env var changes and re-check environment
const stopEnvVarWatch = onEnvVarChange(() => {
  recheckEnv()
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside, true)
  stopEnvVarWatch()
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
      <h2 class="text-lg font-semibold text-gray-900 dark:text-gray-100">Welcome Page Error</h2>
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
        Retry
      </button>
      <button
        class="px-6 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white font-medium text-sm transition-colors cursor-pointer"
        @click="skipToMain"
      >
        Skip to Main
      </button>
    </div>
  </div>

  <!-- Main app layout (fade-in on enter) -->
  <Transition name="main-appear">
    <div v-if="phase === 'main'" class="flex h-screen overflow-hidden">
      <!-- Sidebar -->
      <nav
        class="sidebar w-60 bg-gray-50 dark:bg-gray-900 border-r border-gray-200 dark:border-gray-800 flex flex-col shrink-0"
      >
        <!-- Brand -->
        <div class="sidebar-brand px-5 py-5 flex items-center gap-3">
          <iconify-icon icon="mdi:language-rust" width="24" class="text-orange-500 shrink-0"></iconify-icon>
          <h1 class="text-base font-semibold text-gray-900 dark:text-gray-100 tracking-tight truncate">
            {{ store.appName || 'RustVerse' }}
          </h1>
        </div>

        <!-- Navigation -->
        <div class="flex-1 overflow-y-auto px-3 py-1 space-y-0.5">
          <router-link
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            :class="[
              'sidebar-nav-item flex items-center gap-3 px-3 py-2 rounded-lg text-[13px] font-medium transition-all duration-150',
              route.path === item.path
                ? 'active bg-sky-50 dark:bg-sky-950/40 text-sky-700 dark:text-sky-300'
                : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-800/60',
            ]"
          >
            <iconify-icon
              :icon="item.icon"
              width="18"
              :class="[
                'shrink-0 transition-colors duration-150',
                route.path === item.path ? 'text-sky-600 dark:text-sky-400' : 'text-gray-400 dark:text-gray-500',
              ]"
            ></iconify-icon>
            <span class="truncate">{{ item.label }}</span>
          </router-link>
        </div>

        <!-- Bottom controls -->
        <div class="sidebar-footer border-t border-gray-200 dark:border-gray-800 p-3 space-y-1">
          <!-- Language switcher -->
          <div ref="localeDropdownRef" class="relative">
            <button
              class="sidebar-control flex items-center gap-3 w-full px-3 py-2 rounded-lg text-[13px] font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-800/60 transition-all duration-150"
              @click="localeDropdownOpen = !localeDropdownOpen"
            >
              <iconify-icon icon="mdi:translate" width="18" class="text-gray-400 dark:text-gray-500"></iconify-icon>
              <span class="flex-1 text-left truncate">{{ currentLocaleInfo?.name || currentLocale }}</span>
              <iconify-icon
                icon="mdi:chevron-down"
                width="16"
                class="text-gray-400 transition-transform duration-200"
                :class="localeDropdownOpen ? 'rotate-180' : ''"
              ></iconify-icon>
            </button>
            <!-- Dropdown -->
            <Transition name="dropdown">
              <div
                v-if="localeDropdownOpen"
                class="absolute bottom-full left-0 right-0 mb-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg shadow-gray-200/50 dark:shadow-black/30 overflow-hidden z-50"
              >
                <button
                  v-for="loc in availableLocales"
                  :key="loc.code"
                  :class="[
                    'flex items-center gap-3 w-full px-3 py-2 text-[13px] transition-colors',
                    loc.code === currentLocale
                      ? 'bg-sky-50 dark:bg-sky-950/40 text-sky-700 dark:text-sky-300'
                      : 'text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-700/50',
                  ]"
                  @click="selectLocale(loc.code)"
                >
                  <iconify-icon
                    icon="mdi:check"
                    width="16"
                    class="shrink-0"
                    :class="loc.code === currentLocale ? 'text-sky-600 dark:text-sky-400' : 'text-transparent'"
                  ></iconify-icon>
                  <span class="flex-1 text-left">{{ loc.name }}</span>
                  <span class="text-xs text-gray-400 dark:text-gray-500">{{ loc.english_name }}</span>
                </button>
              </div>
            </Transition>
          </div>

          <!-- Theme toggle -->
          <button
            class="sidebar-control flex items-center gap-3 w-full px-3 py-2 rounded-lg text-[13px] font-medium text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-800/60 transition-all duration-150"
            @click="toggleTheme"
          >
            <Transition name="theme-icon" mode="out-in">
              <iconify-icon
                v-if="isDark"
                key="sun"
                icon="mdi:weather-sunny"
                width="18"
                class="text-amber-400"
              ></iconify-icon>
              <iconify-icon
                v-else
                key="moon"
                icon="mdi:weather-night"
                width="18"
                class="text-indigo-400"
              ></iconify-icon>
            </Transition>
            <span class="truncate">{{ isDark ? t('app.lightMode') : t('app.darkMode') }}</span>
          </button>

          <!-- Uninstall rustup (only show if installed) -->
          <button
            v-if="envCheck?.rustup_installed"
            class="sidebar-control flex items-center gap-3 w-full px-3 py-2 rounded-lg text-[13px] font-medium text-red-500 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300 hover:bg-red-50 dark:hover:bg-red-900/20 transition-all duration-150"
            :disabled="uninstalling"
            @click="showUninstallConfirm = true"
          >
            <iconify-icon icon="mdi:delete-outline" width="18" class="shrink-0"></iconify-icon>
            <span class="truncate">{{ uninstalling ? t('app.uninstalling') : t('app.uninstallRustup') }}</span>
          </button>
        </div>
      </nav>

      <!-- Main content -->
      <main class="flex-1 bg-white dark:bg-gray-950 overflow-hidden">
        <router-view v-slot="{ Component }">
          <keep-alive>
            <component :is="Component" />
          </keep-alive>
        </router-view>
      </main>

      <Toast />
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
  />
</template>

<style scoped>
/* Dropdown animation */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

/* Theme icon switch animation */
.theme-icon-enter-active,
.theme-icon-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.theme-icon-enter-from {
  opacity: 0;
  transform: rotate(-90deg) scale(0.8);
}
.theme-icon-leave-to {
  opacity: 0;
  transform: rotate(90deg) scale(0.8);
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

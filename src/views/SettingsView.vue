<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../composables/useAppStore'
import { useDataRefresh } from '../composables/useDataRefresh'
import PageLayout from '../components/PageLayout.vue'

const { t } = useI18n()
const { isDark, themeMode, setTheme } = useAppStore()
const { notifyNotifSettingsChange } = useDataRefresh()

interface NotificationsConfig {
  enabled: boolean
  install_progress: boolean
  system_updates: boolean
  operation_events: boolean
  default_priority: string
  do_not_disturb: boolean
  sound_enabled: boolean
  auto_cleanup_minutes: number
}

interface UserSettings {
  minimize_to_tray: boolean
  proxy_type: string
  proxy_host: string
  proxy_port: number
  notifications: NotificationsConfig
  theme: string
}

// ── Default: ALL settings start OFF ──
const DEFAULT_SETTINGS: UserSettings = {
  minimize_to_tray: false,
  proxy_type: 'none',
  proxy_host: '',
  proxy_port: 0,
  notifications: {
    enabled: false,
    install_progress: false,
    system_updates: false,
    operation_events: false,
    default_priority: 'medium',
    do_not_disturb: false,
    sound_enabled: false,
    auto_cleanup_minutes: 0,
  },
  theme: 'auto',
}

const settings = ref<UserSettings>({ ...DEFAULT_SETTINGS })
const loading = ref(true)

// ── Per-item action state tracking ──
type ActionState = 'idle' | 'saving' | 'saved' | 'error'

type SettingKey = string // supports "minimize_to_tray", "theme", "notifications.enabled", etc.

const actionState = reactive<Record<string, ActionState>>({})

const actionError = reactive<Record<string, string>>({})

// Timers for auto-dismiss "saved" / "error" feedback
const savedTimers: Partial<Record<SettingKey, ReturnType<typeof setTimeout>>> = {}

// ── Options (computed for locale reactivity) ──
// Module-level t() calls are evaluated BEFORE locale messages load.
// Wrapping in computed ensures labels update on locale switch.
const proxyTypeOptions = computed(() => [
  { value: 'none', label: t('settings.proxyTypeNone') },
  { value: 'system', label: t('settings.proxyTypeSystem') },
  { value: 'manual', label: t('settings.proxyTypeManual') },
])

const themeOptions = computed(() => [
  { value: 'auto', label: t('settings.themeAuto') },
  { value: 'dark', label: t('settings.themeDark') },
  { value: 'light', label: t('settings.themeLight') },
])

const priorityOptions = computed(() => [
  { value: 'high', label: t('notifications.priority.high') },
  { value: 'medium', label: t('notifications.priority.medium') },
  { value: 'low', label: t('notifications.priority.low') },
])

const cleanupOptions = computed(() => [
  { value: 0, label: t('settings.notifAutoCleanupNever') },
  { value: 30, label: t('settings.notifAutoCleanup30min') },
  { value: 60, label: t('settings.notifAutoCleanup60min') },
  { value: -1, label: t('settings.notifAutoCleanupCustom') },
])

// Determine which cleanup option button is selected
const selectedCleanupOption = computed(() => {
  const minutes = settings.value.notifications.auto_cleanup_minutes
  if (minutes === 0) return 0
  if (minutes === 30) return 30
  if (minutes === 60) return 60
  return -1 // custom
})

const customCleanupMinutes = ref(120) // default, will be overridden by loaded settings

// ── Custom cleanup apply handler ──
async function handleApplyCustomCleanup() {
  // Set the value into settings and commit
  settings.value.notifications.auto_cleanup_minutes = customCleanupMinutes.value
  commitSetting(notifKey('auto_cleanup_minutes'))
}

async function handleApplyProxyHost() {
  commitSetting('proxy_host')
}

async function handleApplyProxyPort() {
  commitSetting('proxy_port')
}

// ── Client-side validation (mirrors backend, prevents invalid data reaching disk) ──
function validate(): string | null {
  const s = settings.value
  const validProxyTypes = ['none', 'system', 'manual']
  const validThemes = ['auto', 'dark', 'light']

  if (!validProxyTypes.includes(s.proxy_type)) {
    return `Invalid proxy type "${s.proxy_type}"`
  }
  if (!validThemes.includes(s.theme)) {
    return `Invalid theme "${s.theme}"`
  }
  if (s.proxy_type === 'manual') {
    // Allow empty host/port during mode switch; only validate range when values are present
    if (s.proxy_port !== 0 && (s.proxy_port < 1 || s.proxy_port > 65535)) {
      return t('settings.validationPortRange')
    }
    // Host is only required when port is non-zero (meaningful manual config)
    if (s.proxy_host.trim() !== '' && s.proxy_port === 0) {
      return t('settings.validationPortRequired')
    }
  }
  return null
}

// ── Core: persist settings with error recovery (revert on failure) ──
async function commitSetting(key: SettingKey) {
  // Snapshot current state for potential rollback
  const snapshot: UserSettings = { ...settings.value }

  // Client-side validation
  const validationError = validate()
  if (validationError) {
    // Revert to snapshot
    settings.value = snapshot
    actionState[key] = 'error'
    actionError[key] = validationError
    clearSavedTimer(key)
    savedTimers[key] = setTimeout(() => {
      actionState[key] = 'idle'
      actionError[key] = ''
    }, 5000)
    return
  }

  actionState[key] = 'saving'
  actionError[key] = ''
  clearSavedTimer(key)

  try {
    await invoke('save_settings', { settings: settings.value })
    // Success — data is on disk (verified by backend re-read)
    actionState[key] = 'saved'
    savedTimers[key] = setTimeout(() => {
      actionState[key] = 'idle'
    }, 2500)

    // Notify TopBar to refresh enabled notification categories count
    if (key.startsWith('notifications.')) {
      notifyNotifSettingsChange()
    }
  } catch (e: any) {
    // FAILURE — revert UI to the snapshot so user sees what's actually on disk
    settings.value = snapshot
    actionState[key] = 'error'
    actionError[key] = e?.message || String(e)
    savedTimers[key] = setTimeout(() => {
      actionState[key] = 'idle'
      actionError[key] = ''
    }, 5000)
  }
}

function clearSavedTimer(key: SettingKey) {
  if (savedTimers[key]) {
    clearTimeout(savedTimers[key])
    delete savedTimers[key]
  }
}

// ── Toggle handler: flip + save (with revert on failure) ──
function handleToggle(key: SettingKey) {
  if (key.includes('.')) {
    // Nested key like "notifications.install_progress"
    const [parent, child] = key.split('.') as [string, string]
    ;(settings.value as any)[parent][child] = !(settings.value as any)[parent][child]
  } else {
    ;(settings.value as any)[key] = !(settings.value as any)[key]
  }
  commitSetting(key)
}

// ── Notification sub-key helper ──
function notifKey(k: string): string {
  return `notifications.${k}`
}

// ── Notification category items (computed for locale reactivity) ──
interface NotifItem {
  key: string
  stateKey: string
  icon: string
  labelKey: string
  descKey: string
  getValue: () => boolean
}
const notifItems = computed<NotifItem[]>(() => [
  {
    key: 'install',
    stateKey: notifKey('install_progress'),
    icon: 'mdi:download-box-outline',
    labelKey: 'settings.notifInstallProgress',
    descKey: 'settings.notifInstallProgressDesc',
    getValue: () => settings.value.notifications.install_progress,
  },
  {
    key: 'update',
    stateKey: notifKey('system_updates'),
    icon: 'mdi:update',
    labelKey: 'settings.notifSystemUpdates',
    descKey: 'settings.notifSystemUpdatesDesc',
    getValue: () => settings.value.notifications.system_updates,
  },
  {
    key: 'operation',
    stateKey: notifKey('operation_events'),
    icon: 'mdi:cog-outline',
    labelKey: 'settings.notifOperationEvents',
    descKey: 'settings.notifOperationEventsDesc',
    getValue: () => settings.value.notifications.operation_events,
  },
])

// ── Select handler (button group): set + save ──
function handleSelect(key: SettingKey, value: number | string) {
  // ── Custom cleanup minutes ──
  if (key === 'notifications.auto_cleanup_minutes' && value === -1) {
    // Open custom — use the custom value
    ;(settings.value as any).notifications.auto_cleanup_minutes = customCleanupMinutes.value
    commitSetting(key)
    return
  }

  if (key.includes('.')) {
    const [parent, child] = key.split('.') as [string, string]
    if ((settings.value as any)[parent][child] === value) return
    ;(settings.value as any)[parent][child] = value
  } else {
    if ((settings.value as any)[key] === value) return
    ;(settings.value as any)[key] = value
  }

  // Theme changes must apply immediately to the DOM + TopBar
  if (key === 'theme') {
    setTheme(value as 'dark' | 'light' | 'auto')
  }

  commitSetting(key)
}

// ── Input handler: save via dedicated apply button ──
async function handleApplyProxy() {
  commitSetting('proxy_host')
}

// ── Load settings from database → restore saved state ──
async function loadSettings() {
  loading.value = true
  try {
    const data = await invoke<UserSettings>('get_settings')
    // ── Sync theme from the live runtime state (source of truth) ──
    //       tauri-plugin-store holds the actual displayed mode;
    //       overwriting the DB value with it keeps UI and Settings in lock-step.
    const liveMode = themeMode.value || (isDark.value ? 'dark' : 'light')
    settings.value = { ...data, theme: liveMode }

    // ── Initialise custom cleanup input from the loaded settings ──
    const savedMinutes = data.notifications?.auto_cleanup_minutes
    if (savedMinutes && savedMinutes !== 0 && savedMinutes !== 30 && savedMinutes !== 60) {
      customCleanupMinutes.value = savedMinutes
    }
  } catch {
    // DB read failed or data corrupted — fall back to safe defaults
    settings.value = { ...DEFAULT_SETTINGS }
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadSettings()
})

// ── React to TopBar toggle / external theme changes ──
watch([isDark, themeMode], () => {
  const liveMode = themeMode.value
  if (liveMode !== settings.value.theme) {
    settings.value.theme = liveMode
  }
})

// ── Status icon helpers ──
function statusIcon(state: ActionState): string {
  switch (state) {
    case 'saving':
      return 'mdi:loading'
    case 'saved':
      return 'mdi:check-circle'
    case 'error':
      return 'mdi:alert-circle'
    default:
      return ''
  }
}

function statusColor(state: ActionState): string {
  switch (state) {
    case 'saving':
      return 'text-sky-500'
    case 'saved':
      return 'text-emerald-500'
    case 'error':
      return 'text-red-500'
    default:
      return ''
  }
}
</script>

<template>
  <PageLayout :group="t('nav.group.system')" :title="t('settings.title')" :description="t('settings.description')">
    <!-- Loading state -->
    <div v-if="loading" class="flex items-center justify-center py-20">
      <div class="flex items-center gap-3 text-gray-500 dark:text-gray-400">
        <iconify-icon icon="mdi:loading" width="20" class="animate-spin"></iconify-icon>
        <span class="text-sm">{{ t('settings.loading') }}</span>
      </div>
    </div>

    <!-- Settings form -->
    <div v-else class="space-y-5 max-w-2xl">
      <!-- ════════════════════════════════════════════
           SECTION: General — Minimize to tray
           ════════════════════════════════════════════ -->
      <section class="bg-gray-50 dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-5">
        <div class="flex items-center gap-2 mb-5">
          <iconify-icon icon="mdi:cog-outline" width="18" class="text-sky-500 shrink-0"></iconify-icon>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100">{{ t('settings.sections.general') }}</h3>
        </div>

        <!-- Row: Minimize to tray -->
        <div class="flex items-start gap-4 group">
          <!-- Left: label + desc -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <iconify-icon
                icon="mdi:monitor-arrow-down"
                width="16"
                class="text-gray-400 dark:text-gray-500 shrink-0"
              ></iconify-icon>
              <span class="text-sm font-medium text-gray-800 dark:text-gray-200">{{
                t('settings.minimizeToTray')
              }}</span>
            </div>
            <p class="text-xs text-gray-500 dark:text-gray-400 mt-1 ml-6">{{ t('settings.minimizeToTrayDesc') }}</p>
          </div>

          <!-- Right: toggle + status -->
          <div class="flex items-center gap-2.5 shrink-0">
            <!-- Status feedback -->
            <Transition name="status-pop" mode="out-in">
              <div
                v-if="actionState.minimize_to_tray !== 'idle'"
                :key="actionState.minimize_to_tray"
                class="flex items-center gap-1 text-xs"
              >
                <iconify-icon
                  :icon="statusIcon(actionState.minimize_to_tray)"
                  :class="[
                    statusColor(actionState.minimize_to_tray),
                    actionState.minimize_to_tray === 'saving' && 'animate-spin',
                  ]"
                  width="14"
                ></iconify-icon>
                <span :class="statusColor(actionState.minimize_to_tray)">
                  {{
                    actionState.minimize_to_tray === 'saving'
                      ? t('settings.saving')
                      : actionState.minimize_to_tray === 'saved'
                      ? t('settings.saved')
                      : actionError.minimize_to_tray
                  }}
                </span>
              </div>
            </Transition>

            <!-- Toggle switch -->
            <button
              type="button"
              role="switch"
              :aria-checked="settings.minimize_to_tray"
              :class="[
                'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-400',
                settings.minimize_to_tray ? 'bg-sky-600' : 'bg-gray-300 dark:bg-gray-600',
              ]"
              @click="handleToggle('minimize_to_tray')"
            >
              <span
                :class="[
                  'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
                  settings.minimize_to_tray ? 'translate-x-5' : 'translate-x-0',
                ]"
              ></span>
            </button>
          </div>
        </div>
      </section>

      <!-- ════════════════════════════════════════════
           SECTION: Proxy
           ════════════════════════════════════════════ -->
      <section class="bg-gray-50 dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-5">
        <div class="flex items-center gap-2 mb-5">
          <iconify-icon icon="mdi:earth" width="18" class="text-emerald-500 shrink-0"></iconify-icon>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100">{{ t('settings.sections.proxy') }}</h3>
        </div>

        <div class="space-y-5">
          <!-- Row: Proxy type -->
          <div>
            <div class="flex items-center gap-2 mb-3">
              <iconify-icon
                icon="mdi:lan-connect"
                width="16"
                class="text-gray-400 dark:text-gray-500 shrink-0"
              ></iconify-icon>
              <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ t('settings.proxyType') }}</span>

              <!-- Inline status -->
              <Transition name="status-pop" mode="out-in">
                <div
                  v-if="actionState.proxy_type !== 'idle'"
                  :key="actionState.proxy_type"
                  class="flex items-center gap-1 text-xs ml-1"
                >
                  <iconify-icon
                    :icon="statusIcon(actionState.proxy_type)"
                    :class="[
                      statusColor(actionState.proxy_type),
                      actionState.proxy_type === 'saving' && 'animate-spin',
                    ]"
                    width="13"
                  ></iconify-icon>
                  <span :class="statusColor(actionState.proxy_type)">
                    {{
                      actionState.proxy_type === 'saving'
                        ? t('settings.saving')
                        : actionState.proxy_type === 'saved'
                        ? t('settings.saved')
                        : actionError.proxy_type
                    }}
                  </span>
                </div>
              </Transition>
            </div>

            <!-- Button group with individual bindings -->
            <div class="flex gap-2 flex-wrap">
              <button
                v-for="opt in proxyTypeOptions"
                :key="opt.value"
                type="button"
                :class="[
                  'relative px-4 py-2 rounded-lg text-sm font-medium transition-all duration-150 cursor-pointer',
                  settings.proxy_type === opt.value
                    ? 'bg-sky-600 text-white shadow-sm'
                    : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border border-gray-300 dark:border-gray-600 hover:border-sky-400 dark:hover:border-sky-500 hover:text-gray-900 dark:hover:text-gray-200',
                ]"
                @click="handleSelect('proxy_type', opt.value)"
              >
                <!-- Selected indicator dot -->
                <span
                  v-if="settings.proxy_type === opt.value"
                  class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-white/80"
                ></span>
                {{ opt.label }}
              </button>
            </div>
          </div>

          <!-- Row: Manual proxy address + port (collapsible) -->
          <Transition name="slide-fade">
            <div v-if="settings.proxy_type === 'manual'" class="space-y-4 pt-2">
              <!-- Proxy host -->
              <div>
                <div class="flex items-center gap-2 mb-2">
                  <iconify-icon
                    icon="mdi:server"
                    width="16"
                    class="text-gray-400 dark:text-gray-500 shrink-0"
                  ></iconify-icon>
                  <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{{
                    t('settings.serverAddress')
                  }}</span>
                </div>
                <div class="flex items-center gap-2">
                  <input
                    v-model="settings.proxy_host"
                    type="text"
                    :placeholder="t('settings.serverAddressPlaceholder')"
                    class="flex-1 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500 focus:border-transparent transition-colors"
                  />
                  <!-- Dedicated Apply button -->
                  <div class="flex items-center gap-1.5">
                    <Transition name="status-pop" mode="out-in">
                      <div
                        v-if="actionState.proxy_host !== 'idle'"
                        :key="actionState.proxy_host"
                        class="flex items-center gap-1 text-xs whitespace-nowrap"
                      >
                        <iconify-icon
                          :icon="statusIcon(actionState.proxy_host)"
                          :class="[
                            statusColor(actionState.proxy_host),
                            actionState.proxy_host === 'saving' && 'animate-spin',
                          ]"
                          width="13"
                        ></iconify-icon>
                        <span :class="statusColor(actionState.proxy_host)">
                          {{
                            actionState.proxy_host === 'saving'
                              ? t('settings.saving')
                              : actionState.proxy_host === 'saved'
                              ? t('settings.saved')
                              : actionError.proxy_host
                          }}
                        </span>
                      </div>
                    </Transition>
                    <button
                      type="button"
                      :disabled="actionState.proxy_host === 'saving'"
                      class="px-3.5 py-2 rounded-lg text-xs font-semibold transition-all duration-150 cursor-pointer bg-sky-600 hover:bg-sky-500 text-white disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-400"
                      @click="handleApplyProxyHost()"
                    >
                      {{ t('settings.apply') }}
                    </button>
                  </div>
                </div>
              </div>

              <!-- Proxy port -->
              <div>
                <div class="flex items-center gap-2 mb-2">
                  <iconify-icon
                    icon="mdi:numeric"
                    width="16"
                    class="text-gray-400 dark:text-gray-500 shrink-0"
                  ></iconify-icon>
                  <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ t('settings.port') }}</span>
                </div>
                <div class="flex items-center gap-2">
                  <input
                    v-model.number="settings.proxy_port"
                    type="number"
                    min="1"
                    max="65535"
                    :placeholder="t('settings.portPlaceholder')"
                    class="flex-1 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500 focus:border-transparent transition-colors"
                  />
                  <div class="flex items-center gap-1.5">
                    <Transition name="status-pop" mode="out-in">
                      <div
                        v-if="actionState.proxy_port !== 'idle'"
                        :key="actionState.proxy_port"
                        class="flex items-center gap-1 text-xs whitespace-nowrap"
                      >
                        <iconify-icon
                          :icon="statusIcon(actionState.proxy_port)"
                          :class="[
                            statusColor(actionState.proxy_port),
                            actionState.proxy_port === 'saving' && 'animate-spin',
                          ]"
                          width="13"
                        ></iconify-icon>
                        <span :class="statusColor(actionState.proxy_port)">
                          {{
                            actionState.proxy_port === 'saving'
                              ? t('settings.saving')
                              : actionState.proxy_port === 'saved'
                              ? t('settings.saved')
                              : actionError.proxy_port
                          }}
                        </span>
                      </div>
                    </Transition>
                    <button
                      type="button"
                      :disabled="actionState.proxy_port === 'saving'"
                      class="px-3.5 py-2 rounded-lg text-xs font-semibold transition-all duration-150 cursor-pointer bg-sky-600 hover:bg-sky-500 text-white disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-400"
                      @click="handleApplyProxyPort()"
                    >
                      {{ t('settings.apply') }}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </Transition>

          <!-- Proxy info note -->
          <p class="text-xs text-gray-500 dark:text-gray-400 flex items-start gap-1.5 pt-1">
            <iconify-icon icon="mdi:information-outline" width="14" class="shrink-0 mt-0.5"></iconify-icon>
            {{ t('settings.proxyNote') }}
          </p>
        </div>
      </section>

      <!-- ════════════════════════════════════════════
           SECTION: Notifications
           ════════════════════════════════════════════ -->
      <section class="bg-gray-50 dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-5">
        <div class="flex items-center gap-2 mb-5">
          <iconify-icon icon="mdi:bell-outline" width="18" class="text-amber-500 shrink-0"></iconify-icon>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100">
            {{ t('settings.sections.notifications') }}
          </h3>
        </div>

        <!-- Master toggle -->
        <div class="flex items-start gap-4">
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <iconify-icon
                icon="mdi:bell-ring-outline"
                width="16"
                class="text-gray-400 dark:text-gray-500 shrink-0"
              ></iconify-icon>
              <span class="text-sm font-medium text-gray-800 dark:text-gray-200">{{
                t('settings.notificationsMaster')
              }}</span>
            </div>
            <p class="text-xs text-gray-500 dark:text-gray-400 mt-1 ml-6">
              {{ t('settings.notificationsMasterDesc') }}
            </p>
          </div>
          <div class="flex items-center gap-2.5 shrink-0">
            <Transition name="status-pop" mode="out-in">
              <div
                v-if="(actionState['notifications.enabled'] || 'idle') !== 'idle'"
                class="flex items-center gap-1 text-xs"
              >
                <iconify-icon
                  :icon="statusIcon(actionState['notifications.enabled'] || 'idle')"
                  :class="[
                    statusColor(actionState['notifications.enabled'] || 'idle'),
                    (actionState['notifications.enabled'] || '') === 'saving' && 'animate-spin',
                  ]"
                  width="14"
                ></iconify-icon>
                <span :class="statusColor(actionState['notifications.enabled'] || 'idle')">
                  {{
                    (actionState['notifications.enabled'] || '') === 'saving'
                      ? t('settings.saving')
                      : (actionState['notifications.enabled'] || '') === 'saved'
                      ? t('settings.saved')
                      : actionError['notifications.enabled']
                  }}
                </span>
              </div>
            </Transition>
            <button
              type="button"
              role="switch"
              :aria-checked="settings.notifications.enabled"
              :class="[
                'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-400',
                settings.notifications.enabled ? 'bg-sky-600' : 'bg-gray-300 dark:bg-gray-600',
              ]"
              @click="handleToggle(notifKey('enabled'))"
            >
              <span
                :class="[
                  'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
                  settings.notifications.enabled ? 'translate-x-5' : 'translate-x-0',
                ]"
              ></span>
            </button>
          </div>
        </div>

        <!-- Expanded notification settings -->
        <div v-if="settings.notifications.enabled" class="mt-5 ml-2 pl-4 space-y-4 border-l-2 border-amber-500/20">
          <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">{{ t('settings.notificationsCategories') }}</p>

          <!-- ── Per-category toggles ── -->
          <div v-for="item in notifItems" :key="item.key" class="flex items-start gap-4">
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <iconify-icon
                  :icon="item.icon"
                  width="16"
                  class="text-gray-400 dark:text-gray-500 shrink-0"
                ></iconify-icon>
                <span class="text-sm font-medium text-gray-800 dark:text-gray-200">{{ t(item.labelKey) }}</span>
              </div>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1 ml-6">{{ t(item.descKey) }}</p>
            </div>
            <div class="flex items-center gap-2.5 shrink-0">
              <Transition name="status-pop" mode="out-in">
                <div v-if="(actionState[item.stateKey] || 'idle') !== 'idle'" class="flex items-center gap-1 text-xs">
                  <iconify-icon
                    :icon="statusIcon(actionState[item.stateKey] || 'idle')"
                    :class="[
                      statusColor(actionState[item.stateKey] || 'idle'),
                      (actionState[item.stateKey] || '') === 'saving' && 'animate-spin',
                    ]"
                    width="14"
                  ></iconify-icon>
                  <span :class="statusColor(actionState[item.stateKey] || 'idle')">
                    {{
                      (actionState[item.stateKey] || '') === 'saving'
                        ? t('settings.saving')
                        : (actionState[item.stateKey] || '') === 'saved'
                        ? t('settings.saved')
                        : actionError[item.stateKey]
                    }}
                  </span>
                </div>
              </Transition>
              <button
                type="button"
                role="switch"
                :aria-checked="item.getValue()"
                :class="[
                  'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-400',
                  item.getValue() ? 'bg-sky-600' : 'bg-gray-300 dark:bg-gray-600',
                ]"
                @click="handleToggle(item.stateKey)"
              >
                <span
                  :class="[
                    'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
                    item.getValue() ? 'translate-x-5' : 'translate-x-0',
                  ]"
                ></span>
              </button>
            </div>
          </div>

          <!-- ── Divider ── -->
          <div class="border-t border-gray-200 dark:border-gray-700 my-2"></div>

          <!-- ── Do-not-disturb ── -->
          <div class="flex items-start gap-4">
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <iconify-icon
                  icon="mdi:sleep"
                  width="16"
                  class="text-gray-400 dark:text-gray-500 shrink-0"
                ></iconify-icon>
                <span class="text-sm font-medium text-gray-800 dark:text-gray-200">{{ t('settings.notifDnd') }}</span>
              </div>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1 ml-6">{{ t('settings.notifDndDesc') }}</p>
            </div>
            <div class="flex items-center gap-2.5 shrink-0">
              <Transition name="status-pop" mode="out-in">
                <div
                  v-if="(actionState[notifKey('do_not_disturb')] || 'idle') !== 'idle'"
                  class="flex items-center gap-1 text-xs"
                >
                  <iconify-icon
                    :icon="statusIcon(actionState[notifKey('do_not_disturb')] || 'idle')"
                    :class="[
                      statusColor(actionState[notifKey('do_not_disturb')] || 'idle'),
                      (actionState[notifKey('do_not_disturb')] || '') === 'saving' && 'animate-spin',
                    ]"
                    width="14"
                  ></iconify-icon>
                  <span :class="statusColor(actionState[notifKey('do_not_disturb')] || 'idle')">
                    {{
                      (actionState[notifKey('do_not_disturb')] || '') === 'saving'
                        ? t('settings.saving')
                        : (actionState[notifKey('do_not_disturb')] || '') === 'saved'
                        ? t('settings.saved')
                        : actionError[notifKey('do_not_disturb')]
                    }}
                  </span>
                </div>
              </Transition>
              <button
                type="button"
                role="switch"
                :aria-checked="settings.notifications.do_not_disturb"
                :class="[
                  'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-400',
                  settings.notifications.do_not_disturb ? 'bg-sky-600' : 'bg-gray-300 dark:bg-gray-600',
                ]"
                @click="handleToggle(notifKey('do_not_disturb'))"
              >
                <span
                  :class="[
                    'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
                    settings.notifications.do_not_disturb ? 'translate-x-5' : 'translate-x-0',
                  ]"
                ></span>
              </button>
            </div>
          </div>

          <!-- ── Divider ── -->
          <div class="border-t border-gray-200 dark:border-gray-700 my-2"></div>

          <!-- ── Default priority ── -->
          <div>
            <div class="flex items-start gap-4 mb-3">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <iconify-icon
                    icon="mdi:flag-outline"
                    width="16"
                    class="text-gray-400 dark:text-gray-500 shrink-0"
                  ></iconify-icon>
                  <span class="text-sm font-medium text-gray-800 dark:text-gray-200">{{
                    t('settings.notifDefaultPriority')
                  }}</span>
                </div>
              </div>
              <div class="flex items-center gap-2.5 shrink-0">
                <Transition name="status-pop" mode="out-in">
                  <div
                    v-if="(actionState[notifKey('default_priority')] || 'idle') !== 'idle'"
                    class="flex items-center gap-1 text-xs"
                  >
                    <iconify-icon
                      :icon="statusIcon(actionState[notifKey('default_priority')] || 'idle')"
                      :class="[
                        statusColor(actionState[notifKey('default_priority')] || 'idle'),
                        (actionState[notifKey('default_priority')] || '') === 'saving' && 'animate-spin',
                      ]"
                      width="14"
                    ></iconify-icon>
                    <span :class="statusColor(actionState[notifKey('default_priority')] || 'idle')">
                      {{
                        (actionState[notifKey('default_priority')] || '') === 'saving'
                          ? t('settings.saving')
                          : (actionState[notifKey('default_priority')] || '') === 'saved'
                          ? t('settings.saved')
                          : actionError[notifKey('default_priority')]
                      }}
                    </span>
                  </div>
                </Transition>
              </div>
            </div>
            <div class="flex gap-2 flex-wrap ml-6">
              <button
                v-for="opt in priorityOptions"
                :key="opt.value"
                type="button"
                :class="[
                  'relative px-4 py-2 rounded-lg text-sm font-medium transition-all duration-150 cursor-pointer',
                  settings.notifications.default_priority === opt.value
                    ? 'bg-sky-600 text-white shadow-sm'
                    : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border border-gray-300 dark:border-gray-600 hover:border-sky-400 dark:hover:border-sky-500 hover:text-gray-900 dark:hover:text-gray-200',
                ]"
                @click="handleSelect(notifKey('default_priority'), opt.value)"
              >
                <span
                  v-if="settings.notifications.default_priority === opt.value"
                  class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-white/80"
                ></span>
                {{ opt.label }}
              </button>
            </div>
          </div>

          <!-- ── Sound enabled ── -->
          <div class="flex items-start gap-4">
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <iconify-icon
                  icon="mdi:volume-high"
                  width="16"
                  class="text-gray-400 dark:text-gray-500 shrink-0"
                ></iconify-icon>
                <span class="text-sm font-medium text-gray-800 dark:text-gray-200">{{
                  t('settings.notifSoundEnabled')
                }}</span>
              </div>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1 ml-6">
                {{ t('settings.notifSoundEnabledDesc') }}
              </p>
            </div>
            <div class="flex items-center gap-2.5 shrink-0">
              <Transition name="status-pop" mode="out-in">
                <div
                  v-if="(actionState[notifKey('sound_enabled')] || 'idle') !== 'idle'"
                  class="flex items-center gap-1 text-xs"
                >
                  <iconify-icon
                    :icon="statusIcon(actionState[notifKey('sound_enabled')] || 'idle')"
                    :class="[
                      statusColor(actionState[notifKey('sound_enabled')] || 'idle'),
                      (actionState[notifKey('sound_enabled')] || '') === 'saving' && 'animate-spin',
                    ]"
                    width="14"
                  ></iconify-icon>
                  <span :class="statusColor(actionState[notifKey('sound_enabled')] || 'idle')">
                    {{
                      (actionState[notifKey('sound_enabled')] || '') === 'saving'
                        ? t('settings.saving')
                        : (actionState[notifKey('sound_enabled')] || '') === 'saved'
                        ? t('settings.saved')
                        : actionError[notifKey('sound_enabled')]
                    }}
                  </span>
                </div>
              </Transition>
              <button
                type="button"
                role="switch"
                :aria-checked="settings.notifications.sound_enabled"
                :class="[
                  'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-400',
                  settings.notifications.sound_enabled ? 'bg-sky-600' : 'bg-gray-300 dark:bg-gray-600',
                ]"
                @click="handleToggle(notifKey('sound_enabled'))"
              >
                <span
                  :class="[
                    'pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out',
                    settings.notifications.sound_enabled ? 'translate-x-5' : 'translate-x-0',
                  ]"
                ></span>
              </button>
            </div>
          </div>

          <!-- ── Auto-cleanup ── -->
          <div>
            <div class="flex items-start gap-4 mb-3">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <iconify-icon
                    icon="mdi:broom"
                    width="16"
                    class="text-gray-400 dark:text-gray-500 shrink-0"
                  ></iconify-icon>
                  <span class="text-sm font-medium text-gray-800 dark:text-gray-200">{{
                    t('settings.notifAutoCleanup')
                  }}</span>
                </div>
                <p class="text-xs text-gray-500 dark:text-gray-400 mt-1 ml-6">
                  {{ t('settings.notifAutoCleanupDesc') }}
                </p>
              </div>
              <div class="flex items-center gap-2.5 shrink-0">
                <Transition name="status-pop" mode="out-in">
                  <div
                    v-if="(actionState[notifKey('auto_cleanup_minutes')] || 'idle') !== 'idle'"
                    class="flex items-center gap-1 text-xs"
                  >
                    <iconify-icon
                      :icon="statusIcon(actionState[notifKey('auto_cleanup_minutes')] || 'idle')"
                      :class="[
                        statusColor(actionState[notifKey('auto_cleanup_minutes')] || 'idle'),
                        (actionState[notifKey('auto_cleanup_minutes')] || '') === 'saving' && 'animate-spin',
                      ]"
                      width="14"
                    ></iconify-icon>
                    <span :class="statusColor(actionState[notifKey('auto_cleanup_minutes')] || 'idle')">
                      {{
                        (actionState[notifKey('auto_cleanup_minutes')] || '') === 'saving'
                          ? t('settings.saving')
                          : (actionState[notifKey('auto_cleanup_minutes')] || '') === 'saved'
                          ? t('settings.saved')
                          : actionError[notifKey('auto_cleanup_minutes')]
                      }}
                    </span>
                  </div>
                </Transition>
              </div>
            </div>
            <div class="flex gap-2 flex-wrap ml-6">
              <button
                v-for="opt in cleanupOptions"
                :key="opt.value"
                type="button"
                :class="[
                  'relative px-4 py-2 rounded-lg text-sm font-medium transition-all duration-150 cursor-pointer',
                  selectedCleanupOption === opt.value
                    ? 'bg-sky-600 text-white shadow-sm'
                    : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border border-gray-300 dark:border-gray-600 hover:border-sky-400 dark:hover:border-sky-500 hover:text-gray-900 dark:hover:text-gray-200',
                ]"
                @click="handleSelect(notifKey('auto_cleanup_minutes'), opt.value)"
              >
                <span
                  v-if="selectedCleanupOption === opt.value"
                  class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-white/80"
                ></span>
                {{ opt.label }}
              </button>
            </div>

            <!-- Custom minutes input (shown when "Custom" is selected) -->
            <div v-if="selectedCleanupOption === -1" class="flex items-center gap-2 ml-6 mt-3">
              <input
                v-model.number="customCleanupMinutes"
                type="number"
                min="1"
                max="525600"
                class="w-28 px-3 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-sky-500 focus:border-transparent transition-colors"
              />
              <span class="text-sm text-gray-500 dark:text-gray-400">{{ t('settings.minutes') }}</span>
              <button
                type="button"
                class="px-3 py-2 rounded-lg bg-sky-600 text-white text-sm font-medium hover:bg-sky-700 transition-colors flex items-center gap-1.5 disabled:opacity-50"
                :disabled="actionState[notifKey('auto_cleanup_minutes')] === 'saving'"
                @click="handleApplyCustomCleanup()"
              >
                <span
                  v-if="actionState[notifKey('auto_cleanup_minutes')] === 'saving'"
                  class="i-mdi:loading w-3.5 h-3.5 animate-spin"
                />
                {{ t('settings.apply') }}
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- ════════════════════════════════════════════
           SECTION: Appearance
           ════════════════════════════════════════════ -->
      <section class="bg-gray-50 dark:bg-gray-900 rounded-xl border border-gray-200 dark:border-gray-800 p-5">
        <div class="flex items-center gap-2 mb-5">
          <iconify-icon icon="mdi:palette-outline" width="18" class="text-violet-500 shrink-0"></iconify-icon>
          <h3 class="text-sm font-semibold text-gray-900 dark:text-gray-100">
            {{ t('settings.sections.appearance') }}
          </h3>
        </div>

        <div class="space-y-4">
          <div>
            <div class="flex items-center gap-2 mb-3">
              <iconify-icon
                icon="mdi:theme-light-dark"
                width="16"
                class="text-gray-400 dark:text-gray-500 shrink-0"
              ></iconify-icon>
              <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ t('settings.theme') }}</span>

              <Transition name="status-pop" mode="out-in">
                <div
                  v-if="actionState.theme !== 'idle'"
                  :key="actionState.theme"
                  class="flex items-center gap-1 text-xs ml-1"
                >
                  <iconify-icon
                    :icon="statusIcon(actionState.theme)"
                    :class="[statusColor(actionState.theme), actionState.theme === 'saving' && 'animate-spin']"
                    width="13"
                  ></iconify-icon>
                  <span :class="statusColor(actionState.theme)">
                    {{
                      actionState.theme === 'saving'
                        ? t('settings.saving')
                        : actionState.theme === 'saved'
                        ? t('settings.saved')
                        : actionError.theme
                    }}
                  </span>
                </div>
              </Transition>
            </div>

            <div class="flex gap-2 flex-wrap">
              <button
                v-for="opt in themeOptions"
                :key="opt.value"
                type="button"
                :class="[
                  'relative px-4 py-2 rounded-lg text-sm font-medium transition-all duration-150 cursor-pointer',
                  settings.theme === opt.value
                    ? 'bg-sky-600 text-white shadow-sm'
                    : 'bg-white dark:bg-gray-800 text-gray-600 dark:text-gray-400 border border-gray-300 dark:border-gray-600 hover:border-sky-400 dark:hover:border-sky-500 hover:text-gray-900 dark:hover:text-gray-200',
                ]"
                @click="handleSelect('theme', opt.value)"
              >
                <span
                  v-if="settings.theme === opt.value"
                  class="absolute top-1 right-1 w-1.5 h-1.5 rounded-full bg-white/80"
                ></span>
                {{ opt.label }}
              </button>
            </div>
          </div>

          <p class="text-xs text-gray-500 dark:text-gray-400 flex items-start gap-1.5">
            <iconify-icon icon="mdi:information-outline" width="14" class="shrink-0 mt-0.5"></iconify-icon>
            {{ t('settings.themeDesc') }}
          </p>
        </div>
      </section>
    </div>
  </PageLayout>
</template>

<style scoped>
/* ── Slide-fade for collapsible sections (manual proxy) ── */
.slide-fade-enter-active {
  transition: all 0.3s ease-out;
}
.slide-fade-leave-active {
  transition: all 0.2s ease-in;
}
.slide-fade-enter-from {
  opacity: 0;
  transform: translateY(-10px);
}
.slide-fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

/* ── Status indicator pop animation ── */
.status-pop-enter-active {
  transition: all 0.2s ease-out;
}
.status-pop-leave-active {
  transition: all 0.15s ease-in;
}
.status-pop-enter-from {
  opacity: 0;
  transform: scale(0.8);
}
.status-pop-leave-to {
  opacity: 0;
  transform: scale(0.8);
}
</style>

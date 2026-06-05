<script setup lang="ts">
import { ref, shallowRef, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import PageLayout from '../components/PageLayout.vue'
import SearchInput from '../components/SearchInput.vue'
import { useDataRefresh } from '../composables/useDataRefresh'
import { useResponsiveListHeight } from '../composables/useResponsiveListHeight'

const { t } = useI18n()
const router = useRouter()
const { notifyNotificationChange, onNotifSettingsChange } = useDataRefresh()

// ── Types ──

interface Notification {
  id: number
  category: string
  priority: string
  title: string
  body: string
  notif_key: string | null
  params_json: string | null
  action_route: string | null
  is_read: boolean
  sound_enabled: boolean
  default_priority: string
  created_at: number
}

// ── State ──

const PAGE_SIZE = 10
const loading = ref(true)
const loadingMore = ref(false)
const notifications = shallowRef<Notification[]>([])  // P1: shallowRef 避免深度响应式代理开销
const totalCount = ref(0)
const searchQuery = ref('')
const filterCategory = ref('all')
const sortOrder = ref<'newest' | 'oldest'>('newest')
const error = ref('')
const confirmDialog = ref({
  open: false,
  title: '',
  message: '',
  onConfirm: () => {},
})

// Responsive list height: nav(56) + pageHeader(56) + filters(60) + buffer(80)
const { listHeight, listContainerRef } = useResponsiveListHeight({
  filters: 60,
  buffer: 80,
})

/** Whether there are more notifications to load. */
const hasMore = computed(() => notifications.value.length < totalCount.value)

// ── i18n message resolution ──

/** Resolve the display title and body for a notification.
 *  When `notif_key` is present, uses vue-i18n with the params from `params_json`.
 *  Falls back to raw `title`/`body` for legacy notifications. */
function resolveMessage(n: Notification): { title: string; body: string } {
  if (!n.notif_key) {
    // Legacy notification — use raw title/body
    return { title: n.title, body: n.body }
  }
  try {
    const keyBase = `notifications.messages.${n.notif_key}`
    const params = n.params_json ? (JSON.parse(n.params_json) as Record<string, string>) : {}
    return {
      title: t(`${keyBase}.title`),
      body: t(`${keyBase}.body`, params),
    }
  } catch {
    // Fallback: if i18n key is missing or JSON parse fails, show raw title/body
    return { title: n.title || n.notif_key, body: n.body || '' }
  }
}

// ── Computed ──

const categories = [
  { value: 'all', label: () => t('notifications.filters.all') },
  { value: 'install', label: () => t('notifications.categories.install') },
  { value: 'update', label: () => t('notifications.categories.update') },
  { value: 'operation', label: () => t('notifications.categories.operation') },
]

const filteredNotifications = computed(() => {
  let result = [...notifications.value]

  if (filterCategory.value !== 'all') {
    const f = filterCategory.value.toLowerCase()
    result = result.filter(n => (n.category || '').toLowerCase() === f)
  }

  const q = searchQuery.value.trim().toLowerCase()
  if (q) {
    result = result.filter(n => {
      const msg = resolveMessage(n)
      return msg.title.toLowerCase().includes(q) || msg.body.toLowerCase().includes(q)
    })
  }

  if (sortOrder.value === 'oldest') {
    result.reverse()
  }
  // Default: newest first (list_notifications already returns this)

  return result
})

const unreadCount = computed(() => notifications.value.filter(n => !n.is_read).length)
const hasSelection = computed(() => {
  // Only consider notifications that match the current category filter
  const visible =
    filterCategory.value === 'all'
      ? notifications.value
      : notifications.value.filter(n => (n.category || '').toLowerCase() === filterCategory.value.toLowerCase())
  return visible.some(n => !n.is_read)
})

// ── Actions ──

async function loadNotifications() {
  loading.value = true
  error.value = ''
  try {
    const [list, count] = await Promise.all([
      invoke<Notification[]>('notify_list', { limit: PAGE_SIZE, offset: 0 }),
      invoke<number>('notify_count'),
    ])
    notifications.value = list
    totalCount.value = count
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function loadMore() {
  if (loadingMore.value || !hasMore.value) return
  loadingMore.value = true
  try {
    const more = await invoke<Notification[]>('notify_list', {
      limit: PAGE_SIZE,
      offset: notifications.value.length,
    })
    // P1: Replace entire array to trigger shallowRef reactivity
    notifications.value = [...notifications.value, ...more]
    // Refresh total count in case new notifications arrived
    totalCount.value = await invoke<number>('notify_count')
  } catch (e) {
    console.error('[NotificationCenter] loadMore failed:', e)
  } finally {
    loadingMore.value = false
  }
}

async function markRead(id: number) {
  try {
    await invoke('notify_mark_read', { id })
    // P1: Replace entire array to trigger shallowRef reactivity
    notifications.value = notifications.value.map(n =>
      n.id === id ? { ...n, is_read: true } : n
    )
    notifyNotificationChange()
  } catch (e) {
    console.error('[NotificationCenter] markRead failed:', e)
  }
}

async function markUnread(id: number) {
  try {
    await invoke('notify_mark_unread', { id })
    // P1: Replace entire array to trigger shallowRef reactivity
    notifications.value = notifications.value.map(n =>
      n.id === id ? { ...n, is_read: false } : n
    )
    notifyNotificationChange()
  } catch (e) {
    console.error('[NotificationCenter] markUnread failed:', e)
  }
}

async function markAllRead() {
  // Only mark visible (filtered) notifications as read
  const visible =
    filterCategory.value === 'all'
      ? notifications.value
      : notifications.value.filter(n => (n.category || '').toLowerCase() === filterCategory.value.toLowerCase())
  for (const n of visible) {
    if (!n.is_read) await markRead(n.id)
  }
  // Reload from backend for consistency
  await loadNotifications()
  // Notify TopBar to refresh its unread count
  notifyNotificationChange()
}

async function deleteReadBefore() {
  // Delete read notifications older than 7 days
  const cutoff = Date.now()
  try {
    const deleted = await invoke<number>('notification_delete_read_before', { beforeMs: cutoff })
    if (deleted > 0) {
      await loadNotifications()
      notifyNotificationChange()
    }
  } catch (e) {
    console.error('[NotificationCenter] deleteReadBefore failed:', e)
  }
}

async function refreshUnreadCount() {
  try {
    const count = await invoke<number>('notify_unread_count')
    // Only used for global badge (future), local unreadCount computed remains the source of truth
    return count
  } catch {
    return 0
  }
}

async function deleteNotification(id: number) {
  try {
    await invoke('notify_delete', { id })
    notifications.value = notifications.value.filter(x => x.id !== id)
    notifyNotificationChange()
  } catch {
    /* ignore */
  }
}

function confirmDeleteAll() {
  if (notifications.value.length === 0) return
  confirmDialog.value = {
    open: true,
    title: t('notifications.actions.deleteAll'),
    message: t('notifications.actions.deleteAllConfirm', { count: notifications.value.length }),
    onConfirm: async () => {
      try {
        await invoke('notify_delete_all')
        notifications.value = []
        notifyNotificationChange()
      } catch {
        /* ignore */
      }
      confirmDialog.value.open = false
    },
  }
}

async function deleteAll() {
  confirmDeleteAll()
}

async function handleClickNotification(n: Notification) {
  if (!n.is_read) {
    await markRead(n.id)
  }
  if (n.action_route) {
    router.push(n.action_route)
  }
}

function priorityClass(p: string) {
  return p === 'high'
    ? 'bg-red-500/20 text-red-400'
    : p === 'medium'
    ? 'bg-amber-500/20 text-amber-400'
    : 'bg-slate-500/20 text-slate-400'
}

function categoryIcon(c: string): string {
  const lc = c.toLowerCase()
  return lc === 'install' ? 'mdi:download-box-outline' : lc === 'update' ? 'mdi:update' : 'mdi:cog-outline'
}

function categoryClass(c: string) {
  const lc = c.toLowerCase()
  return lc === 'install' ? 'text-emerald-400' : lc === 'update' ? 'text-cyan-400' : 'text-violet-400'
}

function formatTime(ts: number) {
  const d = new Date(ts)
  const now = new Date()
  const diff = now.getTime() - d.getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 1) return t('notifications.time.justNow')
  if (mins < 60) return t('notifications.time.minsAgo', { n: mins })
  const hours = Math.floor(mins / 60)
  if (hours < 24) return t('notifications.time.hoursAgo', { n: hours })
  const days = Math.floor(hours / 24)
  if (days < 7) return t('notifications.time.daysAgo', { n: days })
  return d.toLocaleDateString()
}

let notifSettingsWatchStop: (() => void) | null = null

onMounted(() => {
  loadNotifications()
  startRealtimeListener()
  // Watch for notification settings changes (e.g., auto-cleanup triggered)
  notifSettingsWatchStop = onNotifSettingsChange(() => {
    loadNotifications()
  })
})

onBeforeUnmount(() => {
  if (unlistenNotif) {
    unlistenNotif()
    unlistenNotif = null
  }
  if (unlistenCleanup) {
    unlistenCleanup()
    unlistenCleanup = null
  }
  if (notifSettingsWatchStop) {
    notifSettingsWatchStop()
    notifSettingsWatchStop = null
  }
  // P3: Close AudioContext to release audio resources
  if (audioCtx) {
    audioCtx.close()
    audioCtx = null
  }
})

// ── Notification sound (Web Audio API) ──
let audioCtx: AudioContext | null = null

function playNotificationSound(): void {
  try {
    if (!audioCtx) {
      audioCtx = new AudioContext()
    }
    const osc = audioCtx.createOscillator()
    const gain = audioCtx.createGain()
    osc.connect(gain)
    gain.connect(audioCtx.destination)
    osc.type = 'sine'
    // Two-tone chime: 880 Hz → 1047 Hz
    osc.frequency.setValueAtTime(880, audioCtx.currentTime)
    osc.frequency.setValueAtTime(1047, audioCtx.currentTime + 0.1)
    gain.gain.setValueAtTime(0.15, audioCtx.currentTime)
    gain.gain.exponentialRampToValueAtTime(0.01, audioCtx.currentTime + 0.3)
    osc.start(audioCtx.currentTime)
    osc.stop(audioCtx.currentTime + 0.3)
  } catch {
    // Sound playback is best-effort; silently ignore failures
  }
}

// ── Real-time listener ──
let unlistenNotif: UnlistenFn | null = null
let unlistenCleanup: UnlistenFn | null = null

async function startRealtimeListener() {
  try {
    unlistenNotif = await listen<Notification>('notification:new', event => {
      // P1: Replace entire array to trigger shallowRef reactivity
      notifications.value = [event.payload, ...notifications.value]
      totalCount.value++
      // Play notification sound if enabled by user
      if (event.payload.sound_enabled) {
        playNotificationSound()
      }
    })
    // Listen for auto-cleanup events from the backend
    unlistenCleanup = await listen<number>('notification:cleanup', () => {
      loadNotifications()
      notifyNotificationChange()
    })
  } catch {
    // Listener setup failed — non-critical, polling still works
  }
}
</script>

<template>
  <PageLayout :group="t('nav.group.system')" :title="t('notifications.title')">
    <template #filters>
      <!-- Search -->
      <SearchInput
        v-model="searchQuery"
        :placeholder="t('notifications.filters.searchPlaceholder')"
        class="flex-1 min-w-[200px] max-w-[320px]"
      />

      <!-- Category filter -->
      <select
        v-model="filterCategory"
        class="px-3 py-2 rounded-lg bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-700 text-sm text-gray-700 dark:text-slate-300 focus:outline-none focus:border-sky-500/50 cursor-pointer"
      >
        <option v-for="cat in categories" :key="cat.value" :value="cat.value">
          {{ cat.label() }}
        </option>
      </select>

      <!-- Sort -->
      <button
        class="px-3 py-2 rounded-lg bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-700 text-sm text-gray-700 dark:text-slate-300 hover:border-sky-500/50 transition-colors flex items-center gap-1.5"
        @click="sortOrder = sortOrder === 'newest' ? 'oldest' : 'newest'"
      >
        <iconify-icon
          :icon="sortOrder === 'newest' ? 'mdi:sort-clock-descending' : 'mdi:sort-clock-ascending'"
          width="16"
        ></iconify-icon>
        {{ sortOrder === 'newest' ? t('notifications.sort.newest') : t('notifications.sort.oldest') }}
      </button>

      <div class="flex-1" />

      <!-- Refresh button -->
      <button
        class="px-3 py-2 rounded-lg bg-slate-800 dark:bg-gray-800 border border-slate-700 dark:border-gray-700 text-sm text-slate-300 dark:text-gray-300 hover:border-sky-500/50 transition-colors flex items-center gap-1.5"
        :disabled="loading"
        @click="loadNotifications()"
      >
        <iconify-icon
          :icon="loading ? 'mdi:loading' : 'mdi:refresh'"
          width="16"
          :class="loading ? 'animate-spin' : ''"
        ></iconify-icon>
        {{ loading ? t('notifications.loading') : t('notifications.refresh') }}
      </button>

      <!-- Bulk actions -->
      <button
        v-if="hasSelection"
        class="px-3 py-2 rounded-lg bg-sky-600/15 border border-sky-500/20 text-sky-400 text-sm hover:bg-sky-600/25 transition-colors"
        @click="markAllRead"
      >
        {{ t('notifications.actions.markAllRead') }}
      </button>
      <button
        v-if="notifications.length"
        class="px-3 py-2 rounded-lg bg-amber-600/10 border border-amber-500/15 text-amber-400 text-sm hover:bg-amber-600/20 transition-colors"
        @click="deleteReadBefore"
        :title="t('notifications.actions.deleteReadBeforeTip') || '清理所有已读通知'"
      >
        {{ t('notifications.actions.deleteReadBefore') || '清理已读' }}
      </button>
      <button
        v-if="notifications.length"
        class="px-3 py-2 rounded-lg bg-red-600/10 border border-red-500/15 text-red-400 text-sm hover:bg-red-600/20 transition-colors"
        @click="confirmDeleteAll"
      >
        {{ t('notifications.actions.deleteAll') }}
      </button>
    </template>

    <!-- Error banner -->
    <div
      v-if="error"
      class="mb-4 py-3 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-sm flex items-center gap-2"
    >
      <span class="w-5 h-5 shrink-0 text-red-400">
        <iconify-icon icon="mdi:alert-circle" width="20"></iconify-icon>
      </span>
      {{ error }}
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-20">
      <iconify-icon icon="mdi:loading" width="32" class="animate-spin text-sky-400"></iconify-icon>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="filteredNotifications.length === 0"
      class="flex flex-col items-center justify-center py-20 text-slate-500"
    >
      <iconify-icon icon="mdi:bell-off-outline" width="64" class="mb-3"></iconify-icon>
      <p class="text-sm">{{ t('notifications.empty') }}</p>
    </div>

    <!-- Notification list -->
    <div
      v-else
      ref="listContainerRef"
      class="space-y-2 overflow-y-auto pr-1 rounded-lg"
      :style="{ maxHeight: listHeight }"
    >
      <div
        v-for="n in filteredNotifications"
        :key="n.id"
        class="group relative py-3 rounded-lg border cursor-pointer transition-all"
        :class="[
          n.is_read
            ? 'bg-gray-50 dark:bg-slate-800/40 border-gray-200 dark:border-slate-700/40 hover:bg-gray-100 dark:hover:bg-slate-800/70'
            : 'bg-white dark:bg-slate-800 border-gray-200 dark:border-slate-700 hover:bg-gray-50 dark:hover:bg-slate-800/90 ring-1 ring-sky-500/10',
        ]"
        @click="handleClickNotification(n)"
      >
        <!-- Row: icon | content | time | actions -->
        <div class="flex items-start gap-3">
          <!-- Category icon -->
          <span
            class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 mt-0.5"
            :class="
              (n.category || '').toLowerCase() === 'install'
                ? 'bg-emerald-500/15'
                : (n.category || '').toLowerCase() === 'update'
                ? 'bg-cyan-500/15'
                : 'bg-violet-500/15'
            "
          >
            <!-- Install -->
            <svg
              v-if="(n.category || '').toLowerCase() === 'install'"
              class="text-emerald-400"
              width="16"
              height="16"
              viewBox="0 0 24 24"
            >
              <path
                fill="currentColor"
                d="M8 17v2h8v-2zm8-7l-4 4l-4-4h2.5V2h3v8zM5 22c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h14c1.1 0 2 .9 2 2v16c0 1.1-.9 2-2 2zm0-2h14V4H5z"
              />
            </svg>
            <!-- Update -->
            <svg
              v-else-if="(n.category || '').toLowerCase() === 'update'"
              class="text-cyan-400"
              width="16"
              height="16"
              viewBox="0 0 24 24"
            >
              <path
                fill="currentColor"
                d="M21 10.12h-6.78l2.74-2.82c-2.73-2.7-7.15-2.8-9.88-.1s-2.73 7.08 0 9.79s7.06 2.7 9.79 0c1.37-1.36 2.13-3.14 2.13-5h2c0 2.39-1 4.62-2.71 6.34c-3.38 3.37-8.86 3.37-12.24 0c-3.37-3.38-3.37-8.86 0-12.24c3.06-3.07 8.29-3.29 11.57-.64L21 3z"
              />
            </svg>
            <!-- Default: System / Operation / Security -->
            <svg v-else class="text-violet-400" width="16" height="16" viewBox="0 0 24 24">
              <path
                fill="currentColor"
                d="M12 15.5A3.5 3.5 0 0 1 8.5 12A3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5a3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97s-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98L14.5 2.42c-.04-.24-.25-.42-.5-.42h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.12.22-.07.49.12.64l2.11 1.63c-.04.32-.07.65-.07.97s.03.65.07.97l-2.11 1.66c-.19.15-.24.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64z"
              />
            </svg>
          </span>

          <!-- Body -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2 mb-0.5">
              <span
                class="text-sm font-medium text-gray-900 dark:text-slate-200 truncate"
                :class="{ 'opacity-70': n.is_read }"
              >
                {{ resolveMessage(n).title }}
              </span>
              <!-- Unread dot -->
              <span v-if="!n.is_read" class="w-2 h-2 rounded-full bg-sky-400 shrink-0" />
            </div>
            <p
              class="text-xs text-gray-500 dark:text-slate-400 line-clamp-2 leading-relaxed"
              :class="{ 'opacity-50': n.is_read }"
            >
              {{ resolveMessage(n).body }}
            </p>
            <div class="flex items-center gap-2 mt-1.5">
              <span
                class="text-[10px] px-1.5 py-0.5 rounded font-medium uppercase tracking-wider"
                :class="priorityClass(n.priority)"
              >
                {{ t(`notifications.priority.${n.priority}`) }}
              </span>
              <span class="text-[11px] text-gray-400 dark:text-slate-500">{{ formatTime(n.created_at) }}</span>
              <span v-if="n.action_route" class="text-[11px] text-sky-500 flex items-center gap-0.5">
                <iconify-icon icon="mdi:open-in-new" width="12"></iconify-icon>
                {{ t('notifications.clickable') }}
              </span>
            </div>
          </div>

          <!-- Time (desktop) -->
          <span class="hidden sm:block text-[11px] text-gray-400 dark:text-slate-500 whitespace-nowrap pt-0.5">{{
            formatTime(n.created_at)
          }}</span>

          <!-- Hover actions -->
          <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
            <button
              :title="n.is_read ? t('notifications.actions.markUnread') : t('notifications.actions.markRead')"
              class="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-sky-400 hover:bg-slate-700/50 transition-colors"
              @click.stop="n.is_read ? markUnread(n.id) : markRead(n.id)"
            >
              <iconify-icon
                :icon="n.is_read ? 'mdi:email-outline' : 'mdi:email-open-outline'"
                width="16"
              ></iconify-icon>
            </button>
            <button
              :title="t('notifications.actions.delete')"
              class="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-red-400 hover:bg-red-500/10 transition-colors"
              @click.stop="deleteNotification(n.id)"
            >
              <iconify-icon icon="mdi:delete-outline" width="16"></iconify-icon>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Load More -->
    <div v-if="hasMore && !loading" class="flex justify-center pt-3">
      <button
        class="px-4 py-2 rounded-lg bg-sky-600/10 border border-sky-500/20 text-sky-400 text-sm hover:bg-sky-600/20 transition-colors flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
        :disabled="loadingMore"
        @click="loadMore()"
      >
        <iconify-icon v-if="loadingMore" icon="mdi:loading" width="16" class="animate-spin"></iconify-icon>
        <iconify-icon v-else icon="mdi:chevron-down" width="16"></iconify-icon>
        {{ loadingMore ? t('notifications.loading') : t('notifications.loadMore') }}
      </button>
    </div>

    <!-- Footer stats -->
    <div v-if="!loading" class="mt-4 text-xs text-gray-500 dark:text-slate-500 flex items-center gap-2">
      <span>{{ t('notifications.stats.total', { n: totalCount }) }}</span>
      <span class="text-gray-400 dark:text-slate-600">|</span>
      <span class="text-sky-400">{{ t('notifications.stats.unread', { n: unreadCount }) }}</span>
    </div>

    <!-- Confirm dialog for deleteAll -->
    <div
      v-if="confirmDialog.open"
      class="fixed inset-0 z-999 flex items-center justify-center bg-black/60"
      @click="confirmDialog.open = false"
    >
      <div class="bg-slate-900 border border-slate-700 rounded-xl p-5 max-w-sm w-full mx-4 shadow-xl" @click.stop>
        <h3 class="text-base font-semibold text-slate-100 mb-2">{{ confirmDialog.title }}</h3>
        <p class="text-sm text-slate-400 mb-5">{{ confirmDialog.message }}</p>
        <div class="flex justify-end gap-3">
          <button
            class="px-4 py-2 rounded-lg text-sm font-medium text-slate-300 bg-slate-800 hover:bg-slate-700 transition-colors"
            @click="confirmDialog.open = false"
          >
            {{ t('common.action.cancel') }}
          </button>
          <button
            class="px-4 py-2 rounded-lg text-sm font-medium text-white bg-red-600 hover:bg-red-500 transition-colors"
            @click="confirmDialog.onConfirm"
          >
            {{ t('common.action.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </PageLayout>
</template>

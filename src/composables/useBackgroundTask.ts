import { reactive, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from './useToast'
import i18n from '../locales'

// ── Types ──────────────────────────────────────────────────────────────────

/**
 * Global task execution status.
 * - `idle`     — no task running (default)
 * - `running`  — a task is in progress
 * - `completed`— the last task completed successfully
 * - `failed`   — the last task ended with an error
 */
export type TaskStatus = 'idle' | 'running' | 'completed' | 'failed'

/** Background task state tracked globally (singleton pattern). */
export interface BackgroundTaskState {
  /** Whether the floating overlay should be shown. */
  visible: boolean
  /** Whether the owner dialog has been minimized to background. */
  minimized: boolean
  /** Human-readable task name (e.g., "Installing Rust"). */
  title: string
  /** Current execution phase. */
  status: TaskStatus
  /** Installation progress percentage 0–100. */
  progress: number
  /** Raw log lines from the backend. */
  lines: string[]
}

/** Subscription callback type. */
export type TaskStatusCallback = (status: TaskStatus, state: Readonly<BackgroundTaskState>) => void

// ── LocalStorage persistence ───────────────────────────────────────────────

const STORAGE_KEY = 'rustverse_bg_task_state'

interface PersistedState {
  title: string
  status: TaskStatus
}

function saveToStorage(s: BackgroundTaskState): void {
  try {
    const data: PersistedState = { title: s.title, status: s.status }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
  } catch {
    // localStorage may be unavailable (e.g., private browsing)
  }
}

function loadFromStorage(): PersistedState | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    return JSON.parse(raw) as PersistedState
  } catch {
    return null
  }
}

function clearStorage(): void {
  try {
    localStorage.removeItem(STORAGE_KEY)
  } catch {
    // ignore
  }
}

// ── Singleton state ────────────────────────────────────────────────────────

/** Always reset to idle on restart since task state is volatile. */
const initialState: BackgroundTaskState = {
  visible: false,
  minimized: false,
  title: '',
  status: 'idle',
  progress: 0,
  lines: [],
}

const state = reactive<BackgroundTaskState>({ ...initialState })

/** Callback invoked to hide the owner dialog when minimizing. */
let onHideDialog: (() => void) | null = null

/** Callback invoked to show the owner dialog when restoring. */
let onShowDialog: (() => void) | null = null

/** Reset timer — auto-sets status to idle after completion/failure display. */
let resetTimer: ReturnType<typeof setTimeout> | null = null

// ── Event notification subsystem ───────────────────────────────────────────

const subscribers = new Set<TaskStatusCallback>()

function notifySubscribers(): void {
  const snapshot: Readonly<BackgroundTaskState> = Object.freeze({ ...state })
  for (const cb of subscribers) {
    // Error-isolate each callback so one failure doesn't break others
    try {
      cb(state.status, snapshot)
    } catch (e) {
      console.error('[BackgroundTask] subscriber error:', e)
    }
  }
}

// ── Public API ─────────────────────────────────────────────────────────────

/**
 * Singleton composable for managing long-running background tasks.
 *
 * Features:
 * - Four global states: idle → running → completed/failed → idle
 * - localStorage persistence across page refreshes
 * - Startup guard to prevent concurrent tasks
 * - Event notification via subscribe/unsubscribe
 * - Status query API for any component
 */
export function useBackgroundTask() {
  // ── Start / update ──

  /**
   * Mark a task as started — sets status to 'running', shows overlay.
   * Caller MUST call `guardStart()` before this to ensure safety.
   */
  function startTask(title: string) {
    // Clear any pending reset timer
    if (resetTimer) {
      clearTimeout(resetTimer)
      resetTimer = null
    }

    state.visible = true
    state.minimized = false
    state.title = title
    state.status = 'running'
    state.progress = 0
    state.lines = []

    saveToStorage(state)
    notifySubscribers()
  }

  /** Append log lines and update progress (parsed from backend events). */
  function appendLine(line: string) {
    state.lines.push(line)
    // Attempt to parse "Downloading… N%" from rustup-install-log events
    const match = line.match(/Downloading\.\.\.\s+(\d+)%/)
    if (match) {
      const pct = parseInt(match[1], 10)
      state.progress = Math.min(pct * 0.8, 80)
    }
  }

  /**
   * Call when the task finishes.
   * Auto-resets to 'idle' after 3s of displaying the result.
   */
  function finishTask(status: 'completed' | 'failed') {
    state.status = status
    state.progress = status === 'completed' ? 100 : state.progress

    saveToStorage(state)
    notifySubscribers()

    // Auto-reset to idle after display period
    if (resetTimer) clearTimeout(resetTimer)
    resetTimer = setTimeout(() => {
      reset()
    }, 3000)
  }

  /** Immediately reset to idle state (called automatically after finishTask delay). */
  function reset() {
    if (resetTimer) {
      clearTimeout(resetTimer)
      resetTimer = null
    }
    state.visible = false
    state.minimized = false
    state.title = ''
    state.status = 'idle'
    state.progress = 0
    state.lines = []

    clearStorage()
    notifySubscribers()
  }

  // ── Guard ──

  /**
   * Check whether a new task can be started.
   * If a task is already running, displays a warning toast (if toast API provided).
   * @returns `true` if a task can be started, `false` otherwise.
   */
  function canStartTask(): boolean {
    return state.status === 'idle'
  }

  /**
   * Guard function — call before any installation.
   * Returns `true` if safe to proceed, `false` and shows a user-friendly message if blocked.
   *
   * @param showAlert - optional callback to display the blocking message (falls back to `alert`)
   */
  async function guardStart(showAlert?: (msg: string) => void): Promise<boolean> {
    // 1. Check frontend global state
    const frontendIdle = canStartTask()

    // 2. Verify with backend
    let backendRunning = false
    try {
      backendRunning = await invoke<boolean>('is_background_task_running')
    } catch {
      // Backend check failed — assume safe to proceed
    }

    if (!frontendIdle) {
      if (!backendRunning) {
        // Frontend state is stale — auto-reset and allow
        reset()
      } else {
        // Backend confirms running — block with message
        const msg = state.title
          ? i18n.global.t('progress.message.taskInProgress', { title: state.title })
          : i18n.global.t('progress.message.genericTaskInProgress')
        if (showAlert) {
          showAlert(msg)
        } else {
          try {
            useToast().info(msg)
          } catch {
            alert(msg)
          }
        }
        return false
      }
    } else if (backendRunning) {
      // Frontend is idle but backend has a running task — sync state and block
      state.status = 'running'
      saveToStorage(state)
      notifySubscribers()
      const msg = i18n.global.t('progress.message.genericTaskInProgress')
      if (showAlert) {
        showAlert(msg)
      } else {
        try {
          useToast().info(msg)
        } catch {
          alert(msg)
        }
      }
      return false
    }

    return true
  }

  // ── Minimize / restore ──

  /**
   * Minimize to background overlay.
   * @param onHide - called to hide the owner dialog
   * @param onShow - called to restore the owner dialog (e.g., re-show ProgressDialog)
   */
  function minimize(onHide: () => void, onShow: () => void) {
    state.minimized = true
    onHideDialog = onHide
    onShowDialog = onShow
    // Trigger the hide immediately
    onHide()
  }

  /** Restore the owner dialog from the overlay. */
  function restore() {
    state.minimized = false
    onShowDialog?.()
    onHideDialog = null
    onShowDialog = null
  }

  // ── Cancel ──

  /** Request cancellation of the running background task (backend). */
  async function requestCancel() {
    try {
      await invoke('cancel_background_task')
      // Immediately update frontend state
      state.status = 'failed'
      state.lines.push(i18n.global.t('progress.message.taskCancelled'))
      saveToStorage(state)
      notifySubscribers()

      if (resetTimer) clearTimeout(resetTimer)
      resetTimer = setTimeout(() => {
        reset()
      }, 3000)
    } catch (e) {
      console.error('[BackgroundTask] Failed to cancel:', e)
    }
  }

  // ── Backend check ──

  /** Check whether any task is already running on the backend. */
  async function isTaskRunning(): Promise<boolean> {
    try {
      return await invoke<boolean>('is_background_task_running')
    } catch {
      return false
    }
  }

  // ── Status query ──

  /** Get the current global task status. */
  function queryStatus(): TaskStatus {
    return state.status
  }

  // ── Subscription ──

  /**
   * Subscribe to global task status changes.
   * Callback receives `(newStatus, stateSnapshot)`.
   * Returns an unsubscribe function.
   */
  function subscribe(callback: TaskStatusCallback): () => void {
    subscribers.add(callback)
    return () => {
      subscribers.delete(callback)
    }
  }

  /** Remove a callback from subscribers. */
  function unsubscribe(callback: TaskStatusCallback): void {
    subscribers.delete(callback)
  }

  // ── Computed ──

  /** Whether the overlay is currently minimized. */
  const isMinimized = computed(() => state.minimized)

  /** Whether the overlay is active (visible AND minimized). */
  const isActive = computed(() => state.visible && state.minimized)

  return {
    state,
    isMinimized,
    isActive,

    startTask,
    appendLine,
    finishTask,
    reset,
    guardStart,
    canStartTask,

    minimize,
    restore,
    requestCancel,
    isTaskRunning,

    queryStatus,
    subscribe,
    unsubscribe,
  }
}

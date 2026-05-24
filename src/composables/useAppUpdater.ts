import { ref, readonly } from 'vue'
import { check, type Update, type DownloadEvent } from '@tauri-apps/plugin-updater'

export interface AppUpdateState {
  /** Whether an update check is in progress */
  checking: boolean
  /** The available update, null if no update or not yet checked */
  update: Update | null
  /** Download progress info */
  downloadProgress: {
    phase: 'idle' | 'checking' | 'downloading' | 'installing' | 'success' | 'error'
    downloadedBytes: number
    totalBytes: number | null
    error: string | null
  }
}

export function useAppUpdater() {
  const checking = ref(false)
  const update = ref<Update | null>(null)
  const downloadPhase = ref<AppUpdateState['downloadProgress']['phase']>('idle')
  const downloadedBytes = ref(0)
  const totalBytes = ref<number | null>(null)
  const downloadError = ref<string | null>(null)

  /** Error message suitable for display */
  const checkError = ref<string | null>(null)

  /** Check for app updates */
  async function checkForUpdate(): Promise<Update | null> {
    checking.value = true
    downloadPhase.value = 'checking'
    checkError.value = null
    try {
      const result = await check()
      update.value = result
      if (result) {
        console.log(
          `[AppUpdater] Update available: ${result.version} (current: ${result.currentVersion})`,
        )
      } else {
        console.log('[AppUpdater] App is up to date')
      }
      return result
    } catch (e: any) {
      const msg: string = e?.message || String(e)
      console.warn('[AppUpdater] Check failed:', msg)

      // Classify the error for better UX
      if (
        msg.includes('valid release JSON') ||
        msg.includes('fetch a valid release')
      ) {
        // No latest.json on the server → treat as "up to date", not an error
        checkError.value = null
        console.log('[AppUpdater] No release JSON found — app is up to date')
      } else if (
        msg.includes('network') ||
        msg.includes('fetch') ||
        msg.includes('dns') ||
        msg.includes('connect')
      ) {
        checkError.value = 'Network error: unable to reach update server. Please check your connection.'
      } else {
        checkError.value = msg
      }
      return null
    } finally {
      checking.value = false
      if (downloadPhase.value === 'checking') {
        downloadPhase.value = 'idle'
      }
    }
  }

  /** Download and install the update */
  async function downloadAndInstall() {
    const u = update.value
    if (!u) {
      downloadError.value = 'No update available'
      return
    }

    downloadPhase.value = 'downloading'
    downloadError.value = null
    downloadedBytes.value = 0
    totalBytes.value = null

    try {
      await u.downloadAndInstall((event: DownloadEvent) => {
        switch (event.event) {
          case 'Started':
            totalBytes.value = event.data.contentLength ?? null
            break
          case 'Progress':
            downloadedBytes.value += event.data.chunkLength
            break
          case 'Finished':
            downloadPhase.value = 'installing'
            break
        }
      })
      downloadPhase.value = 'success'
    } catch (e: any) {
      console.error('[AppUpdater] Download/install failed:', e)
      downloadError.value = e?.message || String(e)
      downloadPhase.value = 'error'
    }
  }

  /** Download only (without installing) */
  async function downloadOnly() {
    const u = update.value
    if (!u) {
      downloadError.value = 'No update available'
      return
    }

    downloadPhase.value = 'downloading'
    downloadError.value = null
    downloadedBytes.value = 0
    totalBytes.value = null

    try {
      await u.download((event: DownloadEvent) => {
        switch (event.event) {
          case 'Started':
            totalBytes.value = event.data.contentLength ?? null
            break
          case 'Progress':
            downloadedBytes.value += event.data.chunkLength
            break
          case 'Finished':
            break
        }
      })
      downloadPhase.value = 'success'
    } catch (e: any) {
      console.error('[AppUpdater] Download failed:', e)
      downloadError.value = e?.message || String(e)
      downloadPhase.value = 'error'
    }
  }

  /** Install an already-downloaded update */
  async function installUpdate() {
    const u = update.value
    if (!u) {
      downloadError.value = 'No update available'
      return
    }

    downloadPhase.value = 'installing'
    try {
      await u.install()
      downloadPhase.value = 'success'
    } catch (e: any) {
      console.error('[AppUpdater] Install failed:', e)
      downloadError.value = e?.message || String(e)
      downloadPhase.value = 'error'
    }
  }

  /** Reset state */
  function reset() {
    checking.value = false
    update.value = null
    downloadPhase.value = 'idle'
    downloadedBytes.value = 0
    totalBytes.value = null
    downloadError.value = null
  }

  return {
    checking: readonly(checking),
    update: readonly(update),
    downloadPhase: readonly(downloadPhase),
    downloadedBytes: readonly(downloadedBytes),
    totalBytes: readonly(totalBytes),
    downloadError: readonly(downloadError),
    checkError: readonly(checkError),
    checkForUpdate,
    downloadAndInstall,
    downloadOnly,
    installUpdate,
    reset,
  }
}
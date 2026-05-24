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

// === Global shared state (all useAppUpdater calls share this) ===
const checking = ref(false)
// Keep a raw (non-reactive) reference to Update object — reactive proxy breaks private fields!
let rawUpdate: Update | null = null
// For UI-only: expose safe props only (no methods/resource rid)
const update = ref<{
  version: string
  currentVersion: string
  date?: string
  body?: string
} | null>(null)
const downloadPhase = ref<AppUpdateState['downloadProgress']['phase']>('idle')
const downloadedBytes = ref(0)
const totalBytes = ref<number | null>(null)
const downloadError = ref<string | null>(null)
const checkError = ref<string | null>(null)

export function useAppUpdater() {
  /** Check for app updates */
  async function checkForUpdate(): Promise<Update | null> {
    checking.value = true
    downloadPhase.value = 'checking'
    checkError.value = null
    try {
      const result = await check()
      rawUpdate = result
      if (result) {
        update.value = {
          version: result.version,
          currentVersion: result.currentVersion,
          date: result.date,
          body: result.body,
        }
        console.log(
          `[AppUpdater] Update available: ${result.version} (current: ${result.currentVersion})`,
        )
      } else {
        update.value = null
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
        msg.includes('proxy') ||
        msg.includes('tunnel') ||
        msg.includes('407')
      ) {
        // Proxy / tunnel errors — likely Windows system proxy interference
        checkError.value =
          'Proxy error: your system proxy configuration may be blocking the connection. ' +
          'Try disabling system proxy or restarting the application.'
      } else if (
        msg.includes('certificate') ||
        msg.includes('cert') ||
        msg.includes('SSL') ||
        msg.includes('TLS')
      ) {
        // TLS certificate errors
        checkError.value = 'TLS certificate error: the update server certificate could not be verified.'
      } else if (
        msg.includes('timed out') ||
        msg.includes('timeout') ||
        msg.includes('deadline')
      ) {
        checkError.value = 'Connection timed out: unable to reach the update server. Please check your network.'
      } else if (
        msg.includes('connection refused') ||
        msg.includes('reset') ||
        msg.includes('aborted')
      ) {
        checkError.value = 'Connection refused: the update server is unreachable. Please check your firewall or network settings.'
      } else if (
        msg.includes('dns') ||
        msg.includes('resolve') ||
        msg.includes('name') ||
        msg.includes('host')
      ) {
        checkError.value = 'DNS error: unable to resolve the update server domain name.'
      } else if (
        msg.includes('network') ||
        msg.includes('fetch') ||
        msg.includes('error sending request') ||
        msg.includes('connect')
      ) {
        checkError.value = 'Network error: unable to reach the update server. Please check your connection, firewall, and proxy settings.'
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
    const u = rawUpdate
    if (!u) {
      downloadError.value = 'No update available'
      return
    }

    downloadPhase.value = 'downloading'
    downloadError.value = null
    downloadedBytes.value = 0
    totalBytes.value = null

    try {
      // Try with progress callback first (preferred UX)
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
    const u = rawUpdate
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
    const u = rawUpdate
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
    rawUpdate = null
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

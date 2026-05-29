import { ref } from 'vue'
import { useRustup, type HistRelease } from './useRustup'

/**
 * Extract a human-readable error message from a Tauri command error.
 * Tauri serializes AppError as { kind: string, message: string }.
 * The invoke wrapper may throw the raw object or a string.
 */
function extractErrorMessage(e: unknown): string {
  if (typeof e === 'string') return e
  if (e instanceof Error) return e.message
  if (e && typeof e === 'object') {
    // Tauri AppError shape: { kind, message }
    const obj = e as Record<string, unknown>
    if (typeof obj.message === 'string') return obj.message
    // Fallback: try JSON
    try { return JSON.stringify(e) } catch { /* ignore */ }
  }
  return String(e)
}

const releases = ref<HistRelease[]>([])
const loading = ref(false)
const syncing = ref(false)
const syncError = ref<string | null>(null)

export function useHistoryVersions() {
  const {
    listHistReleases,
    searchHistReleases,
    countHistReleases,
    syncFromManifests: invokeSyncFromManifests,
    installToolchain,
  } = useRustup()

  async function syncFromManifests() {
    syncing.value = true
    syncError.value = null
    try {
      const count = await invokeSyncFromManifests()
      await refresh()
      return count
    } catch (e: unknown) {
      syncError.value = extractErrorMessage(e)
      throw e
    } finally {
      syncing.value = false
    }
  }

  async function refresh(channel?: string) {
    loading.value = true
    try {
      releases.value = await listHistReleases(channel)
    } catch {
      // ignore
    } finally {
      loading.value = false
    }
  }

  async function search(keyword: string, channel?: string) {
    loading.value = true
    try {
      releases.value = await searchHistReleases(keyword, channel)
    } catch {
      // ignore
    } finally {
      loading.value = false
    }
  }

  async function count(channel?: string) {
    return countHistReleases(channel)
  }

  async function installFromHistory(channel: string, version: string, date: string) {
    return installToolchain(channel, version, date)
  }

  return {
    releases,
    loading,
    syncing,
    syncError,
    syncFromManifests,
    refresh,
    search,
    count,
    installFromHistory,
  }
}
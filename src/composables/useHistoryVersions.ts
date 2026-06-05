import { ref } from 'vue'
import { useRustup, type HistRelease } from './useRustup'

const PAGE_SIZE = 50

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
const loadingMore = ref(false)
const hasMore = ref(false)
const totalCount = ref(0)
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
      const page = await listHistReleases(channel, 0, PAGE_SIZE)
      releases.value = page.items
      hasMore.value = page.has_more
      totalCount.value = page.total
    } catch {
      // ignore
    } finally {
      loading.value = false
    }
  }

  async function loadMore(channel?: string) {
    if (loadingMore.value || !hasMore.value) return
    loadingMore.value = true
    try {
      const offset = releases.value.length
      const page = await listHistReleases(channel, offset, PAGE_SIZE)
      releases.value = [...releases.value, ...page.items]
      hasMore.value = page.has_more
      totalCount.value = page.total
    } catch {
      // ignore
    } finally {
      loadingMore.value = false
    }
  }

  async function search(keyword: string, channel?: string) {
    loading.value = true
    try {
      const page = await searchHistReleases(keyword, channel, 0, PAGE_SIZE)
      releases.value = page.items
      hasMore.value = page.has_more
      totalCount.value = page.total
    } catch {
      // ignore
    } finally {
      loading.value = false
    }
  }

  async function searchLoadMore(keyword: string, channel?: string) {
    if (loadingMore.value || !hasMore.value) return
    loadingMore.value = true
    try {
      const offset = releases.value.length
      const page = await searchHistReleases(keyword, channel, offset, PAGE_SIZE)
      releases.value = [...releases.value, ...page.items]
      hasMore.value = page.has_more
      totalCount.value = page.total
    } catch {
      // ignore
    } finally {
      loadingMore.value = false
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
    loadingMore,
    hasMore,
    totalCount,
    syncing,
    syncError,
    syncFromManifests,
    refresh,
    loadMore,
    search,
    searchLoadMore,
    count,
    installFromHistory,
  }
}
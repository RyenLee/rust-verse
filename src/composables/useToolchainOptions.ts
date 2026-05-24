import { ref, onMounted, onUnmounted } from 'vue'
import { useRustup, type ToolchainInfo } from './useRustup'
import { useDataRefresh } from './useDataRefresh'

/** Shared reactive toolchain list — all views read from the same source. */
const toolchains = ref<ToolchainInfo[]>([])
const loading = ref(false)

// Reference counting: only the first consumer sets up the change listener
let globalStop: (() => void) | null = null
let refCount = 0

export function useToolchainOptions() {
  const { listToolchains } = useRustup()

  async function refresh() {
    loading.value = true
    try {
      toolchains.value = await listToolchains()
    } catch {
      // ignore
    } finally {
      loading.value = false
    }
  }

  onMounted(() => {
    refCount++
    if (refCount === 1) {
      // First consumer: set up auto-refresh listener and initial load
      const { onToolchainChange } = useDataRefresh()
      globalStop = onToolchainChange(() => refresh())
      // Only auto-load if not already loaded by ToolchainListView
      if (toolchains.value.length === 0) {
        refresh()
      }
    }
  })

  onUnmounted(() => {
    refCount--
    if (refCount === 0 && globalStop) {
      globalStop()
      globalStop = null
    }
  })

  return { toolchains, loading, refresh }
}
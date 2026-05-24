import { ref, computed, onMounted, onUnmounted } from 'vue'

/** Fixed height configuration for elements above the scrollable list */
export interface FixedHeightConfig {
  /** Top nav header, default 56px */
  navHeader?: number
  /** PageLayout header with breadcrumb, default 56px */
  pageHeader?: number
  /** Filters area height, default 0 */
  filters?: number
  /** Section titles / fixed content above list, default 0 */
  aboveList?: number
  /** Bottom buffer to prevent page scroll, default 80px */
  buffer?: number
}

const defaultConfig: Required<FixedHeightConfig> = {
  navHeader: 56,
  pageHeader: 56,
  filters: 0,
  aboveList: 0,
  buffer: 80,
}

/**
 * Composable that computes responsive max-height for a scrollable list area.
 * Automatically adjusts when the window is resized.
 *
 * Usage:
 * ```ts
 * const { listHeight } = useResponsiveListHeight({ aboveList: 200, filters: 50 })
 * // In template: <div :style="{ maxHeight: listHeight }" class="overflow-y-auto scroll-container">
 * ```
 */
export function useResponsiveListHeight(config: FixedHeightConfig = {}) {
  const cfg = { ...defaultConfig, ...config }
  const windowHeight = ref(window.innerHeight)
  const listContainerRef = ref<HTMLElement | null>(null)

  const fixedTotal = cfg.navHeader + cfg.pageHeader + cfg.filters + cfg.aboveList + cfg.buffer

  const listHeight = computed(() => {
    const available = Math.max(100, windowHeight.value - fixedTotal)
    return `${available}px`
  })

  function handleResize() {
    windowHeight.value = window.innerHeight
  }

  onMounted(() => {
    window.addEventListener('resize', handleResize)
  })

  onUnmounted(() => {
    window.removeEventListener('resize', handleResize)
  })

  return { listHeight, listContainerRef, fixedTotal }
}
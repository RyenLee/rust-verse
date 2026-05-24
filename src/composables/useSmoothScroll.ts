import { ref, type Ref, onMounted, onUnmounted, nextTick } from 'vue'

export interface SmoothScrollOptions {
  /** Animation duration in ms (default: 400) */
  duration?: number
  /** Easing function: 'ease', 'ease-in', 'ease-out', 'ease-in-out', 'linear' (default: 'ease-out') */
  easing?: 'ease' | 'ease-in' | 'ease-out' | 'ease-in-out' | 'linear'
  /** Offset from the target element top (default: 0) */
  offset?: number
  /** Scroll direction: 'vertical' | 'horizontal' (default: 'vertical') */
  direction?: 'vertical' | 'horizontal'
  /** Pixels of extra scroll beyond the target (for visual breathing room) */
  padding?: number
}

export function useSmoothScroll(
  containerRef: Ref<HTMLElement | undefined | null>,
  options: SmoothScrollOptions = {},
) {
  const {
    duration = 400,
    easing = 'ease-out',
    offset = 0,
    direction = 'vertical',
    padding = 0,
  } = options

  const isScrolling = ref(false)
  const isAtTop = ref(true)
  const isAtBottom = ref(false)
  const scrollProgress = ref(0)

  let scrollTimer: ReturnType<typeof setTimeout> | null = null

  function getContainer(): HTMLElement | null {
    return containerRef.value ?? null
  }

  /** Scroll to a specific element within the container */
  async function scrollToElement(el: HTMLElement, behavior: ScrollBehavior = 'smooth') {
    const container = getContainer()
    if (!container || !el) return

    const containerRect = container.getBoundingClientRect()
    const elRect = el.getBoundingClientRect()

    if (direction === 'vertical') {
      const scrollTop =
        container.scrollTop + elRect.top - containerRect.top - offset
      container.scrollTo({ top: scrollTop - padding, behavior })
    } else {
      const scrollLeft =
        container.scrollLeft + elRect.left - containerRect.left - offset
      container.scrollTo({ left: scrollLeft - padding, behavior })
    }
  }

  /** Scroll to a specific position with custom animation */
  function scrollTo(position: number, behavior: ScrollBehavior = 'smooth') {
    const container = getContainer()
    if (!container) return

    const prop = direction === 'vertical' ? 'top' : 'left'
    container.scrollTo({ [prop]: position, behavior })
  }

  /** Scroll to top */
  function scrollToTop(behavior: ScrollBehavior = 'smooth') {
    scrollTo(0, behavior)
  }

  /** Scroll to bottom */
  function scrollToBottom(behavior: ScrollBehavior = 'smooth') {
    const container = getContainer()
    if (!container) return
    const max = direction === 'vertical'
      ? container.scrollHeight - container.clientHeight
      : container.scrollWidth - container.clientWidth
    scrollTo(max, behavior)
  }

  /** Animated scroll using requestAnimationFrame for precise control */
  function animateScrollTo(targetPosition: number) {
    const container = getContainer()
    if (!container) return

    const prop = direction === 'vertical' ? 'scrollTop' : 'scrollLeft'
    const start = container[prop]
    const change = targetPosition - start
    const startTime = performance.now()

    isScrolling.value = true

    function animate(currentTime: number) {
      const elapsed = currentTime - startTime
      const progress = Math.min(elapsed / duration, 1)

      // Easing functions
      let eased: number
      switch (easing) {
        case 'ease-in':
          eased = progress * progress
          break
        case 'ease-out':
          eased = 1 - Math.pow(1 - progress, 3)
          break
        case 'ease-in-out':
          eased = progress < 0.5
            ? 2 * progress * progress
            : 1 - Math.pow(-2 * progress + 2, 2) / 2
          break
        case 'linear':
          eased = progress
          break
        default:
          eased = 1 - Math.pow(1 - progress, 3)
      }

      container[prop] = start + change * eased

      if (progress < 1) {
        requestAnimationFrame(animate)
      } else {
        isScrolling.value = false
        updateScrollState()
      }
    }

    requestAnimationFrame(animate)
  }

  /** Animated scroll to a specific position */
  function animatedScrollTo(targetPosition: number) {
    animateScrollTo(targetPosition)
  }

  /** Check if container has overflow content */
  function hasOverflow(): boolean {
    const container = getContainer()
    if (!container) return false
    return direction === 'vertical'
      ? container.scrollHeight > container.clientHeight
      : container.scrollWidth > container.clientWidth
  }

  /** Scroll to the active/focused item (useful for keyboard nav) */
  async function scrollToActive(selector = '.active, [aria-current="true"]') {
    const container = getContainer()
    if (!container) return
    const active = container.querySelector(selector) as HTMLElement | null
    if (active) {
      await nextTick()
      scrollToElement(active)
    }
  }

  /** Update scroll state (isAtTop, isAtBottom, scrollProgress) */
  function updateScrollState() {
    const container = getContainer()
    if (!container) return

    const threshold = 1
    if (direction === 'vertical') {
      const { scrollTop, scrollHeight, clientHeight } = container
      isAtTop.value = scrollTop <= threshold
      isAtBottom.value = scrollTop + clientHeight >= scrollHeight - threshold
      scrollProgress.value = scrollHeight > clientHeight
        ? scrollTop / (scrollHeight - clientHeight)
        : 0
    } else {
      const { scrollLeft, scrollWidth, clientWidth } = container
      isAtTop.value = scrollLeft <= threshold
      isAtBottom.value = scrollLeft + clientWidth >= scrollWidth - threshold
      scrollProgress.value = scrollWidth > clientWidth
        ? scrollLeft / (scrollWidth - clientWidth)
        : 0
    }
  }

  function handleScroll() {
    if (scrollTimer) clearTimeout(scrollTimer)
    isScrolling.value = true
    updateScrollState()
    scrollTimer = setTimeout(() => {
      isScrolling.value = false
    }, 150)
  }

  onMounted(() => {
    const container = getContainer()
    if (container) {
      container.addEventListener('scroll', handleScroll, { passive: true })
      nextTick(updateScrollState)
    }
  })

  onUnmounted(() => {
    const container = getContainer()
    if (container) {
      container.removeEventListener('scroll', handleScroll)
    }
    if (scrollTimer) clearTimeout(scrollTimer)
  })

  return {
    isScrolling,
    isAtTop,
    isAtBottom,
    scrollProgress,
    scrollToElement,
    scrollTo,
    scrollToTop,
    scrollToBottom,
    animatedScrollTo,
    scrollToActive,
    updateScrollState,
    hasOverflow,
  }
}
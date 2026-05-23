import { ref, watch } from 'vue'

let storeInstance: any = null

async function getStore() {
  if (!storeInstance) {
    const { load } = await import('@tauri-apps/plugin-store')
    storeInstance = await load('app-store.json', { autoSave: true })
  }
  return storeInstance
}

const isDark = ref(true)

async function initTheme() {
  try {
    const store = await getStore()
    const stored = await store.get<string>('theme')
    if (stored === 'light') {
      isDark.value = false
      document.documentElement.classList.remove('dark')
    } else {
      isDark.value = true
      document.documentElement.classList.add('dark')
    }
  } catch {
    // Fallback to localStorage if store is unavailable
    const stored = localStorage.getItem('theme')
    if (stored === 'light') {
      isDark.value = false
      document.documentElement.classList.remove('dark')
    } else {
      isDark.value = true
      document.documentElement.classList.add('dark')
    }
  }
}

async function toggleTheme() {
  isDark.value = !isDark.value
  if (isDark.value) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
  try {
    const store = await getStore()
    await store.set('theme', isDark.value ? 'dark' : 'light')
  } catch {
    localStorage.setItem('theme', isDark.value ? 'dark' : 'light')
  }
}

// --- Window size/position persistence ---

interface WindowBounds {
  x: number
  y: number
  width: number
  height: number
}

let saveTimeout: ReturnType<typeof setTimeout> | null = null

async function restoreWindowBounds() {
  try {
    const store = await getStore()
    const bounds = await store.get<WindowBounds>('windowBounds')
    if (!bounds) return
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const win = getCurrentWindow()
    await win.setPosition(new (await import('@tauri-apps/api/dpi')).PhysicalPosition(bounds.x, bounds.y))
    await win.setSize(new (await import('@tauri-apps/api/dpi')).PhysicalSize(bounds.width, bounds.height))
  } catch {
    // Non-critical, skip
  }
}

async function saveWindowBounds() {
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const win = getCurrentWindow()
    const position = await win.outerPosition()
    const size = await win.outerSize()
    const store = await getStore()
    await store.set('windowBounds', {
      x: position.x,
      y: position.y,
      width: size.width,
      height: size.height,
    })
  } catch {
    // Non-critical, skip
  }
}

function setupWindowBoundsListener() {
  try {
    import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
      const win = getCurrentWindow()
      win.onResized(() => {
        if (saveTimeout) clearTimeout(saveTimeout)
        saveTimeout = setTimeout(saveWindowBounds, 500)
      })
      win.onMoved(() => {
        if (saveTimeout) clearTimeout(saveTimeout)
        saveTimeout = setTimeout(saveWindowBounds, 500)
      })
    })
  } catch {
    // Non-critical, skip
  }
}

export function useAppStore() {
  return {
    isDark,
    initTheme,
    toggleTheme,
    restoreWindowBounds,
    setupWindowBoundsListener,
  }
}

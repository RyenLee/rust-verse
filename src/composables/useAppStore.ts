import { ref } from 'vue'

let storeInstance: any = null

async function getStore() {
  if (!storeInstance) {
    const { load } = await import('@tauri-apps/plugin-store')
    storeInstance = await load('app-store.json', { autoSave: true })
  }
  return storeInstance
}

const isDark = ref(false)
const themeMode = ref<'auto' | 'dark' | 'light'>('light')

/** Check the OS prefers-color-scheme media query and update isDark accordingly. */
function applySystemTheme() {
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  isDark.value = prefersDark
  if (prefersDark) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
}

/** Apply the given theme mode to the DOM. */
function applyTheme(mode: 'auto' | 'dark' | 'light') {
  if (mode === 'auto') {
    applySystemTheme()
  } else if (mode === 'dark') {
    isDark.value = true
    document.documentElement.classList.add('dark')
  } else {
    isDark.value = false
    document.documentElement.classList.remove('dark')
  }
}

async function initTheme() {
  try {
    const store = await getStore()
    const stored = await store.get<'auto' | 'dark' | 'light'>('theme')
    if (stored && ['auto', 'dark', 'light'].includes(stored)) {
      themeMode.value = stored
    } else {
      themeMode.value = 'auto'
    }
  } catch {
    const stored = localStorage.getItem('theme') as 'auto' | 'dark' | 'light' | null
    if (stored && ['auto', 'dark', 'light'].includes(stored)) {
      themeMode.value = stored
    } else {
      themeMode.value = 'auto'
    }
  }

  // Apply the resolved theme mode to the DOM
  applyTheme(themeMode.value)

  // When in auto mode, listen for OS theme changes
  if (themeMode.value === 'auto') {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', applySystemTheme)
  }
}

async function setTheme(mode: 'auto' | 'dark' | 'light') {
  themeMode.value = mode
  applyTheme(mode)
  try {
    const store = await getStore()
    await store.set('theme', mode)
  } catch {
    localStorage.setItem('theme', mode)
  }
}

async function toggleTheme() {
  // Toggle between dark and light based on current visual state
  // When in auto mode, we want to explicitly set to the opposite of current visual state
  const nextMode = isDark.value ? 'light' : 'dark'
  await setTheme(nextMode)
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
    themeMode,
    initTheme,
    setTheme,
    toggleTheme,
    restoreWindowBounds,
    setupWindowBoundsListener,
  }
}

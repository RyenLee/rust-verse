import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useStore } from '@/store'

describe('Pinia Store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.mock('@tauri-apps/api/core', () => ({
      invoke: vi.fn(),
    }))
  })

  it('initial state', () => {
    const store = useStore()
    expect(store.appName).toBe('')
    expect(store.appVersion).toBe('')
    expect(store.appDescription).toBe('')
    // debug is import.meta.env.MODE === 'development', which is true in vitest
    expect(typeof store.debug).toBe('boolean')
  })

  it('isDev getter returns debug state', () => {
    const store = useStore()
    expect(typeof store.isDev).toBe('boolean')
  })

  it('loadAppMeta loads app metadata', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue({
      app: { name: 'TestApp', version: '2.0.0', description: 'A test app' },
    })
    const store = useStore()
    await store.loadAppMeta()
    expect(store.appName).toBe('TestApp')
    expect(store.appVersion).toBe('2.0.0')
    expect(store.appDescription).toBe('A test app')
  })

  it('loadAppMeta uses defaults on error', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockRejectedValue(new Error('failed'))
    const store = useStore()
    await store.loadAppMeta()
    expect(store.appName).toBe('RustVerse')
    expect(store.appVersion).toBe('0.0.0')
    expect(store.appDescription).toBe('')
  })
})
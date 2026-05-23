import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { useStore } from '@/store'

describe('Pinia Store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('initial state', () => {
    const store = useStore()
    expect(store.isInitialized).toBe(false)
    expect(store.name).toBe('')
    expect(store.version).toContain('0.1.0')
  })

  it('initApp action', () => {
    const store = useStore()
    store.initApp()
    expect(store.isInitialized).toBe(true)
  })

  it('isReady getter returns true when not initialized', () => {
    const store = useStore()
    expect(store.isReady).toBe(true)
    store.initApp()
    expect(store.isReady).toBe(false)
  })

  it('storeGreet getter returns empty when no name', () => {
    const store = useStore()
    expect(store.storeGreet).toBe('')
  })

  it('storeGreet getter returns greeting when name is set', () => {
    const store = useStore()
    store.name = 'Rust'
    expect(store.storeGreet).toBe('Greetings from Pinia store, Rust!')
  })
})

import { acceptHMRUpdate, defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

interface AppConfig {
  app: {
    name: string
    version: string
    description: string
  }
}

export const useStore = defineStore('main', {
  state: () => ({
    debug: import.meta.env.MODE === 'development',
    appName: '',
    appVersion: '',
    appDescription: '',
  }),

  getters: {
    isDev: (state) => state.debug,
  },

  actions: {
    async loadAppMeta() {
      try {
        const config = await invoke<AppConfig>('get_config')
        this.appName = config.app?.name || 'RustVerse'
        this.appVersion = config.app?.version || '0.0.0'
        this.appDescription = config.app?.description || ''
      } catch (e) {
        console.warn('Failed to load app metadata, using defaults:', e)
        this.appName = 'RustVerse'
        this.appVersion = '0.0.0'
        this.appDescription = ''
      }
    },
  },
})

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useStore, import.meta.hot))
}

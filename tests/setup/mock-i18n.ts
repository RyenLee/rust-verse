import { config } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { beforeAll } from 'vitest'
import type { Plugin } from 'vue'
import en from '@/locales/en'

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  fallbackLocale: 'en',
  messages: { en },
})

beforeAll(() => {
  config.global.plugins.unshift(i18n as unknown as Plugin)
})
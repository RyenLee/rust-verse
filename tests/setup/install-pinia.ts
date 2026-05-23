import { createTestingPinia, type TestingOptions } from '@pinia/testing'
import { config } from '@vue/test-utils'
import { afterAll, beforeAll } from 'vitest'
import type { Plugin } from 'vue'

export function installPinia(options?: Partial<TestingOptions>) {
  const globalConfigBackup = structuredClone(config.global)

  beforeAll(() => {
    config.global.plugins.unshift(createTestingPinia(options) as unknown as Plugin)
  })

  afterAll(() => {
    config.global = globalConfigBackup
  })
}

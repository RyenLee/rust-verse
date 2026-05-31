import { describe, expect, it } from 'vitest'
import router from '@/router'

describe('Router', () => {
  it('has all expected routes', () => {
    const routeNames = router.getRoutes().map((r) => r.name)
    expect(routeNames).toContain('dashboard')
    expect(routeNames).toContain('toolchains')
    expect(routeNames).toContain('components')
    expect(routeNames).toContain('targets')
    expect(routeNames).toContain('overrides')
    expect(routeNames).toContain('plugins')
  })

  it('resolves / to dashboard', async () => {
    await router.push('/')
    expect(router.currentRoute.value.name).toBe('dashboard')
  })

  it('resolves /toolchains to toolchains', async () => {
    await router.push('/toolchains')
    expect(router.currentRoute.value.name).toBe('toolchains')
  })

  it('uses hash history mode', () => {
    expect(router.options.history).toBeDefined()
  })
})

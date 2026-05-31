import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { createRouter, createWebHashHistory } from 'vue-router'
import DashboardView from '@/views/DashboardView.vue'

function createTestRouter() {
  return createRouter({
    history: createWebHashHistory(),
    routes: [
      { path: '/', name: 'dashboard', component: { template: '<div/>' } },
      { path: '/toolchains', name: 'toolchains', component: { template: '<div/>' } },
      { path: '/components', name: 'components', component: { template: '<div/>' } },
    ],
  })
}

describe('DashboardView', () => {
  it('shows loading state initially', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const router = createTestRouter()
    const wrapper = mount(DashboardView, {
      global: { plugins: [router] },
    })
    expect(wrapper.text()).toContain('Loading...')
  })

  it('shows onboarding when rustup not installed', async () => {
    vi.mocked(invoke).mockResolvedValue({ rustup_installed: false, cargo_installed: false })
    const router = createTestRouter()
    const wrapper = mount(DashboardView, {
      global: { plugins: [router] },
    })
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 10))
    expect(wrapper.text()).toContain('Rust Toolchain Not Found')
  })

  it('shows dashboard stats when environment is ready', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({ rustup_installed: true, cargo_installed: true })
      .mockResolvedValueOnce([
        { name: 'stable-x86_64-pc-windows-msvc', channel: 'stable', is_default: true, is_active: true },
        { name: 'nightly-x86_64-pc-windows-msvc', channel: 'nightly', is_default: false, is_active: false },
      ])
      .mockResolvedValueOnce([
        { toolchain: 'stable', up_to_date: true, new_version: null, current_version: '1.80.0' },
      ])

    const router = createTestRouter()
    const wrapper = mount(DashboardView, {
      global: { plugins: [router] },
    })
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('Default Toolchain')
    expect(wrapper.text()).toContain('Installed')
    expect(wrapper.text()).toContain('Environment')
  })

  it('renders quick link sections', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({ rustup_installed: true, cargo_installed: true })
      .mockResolvedValueOnce([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
      .mockResolvedValueOnce([])

    const router = createTestRouter()
    const wrapper = mount(DashboardView, {
      global: { plugins: [router] },
    })
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('Toolchains')
    expect(wrapper.text()).toContain('Updates')
    expect(wrapper.text()).toContain('Components & Targets')
  })
})
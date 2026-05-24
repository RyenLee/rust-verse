import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import { createRouter, createWebHashHistory } from 'vue-router'
import ComponentsView from '@/views/ComponentsView.vue'
import ToolchainSelector from '@/components/ToolchainSelector.vue'

function createTestRouter() {
  return createRouter({
    history: createWebHashHistory(),
    routes: [
      { path: '/', name: 'dashboard', component: { template: '<div/>' } },
      { path: '/toolchains', name: 'toolchains', component: { template: '<div/>' } },
    ],
  })
}

describe('ComponentsView', () => {
  it('shows loading state during data fetch', async () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const router = createTestRouter()
    const wrapper = mount(ComponentsView, {
      global: { plugins: [router] },
    })
    await vi.dynamicImportSettled()
    expect(wrapper.text()).toContain('Components')
    wrapper.unmount()
  })

  it('renders component list with status indicators', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_toolchains') return Promise.resolve([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
      if (cmd === 'list_components') return Promise.resolve([
        { name: 'rustfmt', installed: true },
        { name: 'clippy', installed: true },
        { name: 'miri', installed: false },
      ])
    })
    const router = createTestRouter()
    const wrapper = mount(ComponentsView, {
      global: { plugins: [router] },
    })
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))

    // Select toolchain via ToolchainSelector dropdown
    const selector = wrapper.findComponent(ToolchainSelector)
    const toggleBtn = selector.find('button')
    await toggleBtn.trigger('click')
    await nextTick()

    // Click the stable option in the dropdown
    const optionBtns = selector.findAll('.absolute button')
    const stableOption = optionBtns.find((btn) => btn.text().includes('stable'))
    await stableOption?.trigger('click')
    await nextTick()

    // Click Load button
    const loadBtn = wrapper.findAll('button').find((btn) => btn.text().includes('Load'))
    await loadBtn?.trigger('click')

    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))

    expect(wrapper.text()).toContain('rustfmt')
    expect(wrapper.text()).toContain('clippy')
    expect(wrapper.text()).toContain('miri')
    expect(wrapper.text()).toContain('Remove')
    expect(wrapper.text()).toContain('Install')
    wrapper.unmount()
  })

  it('has search input and toolchain selector', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const router = createTestRouter()
    const wrapper = mount(ComponentsView, {
      global: { plugins: [router] },
    })
    expect(wrapper.find('input').exists()).toBe(true)
    expect(wrapper.findComponent(ToolchainSelector).exists()).toBe(true)
    wrapper.unmount()
  })
})

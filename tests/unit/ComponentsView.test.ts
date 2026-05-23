import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import ComponentsView from '@/views/ComponentsView.vue'

describe('ComponentsView', () => {
  it('shows loading state during data fetch', async () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(ComponentsView)
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
    const wrapper = mount(ComponentsView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))

    // Select toolchain from dropdown
    const select = wrapper.find('select')
    await select.setValue('stable')
    await nextTick()

    // Click Load button
    const loadBtn = wrapper.find('button')
    await loadBtn.trigger('click')

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
    const wrapper = mount(ComponentsView)
    expect(wrapper.find('input').exists()).toBe(true)
    expect(wrapper.find('select').exists()).toBe(true)
    wrapper.unmount()
  })
})
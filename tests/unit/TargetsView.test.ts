import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import TargetsView from '@/views/TargetsView.vue'

describe('TargetsView', () => {
  it('shows Targets heading during data fetch', async () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(TargetsView)
    await vi.dynamicImportSettled()
    expect(wrapper.text()).toContain('Targets')
    wrapper.unmount()
  })

  it('renders target list with install/remove buttons', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_toolchains') return Promise.resolve([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
      if (cmd === 'list_targets') return Promise.resolve([
        { name: 'x86_64-unknown-linux-gnu', installed: true },
        { name: 'wasm32-unknown-unknown', installed: false },
      ])
    })
    const wrapper = mount(TargetsView)
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

    expect(wrapper.text()).toContain('x86_64-unknown-linux-gnu')
    expect(wrapper.text()).toContain('wasm32-unknown-unknown')
    expect(wrapper.text()).toContain('Remove')
    expect(wrapper.text()).toContain('Install')
    wrapper.unmount()
  })

  it('has search input and toolchain selector', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(TargetsView)
    expect(wrapper.find('input').exists()).toBe(true)
    expect(wrapper.find('select').exists()).toBe(true)
    wrapper.unmount()
  })
})
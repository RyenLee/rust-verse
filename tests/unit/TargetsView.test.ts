import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import TargetsView from '@/views/TargetsView.vue'

describe('TargetsView', () => {
  it('shows Targets heading during data fetch', async () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(TargetsView)
    await vi.dynamicImportSettled()
    expect(wrapper.text()).toContain('Targets')
  })

  it('renders target list with install/remove buttons', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
      .mockResolvedValueOnce([
        { name: 'x86_64-pc-windows-msvc', installed: true },
        { name: 'x86_64-unknown-linux-gnu', installed: false },
      ])
    const wrapper = mount(TargetsView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('x86_64-pc-windows-msvc')
    expect(wrapper.text()).toContain('x86_64-unknown-linux-gnu')
    expect(wrapper.text()).toContain('Remove')
    expect(wrapper.text()).toContain('Install')
  })

  it('has search input and toolchain selector', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(TargetsView)
    expect(wrapper.find('input').exists()).toBe(true)
    expect(wrapper.find('select').exists()).toBe(true)
  })
})

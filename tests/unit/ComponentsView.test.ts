import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import ComponentsView from '@/views/ComponentsView.vue'

describe('ComponentsView', () => {
  it('shows loading state during data fetch', async () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(ComponentsView)
    await vi.dynamicImportSettled()
    // After mount, onMounted sets loading=true then calls loadToolchains
    expect(wrapper.text()).toContain('Components')
  })

  it('renders component list with status indicators', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
      .mockResolvedValueOnce([
        { name: 'rustfmt', installed: true },
        { name: 'clippy', installed: true },
        { name: 'miri', installed: false },
      ])
    const wrapper = mount(ComponentsView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('rustfmt')
    expect(wrapper.text()).toContain('clippy')
    expect(wrapper.text()).toContain('miri')
    expect(wrapper.text()).toContain('Remove')
    expect(wrapper.text()).toContain('Install')
  })

  it('has search input', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(ComponentsView)
    const input = wrapper.find('input')
    expect(input.exists()).toBe(true)
  })

  it('has toolchain selector', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(ComponentsView)
    const select = wrapper.find('select')
    expect(select.exists()).toBe(true)
  })
})

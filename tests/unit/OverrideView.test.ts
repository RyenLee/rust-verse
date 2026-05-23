import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import OverrideView from '@/views/OverrideView.vue'

describe('OverrideView', () => {
  it('shows loading state initially', async () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => { }))
    const wrapper = mount(OverrideView)
    await nextTick()
    expect(wrapper.text()).toContain('Loading...')
    wrapper.unmount()
  })

  it('renders override list', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_toolchains') return Promise.resolve([{ name: 'nightly', channel: 'nightly', is_default: false, is_active: false }])
      if (cmd === 'list_overrides') return Promise.resolve([{ path: '/home/user/project', toolchain: 'nightly' }])
    })
    const wrapper = mount(OverrideView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('/home/user/project')
    expect(wrapper.text()).toContain('nightly')
    wrapper.unmount()
  })

  it('shows empty message when no overrides', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_toolchains') return Promise.resolve([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
      if (cmd === 'list_overrides') return Promise.resolve([])
    })
    const wrapper = mount(OverrideView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('No directory overrides configured.')
    wrapper.unmount()
  })

  it('has add override form', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => { }))
    const wrapper = mount(OverrideView)
    expect(wrapper.text()).toContain('Add Override')
    expect(wrapper.find('input').exists()).toBe(true)
    expect(wrapper.find('select').exists()).toBe(true)
    wrapper.unmount()
  })

  it('shows Remove button for each override', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_toolchains') return Promise.resolve([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
      if (cmd === 'list_overrides') return Promise.resolve([{ path: '/project', toolchain: 'stable' }])
    })
    const wrapper = mount(OverrideView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('Remove')
    wrapper.unmount()
  })
})

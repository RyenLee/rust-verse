import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import OverrideView from '@/views/OverrideView.vue'

describe('OverrideView', () => {
  it('shows loading state initially', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(OverrideView)
    expect(wrapper.text()).toContain('Loading')
  })

  it('renders override list', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([{ path: '/home/user/project', toolchain: 'nightly' }])
      .mockResolvedValueOnce([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
    const wrapper = mount(OverrideView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('/home/user/project')
    expect(wrapper.text()).toContain('nightly')
  })

  it('shows empty message when no overrides', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
    const wrapper = mount(OverrideView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('No directory overrides configured')
  })

  it('has add override form', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(OverrideView)
    expect(wrapper.text()).toContain('Add Override')
    expect(wrapper.find('input').exists()).toBe(true)
    expect(wrapper.find('select').exists()).toBe(true)
  })

  it('shows Remove button for each override', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([{ path: '/project', toolchain: 'stable' }])
      .mockResolvedValueOnce([{ name: 'stable', channel: 'stable', is_default: true, is_active: true }])
    const wrapper = mount(OverrideView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    const buttons = wrapper.findAll('button')
    const buttonTexts = buttons.map((b) => b.text())
    expect(buttonTexts).toContain('Remove')
  })
})

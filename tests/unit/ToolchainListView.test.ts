import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import ToolchainListView from '@/views/ToolchainListView.vue'

describe('ToolchainListView', () => {
  it('shows loading state initially', async () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(ToolchainListView)
    await nextTick()
    expect(wrapper.text()).toContain('Loading...')
    wrapper.unmount()
  })

  it('shows empty message when no toolchains', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const wrapper = mount(ToolchainListView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('No toolchains installed.')
    wrapper.unmount()
  })

  it('shows Install New button', async () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(ToolchainListView)
    await nextTick()
    expect(wrapper.text()).toContain('Install New')
    wrapper.unmount()
  })

  it('renders toolchain list', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { name: 'stable-x86_64-pc-windows-msvc', channel: 'stable', is_default: true, is_active: true },
      { name: 'beta-x86_64-pc-windows-msvc', channel: 'beta', is_default: false, is_active: false },
    ])
    const wrapper = mount(ToolchainListView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('stable-x86_64-pc-windows-msvc')
    expect(wrapper.text()).toContain('beta-x86_64-pc-windows-msvc')
    expect(wrapper.text()).toContain('default')
    wrapper.unmount()
  })

  it('opens install dialog on button click', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const wrapper = mount(ToolchainListView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    const btn = wrapper.find('button')
    await btn.trigger('click')
    // Install panel is teleported to <body>
    expect(document.body.textContent).toContain('Install Toolchain')
    expect(document.body.textContent).toContain('Channel')
    wrapper.unmount()
  })

  it('shows uninstall button for non-default toolchains', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { name: 'stable', channel: 'stable', is_default: true, is_active: true },
      { name: 'nightly', channel: 'nightly', is_default: false, is_active: false },
    ])
    const wrapper = mount(ToolchainListView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('Uninstall')
    expect(wrapper.text()).toContain('Set Default')
    wrapper.unmount()
  })
})
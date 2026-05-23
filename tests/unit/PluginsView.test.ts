import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import PluginsView from '@/views/PluginsView.vue'

describe('PluginsView', () => {
  it('shows loading state initially', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(PluginsView)
    expect(wrapper.text()).toContain('Loading')
  })

  it('renders plugin list', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { name: 'cargo-audit', crate_name: 'cargo-audit', version: '0.18.0' },
      { name: 'cargo-expand', crate_name: 'cargo-expand', version: '1.0.0' },
    ])
    const wrapper = mount(PluginsView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('cargo-audit')
    expect(wrapper.text()).toContain('cargo-expand')
    expect(wrapper.text()).toContain('0.18.0')
  })

  it('shows empty message when no plugins', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const wrapper = mount(PluginsView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('No cargo plugins installed')
  })

  it('has install input and button', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(PluginsView)
    const input = wrapper.find('input')
    expect(input.exists()).toBe(true)
    expect(input.attributes('placeholder')).toContain('crate name')
    expect(wrapper.text()).toContain('Install')
  })

  it('shows uninstall button for each plugin', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { name: 'cargo-audit', crate_name: 'cargo-audit', version: '0.18.0' },
    ])
    const wrapper = mount(PluginsView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    const buttons = wrapper.findAll('button')
    const buttonTexts = buttons.map((b) => b.text())
    expect(buttonTexts).toContain('Uninstall')
  })
})

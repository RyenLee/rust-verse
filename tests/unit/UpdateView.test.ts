import { invoke } from '@tauri-apps/api/core'
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import UpdateView from '@/views/UpdateView.vue'

describe('UpdateView', () => {
  it('shows loading state initially', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(UpdateView)
    expect(wrapper.text()).toContain('Checking for updates')
  })

  it('renders update list with status', async () => {
    vi.mocked(invoke).mockResolvedValue([
      { toolchain: 'stable', up_to_date: true, new_version: null, current_version: '1.80.0' },
      { toolchain: 'nightly', up_to_date: false, new_version: '2024-01-15', current_version: '2024-01-10' },
    ])
    const wrapper = mount(UpdateView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('stable')
    expect(wrapper.text()).toContain('Up to date')
    expect(wrapper.text()).toContain('nightly')
    expect(wrapper.text()).toContain('Update available')
  })

  it('shows empty message when no toolchains', async () => {
    vi.mocked(invoke).mockResolvedValue([])
    const wrapper = mount(UpdateView)
    await vi.dynamicImportSettled()
    await new Promise((r) => setTimeout(r, 50))
    expect(wrapper.text()).toContain('No toolchains found')
  })

  it('has Update All button', () => {
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}))
    const wrapper = mount(UpdateView)
    expect(wrapper.text()).toContain('Update All')
  })
})

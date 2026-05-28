import { invoke } from '@tauri-apps/api/core'

export interface MirrorInfo {
  name: string
  index: string
  mirror_type: string
  is_current: boolean
}

export interface MirrorLatency {
  name: string
  is_current: boolean
  network_ms: number | null
  download_ms: number | null
}

export interface CrmTestResult {
  latencies: MirrorLatency[]
}

const CARGO = 'cargo'

export function useMirror() {
  async function checkInstalled(): Promise<boolean> {
    return invoke<boolean>('check_crm_installed')
  }

  async function install(): Promise<void> {
    await invoke('install_crm', { cargoPath: CARGO })
    // Refresh PATH after installing crm
    try {
      await invoke('refresh_process_path')
    } catch {
      // Best-effort refresh
    }
  }

  async function list(): Promise<MirrorInfo[]> {
    return invoke<MirrorInfo[]>('crm_list')
  }

  async function current(): Promise<string> {
    return invoke<string>('crm_current')
  }

  async function version(): Promise<string> {
    return invoke<string>('crm_version')
  }

  async function useMirror(name: string): Promise<void> {
    return invoke('crm_use', { name })
  }

  async function best(mode?: string): Promise<void> {
    return invoke('crm_best', { mode: mode ?? '' })
  }

  async function restoreDefault(): Promise<void> {
    return invoke('crm_default')
  }

  async function test(name?: string): Promise<CrmTestResult> {
    return invoke<CrmTestResult>('crm_test', { name: name ?? null })
  }

  return {
    checkInstalled,
    install,
    list,
    current,
    version,
    useMirror,
    best,
    restoreDefault,
    test,
  }
}

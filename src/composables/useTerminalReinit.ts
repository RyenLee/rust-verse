import { invoke } from '@tauri-apps/api/core'

export interface TerminalReinitResult {
  success: boolean
  tasks_killed: boolean
  proxy_applied: string
  env_refreshed: string
  message: string
}

export function useTerminalReinit() {
  async function reinitTerminal(cancelRunningTasks?: boolean): Promise<TerminalReinitResult> {
    return invoke<TerminalReinitResult>('reinit_terminal', {
      cancelRunningTasks: cancelRunningTasks ?? true,
    })
  }

  return { reinitTerminal }
}
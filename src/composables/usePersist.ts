import { invoke } from '@tauri-apps/api/core'

export function usePersist() {
  async function persistEnvVar(name: string, value: string) {
    return invoke('persist_env_var', { name, value })
  }

  async function removePersistedEnvVar(name: string) {
    return invoke('remove_persisted_env_var', { name })
  }

  async function isEnvVarPersisted(name: string) {
    return invoke<boolean>('is_env_var_persisted', { name })
  }

  async function listPersistedEnvVars() {
    return invoke<string[]>('list_persisted_env_vars')
  }

  return { persistEnvVar, removePersistedEnvVar, isEnvVarPersisted, listPersistedEnvVars }
}

import { invoke } from '@tauri-apps/api/core'

// --- Type definitions ---

export interface EnvVarMeta {
  name: string
  category: string
  description: string
  rec: string | null
  def: string | null
  notes: string
}

export interface EnvVarInfo extends EnvVarMeta {
  value: string
  is_set: boolean
}

export interface EnvVarEntry {
  name: string
  value: string
  is_set: boolean
}

// --- Composable ---

export function useEnvVars() {
  async function listEnvVars() {
    return invoke<EnvVarInfo[]>('list_env_vars')
  }

  async function getEnvVar(name: string) {
    return invoke<EnvVarEntry>('get_env_var', { name })
  }

  async function setEnvVar(name: string, value: string) {
    return invoke<EnvVarEntry>('set_env_var', { name, value })
  }

  async function removeEnvVar(name: string) {
    return invoke<EnvVarEntry>('remove_env_var', { name })
  }

  async function updateEnvVarMeta(params: {
    category: string
    name: string
    description: string
    rec: string | null
    def: string | null
    notes: string
    oldCategory?: string
    oldName?: string
  }) {
    return invoke('update_env_var_meta', {
      category: params.category,
      name: params.name,
      description: params.description,
      rec: params.rec || null,
      def: params.def || null,
      notes: params.notes,
      oldCategory: params.oldCategory ?? null,
      oldName: params.oldName ?? null,
    })
  }

  async function deleteEnvVarMeta(category: string, name: string) {
    return invoke('delete_env_var_meta', { category, name })
  }

  return {
    listEnvVars,
    getEnvVar,
    setEnvVar,
    removeEnvVar,
    updateEnvVarMeta,
    deleteEnvVarMeta,
  }
}

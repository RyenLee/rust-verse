import { invoke } from '@tauri-apps/api/core'

// --- Shared type definitions ---

export interface ToolchainInfo {
  name: string
  display_name: string
  channel: string
  is_default: boolean
  is_active: boolean
}

export interface UpdateInfo {
  toolchain: string
  up_to_date: boolean
  new_version: string | null
  current_version: string | null
}

export interface OverrideInfo {
  path: string
  toolchain: string
}

export interface ComponentInfo {
  name: string
  installed: boolean
}

export interface TargetInfo {
  name: string
  installed: boolean
}

export interface CargoPluginInfo {
  name: string
  crate_name: string
  version: string
  is_official: boolean
}

export interface SearchResult {
  name: string
  description: string
  version: string
}

export interface VersionInfo {
  rustup_version: string | null
  cargo_version: string | null
}

export interface EnvCheck {
  rustup_installed: boolean
  cargo_installed: boolean
  rustup_error: string | null
  cargo_error: string | null
  /** Resolved CARGO_HOME path (from env var or default ~/.cargo) */
  cargo_home: string | null
  /** Resolved RUSTUP_HOME path (from env var or default ~/.rustup) */
  rustup_home: string | null
}

export interface HistRelease {
  version: string
  date: string
  channel: string
}

// --- Rustup commands ---

const RUSTUP = 'rustup'
const CARGO = 'cargo'

export function useRustup() {
  // Toolchains
  async function listToolchains() {
    return invoke<ToolchainInfo[]>('list_toolchains', { rustupPath: RUSTUP })
  }

  async function installToolchain(channel: string, version?: string, date?: string) {
    return invoke('install_toolchain', {
      rustupPath: RUSTUP,
      channel,
      version: version || null,
      date: date || null,
    })
  }

  async function uninstallToolchain(name: string) {
    return invoke('uninstall_toolchain', { rustupPath: RUSTUP, name })
  }

  async function setDefaultToolchain(name: string) {
    return invoke('set_default_toolchain', { rustupPath: RUSTUP, name })
  }

  // Updates
  async function checkUpdate() {
    return invoke<UpdateInfo[]>('check_update', { rustupPath: RUSTUP })
  }

  async function updateAll() {
    return invoke('update_all', { rustupPath: RUSTUP })
  }

  async function updateRustup() {
    return invoke('update_rustup', { rustupPath: RUSTUP })
  }

  // Overrides
  async function listOverrides() {
    return invoke<OverrideInfo[]>('list_overrides', { rustupPath: RUSTUP })
  }

  async function setOverride(dirPath: string, toolchain: string) {
    return invoke('set_override', { rustupPath: RUSTUP, dirPath, toolchain })
  }

  async function removeOverride(dirPath: string) {
    return invoke('remove_override', { rustupPath: RUSTUP, dirPath })
  }

  async function getOverride(dirPath: string) {
    return invoke<OverrideInfo>('get_override', { rustupPath: RUSTUP, dirPath })
  }

  // Components
  async function listComponents(toolchain: string) {
    return invoke<ComponentInfo[]>('list_components', { rustupPath: RUSTUP, toolchain })
  }

  async function addComponent(toolchain: string, component: string) {
    return invoke('add_component', { rustupPath: RUSTUP, toolchain, component })
  }

  async function removeComponent(toolchain: string, component: string) {
    return invoke('remove_component', { rustupPath: RUSTUP, toolchain, component })
  }

  // Targets
  async function listTargets(toolchain: string) {
    return invoke<TargetInfo[]>('list_targets', { rustupPath: RUSTUP, toolchain })
  }

  async function addTarget(toolchain: string, target: string) {
    return invoke('add_target', { rustupPath: RUSTUP, toolchain, target })
  }

  async function removeTarget(toolchain: string, target: string) {
    return invoke('remove_target', { rustupPath: RUSTUP, toolchain, target })
  }

  // Plugins
  async function listCargoPlugins() {
    return invoke<CargoPluginInfo[]>('list_cargo_plugins', { cargoPath: CARGO })
  }

  async function searchPlugins(query: string) {
    return invoke<SearchResult[]>('search_plugins', { cargoPath: CARGO, query })
  }

  async function installPlugin(crateName: string) {
    return invoke('install_plugin', { cargoPath: CARGO, crateName })
  }

  async function uninstallPlugin(crateName: string) {
    return invoke('uninstall_plugin', { cargoPath: CARGO, crateName })
  }

  // Environment
  async function checkEnv() {
    return invoke<EnvCheck>('check_env')
  }

  async function refreshProcessPath() {
    return invoke<string>('refresh_process_path')
  }

  async function getVersions() {
    return invoke<VersionInfo>('get_versions')
  }

  async function uninstallRustup() {
    return invoke<string>('uninstall_rustup')
  }

  async function installRustup() {
    return invoke<void>('install_rustup')
  }

  // Historical versions
  async function listHistReleases(channel?: string) {
    return invoke<HistRelease[]>('list_hist_releases', { channel: channel || null })
  }

  async function searchHistReleases(keyword: string, channel?: string) {
    return invoke<HistRelease[]>('search_hist_releases', { keyword, channel: channel || null })
  }

  async function countHistReleases(channel?: string) {
    return invoke<number>('count_hist_releases', { channel: channel || null })
  }

  async function syncFromManifests() {
    return invoke<number>('sync_from_manifests')
  }

  return {
    // Toolchains
    listToolchains,
    installToolchain,
    uninstallToolchain,
    setDefaultToolchain,
    // Updates
    checkUpdate,
    updateAll,
    updateRustup,
    // Overrides
    listOverrides,
    setOverride,
    removeOverride,
    getOverride,
    // Components
    listComponents,
    addComponent,
    removeComponent,
    // Targets
    listTargets,
    addTarget,
    removeTarget,
    // Plugins
    listCargoPlugins,
    searchPlugins,
    installPlugin,
    uninstallPlugin,
    // Environment
    checkEnv,
    refreshProcessPath,
    getVersions,
    uninstallRustup,
    installRustup,
    // Historical versions
    listHistReleases,
    searchHistReleases,
    countHistReleases,
    syncFromManifests,
  }
}

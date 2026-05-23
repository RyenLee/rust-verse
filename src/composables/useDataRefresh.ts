import { ref, watch } from 'vue'

// Shared reactive signal — incremented when toolchain data changes
const toolchainVersion = ref(0)

// Shared reactive signal — incremented when env var state changes
const envVarVersion = ref(0)

export function useDataRefresh() {
  /** Call after any toolchain install/uninstall/set-default operation */
  function notifyToolchainChange() {
    toolchainVersion.value++
  }

  /**
   * Watch toolchainVersion and call handler when it changes.
   * Returns the stop function.
   */
  function onToolchainChange(handler: () => void) {
    return watch(toolchainVersion, () => {
      handler()
    })
  }

  /** Call after any env var apply/deactivate/edit operation */
  function notifyEnvVarChange() {
    envVarVersion.value++
  }

  /**
   * Watch envVarVersion and call handler when it changes.
   * Returns the stop function.
   */
  function onEnvVarChange(handler: () => void) {
    return watch(envVarVersion, () => {
      handler()
    })
  }

  return {
    notifyToolchainChange,
    onToolchainChange,
    notifyEnvVarChange,
    onEnvVarChange,
  }
}

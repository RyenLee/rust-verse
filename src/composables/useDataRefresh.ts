import { ref, watch } from 'vue'

// Shared reactive signal — incremented when toolchain data changes
const toolchainVersion = ref(0)

// Shared reactive signal — incremented when env var state changes
const envVarVersion = ref(0)

// Shared reactive signal — incremented when notification data changes (for TopBar badge sync)
const notificationVersion = ref(0)

// Shared reactive signal — incremented when notification settings change
const notifSettingsVersion = ref(0)

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

  /** Call after notification data changes (mark read, delete, etc.) — for TopBar badge sync */
  function notifyNotificationChange() {
    notificationVersion.value++
  }

  /** Watch notificationVersion and call handler when notification data changes */
  function onNotificationChange(handler: () => void) {
    return watch(notificationVersion, () => {
      handler()
    })
  }

  /** Call after notification settings are saved */
  function notifyNotifSettingsChange() {
    notifSettingsVersion.value++
  }

  /**
   * Watch notifSettingsVersion and call handler when it changes.
   * Returns the stop function.
   */
  function onNotifSettingsChange(handler: () => void) {
    return watch(notifSettingsVersion, () => {
      handler()
    })
  }

  return {
    notifyToolchainChange,
    onToolchainChange,
    notifyEnvVarChange,
    onEnvVarChange,
    notifyNotificationChange,
    onNotificationChange,
    notifyNotifSettingsChange,
    onNotifSettingsChange,
  }
}

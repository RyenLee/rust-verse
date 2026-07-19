export default {
  title: 'Notification Center',
  empty: 'No notifications',
  clickable: 'clickable',
  refresh: 'Refresh',
  loading: 'Loading...',
  loadMore: 'Load more',

  categories: {
    install: 'Install Progress',
    update: 'System Updates',
    operation: 'Operation Events',
  },

  priority: {
    high: 'High',
    medium: 'Medium',
    low: 'Low',
  },

  filters: {
    all: 'All',
    searchPlaceholder: 'Search notifications...',
  },

  sort: {
    newest: 'Newest',
    oldest: 'Oldest',
  },

  actions: {
    markRead: 'Mark read',
    markUnread: 'Mark unread',
    markAllRead: 'Mark all read',
    delete: 'Delete',
    deleteAll: 'Delete all',
    deleteReadBefore: 'Clean read',
    deleteReadBeforeTip: 'Clean all read notifications',
    deleteAllConfirm: 'Are you sure you want to delete all {count} notifications? This action cannot be undone.',
  },

  time: {
    justNow: 'Just now',
    minsAgo: '{n} min ago',
    hoursAgo: '{n} h ago',
    daysAgo: '{n} d ago',
  },

  stats: {
    total: '{n} total',
    unread: '{n} unread',
  },

  // ── Notification message templates (resolved via notif_key) ──
  messages: {
    toolchain_installed: {
      title: 'Toolchain installed',
      body: '{channel} has been successfully installed.',
    },
    toolchain_uninstalled: {
      title: 'Toolchain uninstalled',
      body: '{name} has been removed.',
    },
    toolchain_install_failed: {
      title: 'Installation failed',
      body: 'Failed to install {channel}: {error}',
    },
    default_toolchain_changed: {
      title: 'Default toolchain changed',
      body: 'Default toolchain is now {name}.',
    },
    rust_env_installed: {
      title: 'Rust environment installed',
      body: 'rustup has been successfully installed. Welcome to RustVerse!',
    },
    rust_env_install_failed: {
      title: 'Installation failed',
      body: 'Rustup installation failed: {error}',
    },
    crm_installed: {
      title: 'CRM installed',
      body: 'Mirror manager (crm) has been installed.',
    },
    plugin_installed: {
      title: 'Plugin installed',
      body: '{name} has been installed.',
    },
    plugin_install_failed: {
      title: 'Plugin installation failed',
      body: 'Failed to install {name}: {error}',
    },
    plugin_uninstalled: {
      title: 'Plugin uninstalled',
      body: '{name} has been uninstalled.',
    },
    plugin_updated: {
      title: 'Plugin updated',
      body: '{name} has been updated to the latest version.',
    },
    plugin_update_failed: {
      title: 'Plugin update failed',
      body: 'Failed to update {name}: {error}',
    },
    toolchain_updates_available: {
      title: 'Toolchain updates available',
      body: '{count} update(s) available: {names}',
    },
    toolchains_updated: {
      title: 'Toolchains updated',
      body: 'All toolchains have been updated to the latest versions.',
    },
    toolchain_update_failed: {
      title: 'Update failed',
      body: 'Failed to update toolchains: {error}',
    },
    rustup_updated: {
      title: 'rustup updated',
      body: 'rustup has been updated to the latest version.',
    },
    rustup_update_failed: {
      title: 'Update failed',
      body: 'Failed to update rustup: {error}',
    },
    network_diag_failed: {
      title: 'Network diagnostic failed',
      body: 'Unable to reach update server. Check your connection.',
    },
    release_synced: {
      title: 'Release data synced',
      body: 'Synced {count} {channel} release(s).',
    },
    component_added: {
      title: 'Component added',
      body: '{name} has been added to {toolchain}.',
    },
    component_removed: {
      title: 'Component removed',
      body: '{name} has been removed from {toolchain}.',
    },
    target_added: {
      title: 'Target added',
      body: '{name} has been added to {toolchain}.',
    },
    target_removed: {
      title: 'Target removed',
      body: '{name} has been removed from {toolchain}.',
    },
    mirror_switched: {
      title: 'Mirror switched',
      body: 'Switched to mirror: {name}.',
    },
    mirror_best: {
      title: 'Best mirror selected',
      body: 'Best mirror selected with mode: {mode}.',
    },
    mirror_reset: {
      title: 'Mirror reset',
      body: 'Restored to default official registry.',
    },
    env_var_set: {
      title: 'Environment variable set',
      body: '{name}={value}',
    },
    env_var_removed: {
      title: 'Environment variable removed',
      body: '{name} has been unset.',
    },
    env_var_persisted: {
      title: 'Environment variable persisted',
      body: '{name}={value}',
    },
    persist_var_removed: {
      title: 'Persisted variable removed',
      body: '{name} has been removed from system.',
    },
    override_set: {
      title: 'Override set',
      body: 'Toolchain override: {toolchain} → {path}',
    },
    override_removed: {
      title: 'Override removed',
      body: 'Override removed for: {path}',
    },
    env_check_failed: {
      title: 'Environment check failed',
      body: 'rustup: {rustup}, cargo: {cargo}',
    },
  },
}

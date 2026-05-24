export default {
  title: 'Cargo Plugins',
  description: 'Search, install and manage Cargo plugins',
  section: {
    searchCrates: 'Search on crates.io',
    installedPlugins: 'Installed Plugins',
    installByName: 'Install by crate name',
  },
  placeholder: {
    search: 'Search plugins (e.g. cargo-audit)...',
    installByName: 'Install by crate name (e.g. cargo-audit)...',
    filter: 'Filter installed...',
  },
  status: {
    serverError: 'crates.io server error. Please try again later.',
    noFilterMatch: 'No plugins match your filter.',
    noPlugins: 'No cargo plugins installed.',
  },
  badge: {
    official: 'Official',
  },
  dialog: {
    confirmUninstall: 'Confirm Uninstall',
    uninstallConfirm: 'Remove {name}?',
  },
  progress: {
    title: 'Installing Plugin',
    running: 'Installing {name}...',
    success: '{name} installed successfully',
    failed: 'Failed to install {name}',
  },
}

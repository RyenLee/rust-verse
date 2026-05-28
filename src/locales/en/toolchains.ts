export default {
  title: 'Toolchains',
  description: 'Install, uninstall and manage Rust toolchains',
  action: {
    installNew: 'Install New',
    setDefault: 'Set Default',
    historyVersions: 'History',
    browseHistory: 'Browse History Versions',
  },
  status: {
    noToolchains: 'No toolchains installed.',
    installFirst: 'Please install a toolchain first',
    goInstall: 'Go to Install',
  },
  dialog: {
    installTitle: 'Install Toolchain',
    confirmUninstall: 'Confirm Uninstall',
    uninstallConfirm: 'Remove toolchain {name}?',
  },
  form: {
    channel: 'Channel',
    date: 'Date',
    dateOptional: '(optional)',
    datePlaceholder: 'Select date',
  },
  channel: {
    stable: 'Stable',
    stableDesc: 'Latest stable release',
    beta: 'Beta',
    betaDesc: 'Latest beta release',
    nightly: 'Nightly',
    nightlyDesc: 'Daily build',
    latestVersion: 'Latest version',
  },
  help: {
    dateHelp: 'Specify a date for a particular version (e.g. 2024-01-15). Leave empty for the latest.',
    stableBetaHelp: '{channel} always installs the latest available version. No date selection needed.',
    historyHint: 'Need a specific historical version? Browse and select from the history versions page.',
  },
  progress: {
    title: 'Installing Toolchain',
    running: 'Installing {channel}...',
    success: '{channel} installed successfully',
    failed: 'Installation failed',
    installing: 'Installing...',
    cancelled: 'Installation cancelled',
  },
}

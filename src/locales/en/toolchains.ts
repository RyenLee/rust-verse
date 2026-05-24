export default {
  title: 'Toolchains',
  description: 'Install, uninstall and manage Rust toolchains',
  action: {
    installNew: 'Install New',
    setDefault: 'Set Default',
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
  },
  channel: {
    stable: 'Stable',
    stableDesc: 'Latest stable release',
    beta: 'Beta',
    betaDesc: 'Latest beta release',
    nightly: 'Nightly',
    nightlyDesc: 'Daily build (date optional)',
  },
  help: {
    dateHelp: 'Specify a date for a particular nightly build (e.g. 2024-01-15). Leave empty for the latest nightly.',
    stableBetaHelp: '{channel} always installs the latest available version. No date selection needed.',
  },
  progress: {
    title: 'Installing Toolchain',
    running: 'Installing {channel}...',
    success: '{channel} installed successfully',
    failed: 'Installation failed',
    installing: 'Installing...',
  },
}

export default {
  title: 'History Versions',
  description: 'Browse and install historical Rust toolchain versions',
  descriptionSelect: 'Select a historical version to go back and install',
  action: {
    sync: 'Sync Releases',
    syncing: 'Syncing...',
    searchPlaceholder: 'Search version or date...',
    backToToolchains: 'Back to Toolchains',
    select: 'Select',
  },
  filter: {
    dateFrom: 'Start date',
    dateTo: 'End date',
    to: 'to',
    dateRange: 'Select date range',
  },
  channel: {
    stable: 'Stable',
    beta: 'Beta',
    nightly: 'Nightly',
  },
  status: {
    installed: 'Installed',
    noData: 'No release data yet. Click "Sync Releases" to fetch.',
  },
  error: {
    syncHint: 'Check your internet connection and try again, or switch to another channel.',
  },
  progress: {
    title: 'Installing Historical Version',
    running: 'Installing {version}...',
    success: '{version} installed successfully',
    failed: 'Installation failed',
  },
}

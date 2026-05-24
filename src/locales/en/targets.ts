export default {
  title: 'Targets',
  description: 'Manage Rust toolchain compilation targets',
  action: {
    installing: 'Installing',
    removing: 'Removing',
  },
  status: {
    selectPrompt: 'Select a toolchain and click "Load" to view targets.',
  },
  placeholder: {
    search: 'Search targets...',
    toolchainPlaceholder: 'Select Toolchain',
  },
  progress: {
    installTitle: 'Installing Target',
    removeTitle: 'Removing Target',
    running: '{action} {name}...',
    log: '{action} {name} for {toolchain}...',
    success: '{name} installed/removed successfully',
    failed: 'Failed to install/remove {name}',
  },
}

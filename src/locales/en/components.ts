export default {
  title: 'Components',
  description: 'Manage Rust toolchain components',
  action: {
    installing: 'Installing',
    removing: 'Removing',
  },
  status: {
    selectPrompt: 'Select a toolchain and click "Load" to view components.',
  },
  section: {
    installed: 'Installed',
    available: 'Available',
  },
  placeholder: {
    search: 'Search components...',
    toolchainPlaceholder: 'Select Toolchain',
  },
  progress: {
    installTitle: 'Installing Component',
    removeTitle: 'Removing Component',
    running: '{action} {name}...',
    log: '{action} {name} for {toolchain}...',
    success: '{name} installed/removed successfully',
    failed: 'Failed to install/remove {name}',
  },
  datePicker: {
    today: 'Today',
    clear: 'Clear',
    selectStartDate: 'Select start date',
    selectEndDate: 'Select end date',
  },
}

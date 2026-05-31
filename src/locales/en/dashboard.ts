export default {
  title: 'Dashboard',
  description: 'Overview of your Rust development environment',
  rustNotFound: 'Rust Toolchain Not Found',
  rustNotFoundDesc: 'This application requires {rustup} to manage Rust toolchains.',
  action: {
    installRustup: 'Install rustup',
    viewGuide: 'View installation guide',
    manageToolchains: 'Manage installed toolchains',
    checkUpdates: 'Check and apply updates',
    updateToolchains: 'Update toolchains and rustup',
    addRemoveComponents: 'Add or remove components',
    uninstallRustupDesc: 'Completely remove rustup and all toolchains from your system',
  },
  status: {
    afterInstall: 'After installing, restart this application.',
    upToDate: 'Up to date',
    updatesAvailable: '{count} available',
    ready: 'Ready',
    networkError: 'Cannot connect · Update check skipped',
  },
  card: {
    defaultToolchain: 'Default Toolchain',
    installed: 'Installed',
    updates: 'Updates',
    environment: 'Environment',
  },
  section: {
    versions: 'Versions',
    quickActions: 'Quick Actions',
  },
  label: {
    rustup: 'rustup',
    cargo: 'cargo',
    componentsTargets: 'Components & Targets',
  },
}
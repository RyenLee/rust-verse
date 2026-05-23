# Rust Manager

A cross-platform desktop application for visually managing Rust toolchains, components, targets, and cargo plugins. Built with Tauri 2 + Vue 3 + TypeScript + Tailwind CSS 4.

## Features

- **Toolchain Management** — Install, uninstall, and switch between stable/beta/nightly toolchains
- **Component Management** — Add or remove rustfmt, clippy, miri and other components per toolchain
- **Target Management** — Install cross-compilation targets with search and filter
- **Directory Overrides** — Set per-directory toolchain overrides
- **Update Center** — Check and apply toolchain updates with streaming progress
- **Cargo Plugins** — Install and uninstall cargo subcommands
- **Environment Variables** — View, set, and persist environment variables
- **Auto Update** — Built-in application auto-update via tauri-plugin-updater

## Prerequisites

- [Node.js](https://nodejs.org/) >= 20
- [pnpm](https://pnpm.io/) >= 9
- [Rust](https://rustup.rs/) (stable toolchain)
- [Tauri 2 prerequisites](https://tauri.app/start/prerequisites/)
- [rustup](https://rustup.rs/) (required at runtime for toolchain management)

## Quick Start

```sh
# Install dependencies
pnpm install

# Run development server
pnpm tauri dev
```

## Available Commands

| Command | Description |
|---------|-------------|
| `pnpm tauri dev` | Start dev server with hot reload |
| `pnpm test` | Run unit tests (Vitest) |
| `pnpm test:e2e` | Run E2E tests (Playwright) |
| `pnpm build` | Build frontend for production |
| `pnpm tauri build` | Build desktop installer |
| `pnpm check` | Run `cargo check` on backend |
| `pnpm type-check` | Run TypeScript type checking |

## Project Structure

```
rust-manager/
├── src/                              # Frontend (Vue 3 + TypeScript)
│   ├── assets/                       # Static assets (CSS, images)
│   ├── components/                   # Reusable UI components
│   │   ├── BaseButton.vue
│   │   ├── ConfirmDialog.vue
│   │   ├── EmptyState.vue
│   │   ├── ProgressDialog.vue
│   │   ├── SplashScreen.vue
│   │   └── Toast.vue
│   ├── composables/                  # Vue composables (shared logic)
│   │   ├── useAppStore.ts            # Store access helper
│   │   ├── useDataRefresh.ts         # Auto-refresh data polling
│   │   ├── useEnvVars.ts             # Environment variable operations
│   │   ├── useLogger.ts              # Frontend logging bridge
│   │   ├── usePersist.ts             # Persistent state
│   │   ├── useRustup.ts              # Rustup invocation wrapper
│   │   ├── useToast.ts               # Toast notification state
│   │   ├── useToolchainOptions.ts    # Toolchain option helpers
│   │   └── useWithTimeout.ts         # Operation timeout wrapper
│   ├── locales/                      # i18n translations (vue-i18n)
│   │   ├── en/                       # English translations
│   │   └── zh-CN/                    # Chinese translations
│   ├── views/                        # Page components
│   │   ├── DashboardView.vue         # Overview & quick links
│   │   ├── ToolchainListView.vue     # Toolchain CRUD
│   │   ├── ComponentsView.vue        # Component management
│   │   ├── TargetsView.vue           # Target management
│   │   ├── OverrideView.vue          # Directory overrides
│   │   ├── UpdateView.vue            # Update center
│   │   ├── PluginsView.vue           # Cargo plugin management
│   │   ├── EnvVarsView.vue           # Environment variable management
│   │   ├── HelpView.vue              # Help & documentation
│   │   └── WelcomeView.vue           # Onboarding welcome screen
│   ├── App.vue                       # Root layout with sidebar
│   ├── router.ts                     # Vue Router config
│   ├── store.ts                      # Pinia store
│   └── main.ts                       # App entry
├── src-tauri/                        # Backend (Rust + Tauri 2)
│   ├── src/
│   │   ├── commands/                 # Tauri command handlers
│   │   │   ├── toolchain.rs          # Toolchain operations
│   │   │   ├── component.rs          # Component operations
│   │   │   ├── target.rs             # Target operations
│   │   │   ├── override_cmd.rs       # Override operations
│   │   │   ├── update.rs             # Update operations
│   │   │   ├── plugin.rs             # Plugin operations
│   │   │   ├── env_var.rs            # Environment variable operations
│   │   │   ├── env_check.rs          # Environment pre-flight checks
│   │   │   ├── locale.rs             # i18n locale detection
│   │   │   └── persist.rs            # Persistent state management
│   │   ├── system/                   # System utilities
│   │   │   └── env.rs                # Binary detection
│   │   ├── utils/                    # Shared utilities
│   │   │   └── exec.rs               # Command execution
│   │   ├── config.rs                 # App configuration
│   │   ├── db.rs                     # redb database layer
│   │   ├── error.rs                  # Error types
│   │   ├── logger.rs                 # Structured logging
│   │   ├── state.rs                  # App state management
│   │   ├── commands.rs               # Command module re-exports
│   │   ├── system.rs                 # System module re-exports
│   │   ├── utils.rs                  # Utils module re-exports
│   │   ├── lib.rs                    # Plugin & command registration
│   │   └── main.rs                   # Entry point
│   ├── capabilities/                 # Tauri permission config
│   └── tauri.conf.json               # Tauri app config
├── tests/
│   ├── unit/                         # Vitest unit tests (10 files, 42 tests)
│   ├── e2e/                          # Playwright E2E tests
│   │   └── smoke.spec.ts
│   └── setup/                        # Test setup & mocks
│       ├── install-pinia.ts
│       ├── mock-tauri.ts
│       └── testglobals.ts
└── package.json
```

## Testing

### Unit Tests

```sh
# Frontend (42 tests)
pnpm test

# Backend (51 tests)
cargo test --manifest-path src-tauri/Cargo.toml
```

### E2E Tests

```sh
# Requires a running dev server
pnpm test:e2e
```

## Building

```sh
pnpm tauri build
```

This generates platform-specific installers in `src-tauri/target/release/bundle/`:
- **Windows**: NSIS installer (`.exe`) with Chinese/English language support
- **macOS**: `.dmg` and `.app`
- **Linux**: `.deb` and `.AppImage`

## Auto Update

The app includes `tauri-plugin-updater` for automatic updates. To enable:

1. Generate a signing key pair:
   ```sh
   pnpm tauri signer generate -w ~/.tauri/rust-manager.key
   ```
2. Set the `TAURI_SIGNING_PRIVATE_KEY` environment variable during builds
3. Configure `plugins.updater.pubkey` and `plugins.updater.endpoints` in `tauri.conf.json`
4. Host update manifests at your configured endpoint

## License

MIT

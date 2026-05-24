# RustVerse

A cross-platform desktop application for visually managing Rust toolchains, components, targets, and cargo plugins. Built with Tauri 2 + Vue 3 + TypeScript + Tailwind CSS 4.

## Features

- **Toolchain Management** — Install, uninstall, and switch between stable/beta/nightly toolchains
- **Component Management** — Add or remove rustfmt, clippy, miri and other components per toolchain
- **Target Management** — Install cross-compilation targets with search and filter
- **Directory Overrides** — Set per-directory toolchain overrides
- **Update Center** — Check and apply toolchain updates with streaming progress
- **Cargo Plugins** — Install and uninstall cargo subcommands
- **Environment Variables** — View, set, and persist environment variables with CARGO_HOME PATH auto-management
- **Auto Update** — Built-in application auto-update via tauri-plugin-updater with signed artifacts
- **Crates Mirror** — Integrated crm tool for managing crates.io mirror sources with auto-optimal switching

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
rust-verse/
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
│   │   ├── useMirror.ts              # Crates mirror management
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
│   │   ├── MirrorView.vue            # Crates mirror management
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
│   │   │   ├── mirror.rs             # Mirror operations
│   │   │   ├── env_var.rs            # Environment variable operations
│   │   │   ├── env_check.rs          # Environment pre-flight checks
│   │   │   ├── locale.rs             # i18n locale detection
│   │   │   └── persist.rs            # Persistent state & PATH management
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
├── scripts/                          # Build & release scripts
│   ├── bump-version.cjs              # Sync version across config files
│   ├── generate-locale-config.cjs    # Generate locale config from metadata
│   ├── generate-signer-key.cjs       # Generate/verify updater signing keys
│   └── push-release.cjs              # Automated release workflow
├── tests/
│   ├── unit/                         # Vitest unit tests (10 files)
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
# Frontend
pnpm test

# Backend
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

The app includes `tauri-plugin-updater` for automatic updates with signed artifacts.

### Setup

1. Generate a signing key pair:
   ```sh
   node scripts/generate-signer-key.cjs
   ```
   This creates `.tauri-signer-key` (private) and updates `.tauri-signer-key.pub` (public).

2. Add the private key to your CI secrets:
   - GitHub: **Settings > Secrets > `TAURI_SIGNING_PRIVATE_KEY`**

3. Configure `plugins.updater.endpoints` in `tauri.conf.json` to point to your update manifest server.

4. Build with `pnpm tauri build` — signed update artifacts (`.tar.gz` + `.sig`) are generated automatically.

### Release Workflow

```sh
# Full release with version bump and signing
node scripts/push-release.cjs [version]

# Dry run (no actual changes)
node scripts/push-release.cjs --dry-run

# Skip version bump
node scripts/push-release.cjs --skip-bump
```

See [CHANGES.md](./CHANGES.md) for release history.

## License

MIT

---

<a id="chinese"></a>

## 中文说明

RustVerse 是一个跨平台桌面应用，用于可视化管理 Rust 工具链、组件、编译目标和 Cargo 插件。基于 Tauri 2 + Vue 3 + TypeScript + Tailwind CSS 4 构建。

### 核心功能

| 功能 | 说明 |
|------|------|
| 工具链管理 | 安装、卸载、切换 stable/beta/nightly 工具链 |
| 组件管理 | 添加或移除 rustfmt、clippy、miri 等组件 |
| 编译目标管理 | 安装交叉编译目标，支持搜索和筛选 |
| 目录覆盖 | 按目录设置工具链覆盖 |
| 更新中心 | 流式进度展示工具链更新 |
| Cargo 插件 | 安装和卸载 cargo 子命令 |
| 环境变量 | 查看、设置、持久化环境变量，CARGO_HOME 自动管理 PATH |
| 自动更新 | 通过 tauri-plugin-updater 实现签名自动更新 |
| Crates 镜像 | 集成 crm 工具管理 crates.io 镜像源，支持自动最优切换 |

### 快速开始

```sh
pnpm install
pnpm tauri dev
```

### 自动更新签名

首次使用需生成签名密钥对：

```sh
node scripts/generate-signer-key.cjs
```

然后将输出的私钥添加到 GitHub 仓库的 Secrets（`TAURI_SIGNING_PRIVATE_KEY`）。

### 版本历史

详细的版本变更记录请参阅 [CHANGES.md](./CHANGES.md#chinese)。

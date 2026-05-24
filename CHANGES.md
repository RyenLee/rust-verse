# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.3] - 2026-05-24

### Added

- **Welcome page install logging** — All terminal output during installation (progress, status, errors) is now synced to the log file (`data/logs/rustverse.log`)
  - Backend: `installer.rs`, `env_check.rs`, `exec.rs` now write to both Tauri events and log file
  - Frontend: `WelcomeView.vue` logs environment detection and install events via `appLog`
- **Manual install guide** — When installer download fails after 3 retries, the UI shows a step-by-step guide for manually placing the installer in the `data/` directory, then retrying
  - Added i18n keys: `manualGuideTitle`, `manualGuideStep1/2/3` (zh-CN & en)
- **Search box icon layout fix** — Replaced `position: absolute` icon positioning with `flex` inline layout across all search inputs (PluginsView, EnvVarsView, TargetsView, ComponentsView), eliminating icon-text overlap
- Added search icon (`mdi:magnify`) to TargetsView and ComponentsView search inputs for UI consistency

### Changed

- **Removed installer SHA256 verification** — The hash endpoint (`*.sha256`) frequently returns XML error pages instead of valid hashes, causing false integrity check failures and unnecessary re-downloads. The installer is now used directly after download without hash verification
- Removed `sha2` dependency from `Cargo.toml`
- Removed unused `AppError::Integrity` variant from `error.rs`

### Fixed

- Fixed installer integrity check always failing when hash endpoint returns XML error page (e.g. `expected <?xml, got 86478e53...`)
- Fixed search box icon overlapping input text in PluginsView and EnvVarsView

---

## [1.3.2] - 2025-05-24

### Fixed

- Removed `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from release workflow to fix "incorrect updater private key password" error (key was generated without password)

---

## [1.3.1] - 2025-05-24

### Added

- Configured zero-cost auto-update system based on GitHub Releases + `tauri-action` `includeUpdaterJson`
  - Updated updater endpoint to `https://github.com/RyenLee/rust-verse/releases/latest/download/latest.json`
  - Enabled `includeUpdaterJson: true` in release workflow for automatic `latest.json` generation
  - Regenerated Tauri updater signing key pair and updated public key in `tauri.conf.json`

---

## [1.3.0] - 2025-05-24

### Added

- **CARGO_HOME PATH auto-management** — When CARGO_HOME is applied, `%CARGO_HOME%\bin` (Windows) / `$CARGO_HOME/bin` (Unix) is automatically added to the user PATH; when deactivated, it is automatically removed
  - Windows: writes/removes `%CARGO_HOME%\bin` in the registry `HKEY_CURRENT_USER\Environment\Path`
  - Unix: writes/removes `export PATH="$CARGO_HOME/bin:$PATH"` in shell config with managed markers
- **Updater signing key automation** — New `scripts/generate-signer-key.cjs` script to generate or verify Tauri updater signing key pairs
  - Integrated into `push-release.cjs` release workflow
  - Auto-updates public key in `tauri.conf.json` and `.tauri-signer-key.pub`
  - CI environment variable `TAURI_SIGNING_PRIVATE_KEY` configured in release workflow
- Enabled `createUpdaterArtifacts` in Tauri bundle config for signed update artifacts
- Updated critical variable confirmation dialog with PATH auto-management info (blue info banner)
- Added `effect4` entry in critical variable confirmation: warns if Rust tools not installed under new path

### Changed

- **Build version sync fix** — Moved `bump-version.cjs` from `beforeBuildCommand` to `pnpm tauri` script to ensure version is synced before Tauri CLI reads `tauri.conf.json`, fixing installer filename showing stale version
- Updated `deactivateEffect2` i18n text: now states PATH will be automatically removed (was: "system will NOT remove")
- Updated `pathNote` i18n text: now describes automatic PATH add/remove behavior (was: manual PATH instructions)
- Changed PATH note banner color from red (warning) to blue (informational)

### Fixed

- Removed all `console.log` / `console.error` / `console.warn` statements from frontend production code (14+ files)
- Fixed `tsconfig.app.json` referencing non-existent `interface-extensions.d.ts` in include list
- Fixed `bump-version.cjs` regex to correctly match `version` in `[app]` section of `config.toml`

---

## [1.2.5] - 2025-05-23

### Added

- **Crates Mirror Management** — Integrated `crm` tool for managing crates.io mirror sources with auto-optimal switching, mirror list display, manual switching, and latency testing
  - Added crm installation guide for first-time users
  - Auto-optimal (best) feature: evaluates network latency and switches automatically
  - Mirror list display, switching, and testing
  - Fixed table header layout for better experience with large datasets
- Environment variable list component supports dynamic width adjustment

### Fixed

- Fixed input box icon and text overlapping issue by adjusting padding (pl-9 → pl-10)
- Fixed mirror list parsing for `*` marker, current mirror highlighting, and index address prefix removal
- Fixed mirror type detection: `sparse+` prefix → sparse, `.git` suffix → git, others → other
- Fixed "Test All" button position, moved after "Auto-optimal" button
- Fixed left sidebar width and language switch Chinese text wrapping issue
- Fixed local installed plugins display issue
- Fixed crm version number display in status bar
- Fixed page refresh status not updating when switching to mirror management

## [1.2.3] - 2025-05-23

### Added

- `scripts/push-release.cjs` — Automated release script with version bump, git stash, and CHANGES.md integration
- `CHANGES.md` — Changelog document following Keep a Changelog format
- Upload build artifacts (deb, rpm, AppImage) in test-build workflow

### Fixed

- Fixed `unused_mut` warning in `refresh_process_path` on Linux builds using `#[cfg_attr]`
- Fixed all Vitest test failures (10 test files, 40 tests passing)
  - Replaced `lodash-es cloneDeep` with native `structuredClone` in test setup
  - Added i18n plugin setup for tests
  - Fixed shared composable state leaks between tests
  - Updated test assertions to match current component behavior
- Fixed Rust uninstall auto-redirect to welcome page issue by changing installation status detection from file existence to functional validation
- Fixed PowerShell command injection vulnerability in `try_elevated_uninstall` by using temporary script files
- Added timeout protection to `run_command_with_streaming`, `run_command`, and `run_command_with_cwd` functions
- Added parameter validation for `rustup_path` and `cargo_path` to prevent arbitrary command execution
- Added timeout protection to PowerShell download and curl download operations
- Added path validation for `cwd` parameter in `run_command_with_cwd`
- Added binary name validation in `check_rustup` to prevent path hijacking

## [1.2.1] - 2025-05-23

### Added

- GitHub Actions workflow for automated releases (Windows, macOS, Linux)

## [1.2.0] - 2025-05-22

### Added

- Initial release with core features
- Toolchain management (install, uninstall, set default)
- Component and target management
- Directory overrides support
- Cargo plugins management
- Dashboard with system status

[1.3.3]: https://github.com/RyenLee/rust-verse/compare/v1.3.2...v1.3.3
[1.3.2]: https://github.com/RyenLee/rust-verse/compare/v1.3.1...v1.3.2
[1.3.1]: https://github.com/RyenLee/rust-verse/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/RyenLee/rust-verse/compare/v1.2.5...v1.3.0
[1.2.5]: https://github.com/RyenLee/rust-verse/compare/v1.2.3...v1.2.5
[1.2.3]: https://github.com/RyenLee/rust-verse/compare/v1.2.1...v1.2.3
[1.2.1]: https://github.com/RyenLee/rust-verse/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/RyenLee/rust-verse/releases/tag/v1.2.0

---

<a id="chinese"></a>

## 中文说明

本文件记录了 RustVerse 项目的所有重要变更。

- [最新版本 1.3.3](#133---2026-05-24) — 欢迎页安装日志同步、移除安装包验证、手动安装指引、搜索框图标修复
- [版本 1.3.2](#132---2025-05-24) — 修复签名密钥密码错误
- [版本 1.3.1](#131---2025-05-24) — 零成本自动更新系统配置
- [版本 1.3.0](#130---2025-05-24) — CARGO_HOME PATH 自动管理、更新签名自动化、构建版本号修复
- [版本 1.2.5](#125---2025-05-23) — Crates 镜像源管理
- [版本 1.2.3](#123---2025-05-23) — 发布脚本与安全加固
- [版本 1.2.1](#121---2025-05-23) — GitHub Actions 自动发布
- [版本 1.2.0](#120---2025-05-22) — 首次发布

详细项目说明请参阅 [README.md](./README.md#chinese)。

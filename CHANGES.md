# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.7] - 2026-05-25

### Added

- **DateRangePicker component** — New `DateRangePicker.vue` with dropdown calendar for selecting start/end dates in a single interaction; auto-swaps if end < start; highlights range visually with sky-500 endpoints and sky-100/900 range fill
- **useCalendar composable** — Extracted shared calendar logic (`useCalendar.ts`) from DatePicker into a reusable composable providing `generateCalendarGrid`, `fmtDate`, `parseDate`, `todayStr`, month navigation, and reactive calendar state
- **Cross-component toolchain refresh** — `useDataRefresh` now exports `onToolchainChange` watcher; HistoryVersionView calls `notifyToolchainChange()` after install, and ToolchainListView auto-refreshes via `onToolchainChange(() => refresh())` (keep-alive safe)
- **Unit tests for useCalendar** — 24 test cases in `tests/unit/useCalendar.test.ts` covering `fmtDate`, `parseDate`, `todayStr`, `generateCalendarGrid`, and `useCalendar` composable

### Changed

- **DatePicker refactored** — Replaced inline calendar logic with shared `useCalendar` composable; code significantly simplified
- **History version filters redesigned** — Compact single-row flex layout with channel tabs, DateRangePicker, and SearchInput; responsive wrapping on narrow screens
- **Install panel simplified** — Removed date picker and "browse history versions" link from toolchain install slide panel; all three channels display "最新版" description
- **History versions button** — Now navigates directly to the History Versions menu page instead of opening a slide panel

### Fixed

- Fixed history version page not navigating to the correct channel tab when entering from toolchains page
- Fixed selected version not navigating back to the correct channel in toolchains page
- Fixed "浏览历史记录" button not closing the slide panel before navigation
- Fixed future dates not being disabled/greyed out in custom calendar components
- Fixed date picker width too narrow to display full YYYY-MM-DD format
- Fixed `@vuepic/vue-datepicker` default export incompatibility — replaced with custom DatePicker/DateRangePicker components
- Fixed `tsconfig.app.json` configuration errors (removed invalid `types: ["node"]` and `ignoreDeprecations: "6.0"`)
- Fixed toolchain list page not refreshing after installing from history versions page (keep-alive cache issue)

## [1.3.6] - 2026-05-25

### Added

- **Project homepage launched** — New GitHub Pages site at https://ryenlee.github.io/rust-verse/ with auto-update service landing page
- **History versions page** — New `HistoryVersionView.vue` displaying stable/beta/nightly release history with date range filter, search, and install/select capabilities
  - Route navigation between toolchain install page and history versions page with channel pre-fill
  - Date range picker for filtering releases by time period
  - "Select version" mode that navigates back to install panel with selected channel pre-filled
  - Sync releases data per channel
- **Custom DatePicker component** — Native `<input type="date">` based date picker with Tailwind CSS styling, integrated calendar dropdown, today shortcut, and clear button; matches project design system across light/dark themes
- **Minor improvements and bug fixes** — Various small enhancements and stability improvements

### Changed

- **Toolchain install panel simplified** — Removed date picker and "browse history" link from the slide panel; all three channel options now display "最新版 (Latest version)" description
- **History page filters redesigned** — Compact single-row layout with channel tabs, date range, and search input using flex wrap for responsive behavior

### Fixed

- Fixed route navigation timing conflict when closing install panel and navigating to history page
- Fixed TypeScript config errors in `tsconfig.app.json`

## [1.3.5] - 2026-05-25

### Added

- **NSIS uninstall cleanup hook** — Custom NSIS hooks (`windows/hooks.nsh`) with `NSIS_HOOK_POSTUNINSTALL` to recursively remove all runtime-created files (update downloads, data) from install directory on uninstall
- **Windows Defender exclusion hints** — Friendly error guidance in WelcomeView for os error 448 (untrusted mount point), with detailed resolution steps in backend error messages (`env_check.rs`, `exec.rs`)

### Changed

- **Windows build: NSIS only** — Removed MSI target from bundle config, Windows now produces only NSIS `.exe` installer
- **Improved `latest.json` generation script** — Fixed macOS updater artifact lookup from `dmg/` to `macos/` directory, added precise version regex matching and `arch`/`installerExt` filtering

## [1.3.4] - 2026-05-24

### Added

- **App online update frontend UI** — New `AppUpdateView.vue` page with version check, download progress, and install capabilities
  - Composable `useAppUpdater.ts` wrapping `@tauri-apps/plugin-updater` with error classification and progress tracking
  - Added "系统 (System)" navigation group in sidebar with "软件更新 (App Update)" menu item
  - Included project homepage link on the app update page
- **Toolchain update retry mechanism** — `run_command_with_streaming_retry()` in `exec.rs` with exponential backoff (3s → 6s → 12s, max 60s) and streaming retry log events to frontend
- **CI/CD `latest.json` generation script** — `scripts/generate-latest-json.cjs` scans build artifacts and auto-generates the Tauri updater manifest JSON

### Changed

- **Moved app update out of UpdateView** — The "RustVerse 软件更新" section is now a standalone page under "系统 (System)" nav group, avoiding conflicts with the Rust toolchain update page
- **Improved update error handling** — `useAppUpdater` now classifies errors: missing `latest.json` is treated as "up to date" (expected), network failures show a warning banner in the UI

### Fixed

- Fixed update check console error "Could not fetch a valid release JSON from the remote" — when no release JSON is published, the app now silently treats it as up-to-date instead of showing an error

---

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

[1.3.7]: https://github.com/RyenLee/rust-verse/compare/v1.3.6...v1.3.7
[1.3.6]: https://github.com/RyenLee/rust-verse/compare/v1.3.5...v1.3.6
[1.3.5]: https://github.com/RyenLee/rust-verse/compare/v1.3.4...v1.3.5
[1.3.4]: https://github.com/RyenLee/rust-verse/compare/v1.3.3...v1.3.4
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

- [最新版本 1.3.7](#137---2026-05-25) — 日期范围选择组件、useCalendar 组合式函数、跨组件工具链刷新、安装面板简化
- [版本 1.3.6](#136---2026-05-25) — 历史版本页面、自定义日期选择组件、安装面板简化、项目主页上线
- [版本 1.3.5](#135---2026-05-25) — NSIS 卸载清理、Windows 仅保留 exe 安装包、latest.json 生成脚本修复、os error 448 友好提示
- [版本 1.3.4](#134---2026-05-24) — App 在线更新前端 UI、工具链更新重试机制、latest.json 生成脚本、更新错误分类处理
- [版本 1.3.3](#133---2026-05-24) — 欢迎页安装日志同步、移除安装包验证、手动安装指引、搜索框图标修复
- [版本 1.3.2](#132---2025-05-24) — 修复签名密钥密码错误
- [版本 1.3.1](#131---2025-05-24) — 零成本自动更新系统配置
- [版本 1.3.0](#130---2025-05-24) — CARGO_HOME PATH 自动管理、更新签名自动化、构建版本号修复
- [版本 1.2.5](#125---2025-05-23) — Crates 镜像源管理
- [版本 1.2.3](#123---2025-05-23) — 发布脚本与安全加固
- [版本 1.2.1](#121---2025-05-23) — GitHub Actions 自动发布
- [版本 1.2.0](#120---2025-05-22) — 首次发布

详细项目说明请参阅 [README.md](./README.md#chinese)。

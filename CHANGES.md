# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.5] - 2025-05-23

### Added

- **Crates镜像源管理** - 集成crm工具管理crates.io镜像源，支持自动最优切换、镜像列表展示、手动切换、延迟测试等功能
  - 新增crm安装引导功能，初次使用时自动提示安装
  - 支持自动最优（best）功能，评估网络延迟并自动切换
  - 支持镜像列表展示、切换、测试
  - 固定表头布局，提升大数据量时的体验
- 环境变量列表组件支持动态宽度调整

### Fixed

- Fixed input box icon and text overlapping issue by adjusting padding (pl-9 → pl-10)
- Fixed mirror list parsing for * marker, current mirror highlighting, and index address prefix removal
- Fixed mirror type detection: sparse+ prefix → sparse, .git suffix → git, others → other
- Fixed "测试全部" button position, moved to "自动最优" button after
- Fixed left sidebar width and language switch Chinese text wrapping issue
- Fixed local installed plugins display issue
- Fixed crm version number display in status bar
- Fixed page refresh status not updating when switching to mirror management

## [1.2.3] - 2025-05-23

### Added

- `scripts/push-release.cjs` - Automated release script with version bump, git stash, and CHANGES.md integration
- `CHANGES.md` - Changelog document following Keep a Changelog format
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

[1.2.3]: https://github.com/RyenLee/rust-verse/compare/v1.2.1...v1.2.3
[1.2.1]: https://github.com/RyenLee/rust-verse/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/RyenLee/rust-verse/releases/tag/v1.2.0

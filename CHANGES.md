# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

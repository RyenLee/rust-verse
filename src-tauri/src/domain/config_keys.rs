//! 集中化的配置键常量 —— 所有配置 key 的 single source of truth。
//!
//! 使用方式：
//! ```rust
//! use crate::domain::config_keys::keys;
//! let v = store.get_config(keys::BIN_RUSTUP);
//! ```

#[allow(dead_code)]
pub mod keys {
    // ── App ──
    pub const APP_NAME: &str = "app.name";
    pub const APP_VERSION: &str = "app.version";
    pub const APP_DESCRIPTION: &str = "app.description";

    // ── Binaries ──
    pub const BIN_RUSTUP: &str = "binaries.rustup";
    pub const BIN_CARGO: &str = "binaries.cargo";

    // ── Paths ──
    pub const PATHS_CARGO_BIN_RELATIVE: &str = "paths.cargo_bin_relative";

    // ── Locale ──
    pub const LOCALE_FORCE: &str = "locale.force_locale";
    pub const LOCALE_CODES: &str = "locale.codes";
    pub const LOCALE_META: &str = "locale.meta";

    // ── Timeouts ──
    pub const TIMEOUT_CARGO_SEARCH: &str = "timeouts.cargo_search_seconds";
    pub const TIMEOUT_RUSTUP_CHECK: &str = "timeouts.rustup_check_seconds";

    // ── Events ──
    pub const EVENT_INSTALL_LOG: &str = "events.install_log";
    pub const EVENT_INSTALL_FINISHED: &str = "events.install_finished";
    pub const EVENT_PLUGIN_INSTALL_LOG: &str = "events.plugin_install_log";
    pub const EVENT_PLUGIN_INSTALL_FINISHED: &str = "events.plugin_install_finished";
    pub const EVENT_UPDATE_LOG: &str = "events.update_log";
    pub const EVENT_UPDATE_FINISHED: &str = "events.update_finished";

    // ── Parsing ──
    pub const PARSING_DEFAULT_MARKER: &str = "parsing.default_marker";
    pub const PARSING_ACTIVE_MARKER: &str = "parsing.active_marker";
    pub const PARSING_INSTALLED_MARKER: &str = "parsing.installed_marker";
    pub const PARSING_NO_OVERRIDES: &str = "parsing.no_overrides";
    pub const PARSING_UP_TO_DATE: &str = "parsing.up_to_date";
    pub const PARSING_UPDATE_AVAILABLE: &str = "parsing.update_available";
    pub const PARSING_VERSION_SEPARATOR: &str = "parsing.version_separator";
    pub const PARSING_STATUS_SEPARATOR: &str = "parsing.status_separator";
    pub const PARSING_CARGO_PREFIX: &str = "parsing.cargo_prefix";

    // ── Retry ──
    pub const RETRY_UPDATE_MAX: &str = "retry.update_max_retries";
    pub const RETRY_UPDATE_DELAY: &str = "retry.update_delay_ms";
}
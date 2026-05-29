//! Centralized shared constants for the project.
//!
//! Some constants are cfg-gated and only used on specific platforms.
#![allow(dead_code)]

pub mod channel {
    pub const STABLE: &str = "stable";
    pub const BETA: &str = "beta";
    pub const NIGHTLY: &str = "nightly";

    pub static ALL: [&str; 3] = [STABLE, BETA, NIGHTLY];
}

pub mod table_name {
    pub const HISTVER_STABLE: &str = "rs_histver_stable";
    pub const HISTVER_BETA: &str = "rs_histver_beta";
    pub const HISTVER_NIGHTLY: &str = "rs_histver_nightly";
}

pub mod url {
    pub const MANIFESTS: &str = "https://static.rust-lang.org/manifests.txt";
    pub const RUSTUP_WIN: &str = "https://win.rustup.rs/";
    pub const RUSTUP_DIST: &str = "https://static.rust-lang.org/rustup/dist/";
    pub const RUSTUP_SH: &str = "https://sh.rustup.rs";
}

pub mod file_name {
    pub const MANIFESTS_TXT: &str = "manifests.txt";
    pub const RUSTUP_INIT_EXE: &str = "rustup-init.exe";
    pub const RUSTUP_INIT: &str = "rustup-init";
    pub const RUSTUP_INIT_SH: &str = "rustup-init.sh";
    pub const CONFIG_TOML: &str = "config.toml";
    pub const CONFIG_TOML_MIGRATED: &str = "config.toml.migrated";
    pub const SCRUBBING_PREFIX: &str = "scrubbing";
}

pub mod event_name {
    pub const INSTALL_LOG: &str = "rustup-install-log";
    pub const INSTALL_FINISHED: &str = "rustup-install-finished";
    pub const NOTIFICATION_CLEANUP: &str = "notification:cleanup";
    pub const APP_RESTARTING: &str = "app-restarting";
}

pub mod log_module {
    pub const STARTUP: &str = "startup";
    pub const SETUP: &str = "setup";
    pub const INSTALL: &str = "install";
    pub const UPDATE: &str = "update";
    pub const TERMINAL: &str = "terminal";
    pub const TOOLCHAIN: &str = "toolchain";
    pub const CLEANUP: &str = "cleanup";
    pub const STREAM: &str = "stream";
    pub const PROXY: &str = "proxy";
    pub const MANIFEST: &str = "manifest";
}

pub mod installer {
    pub const DEFAULT_TOOLCHAIN: &str = "stable";
    pub const FLAG_YES: &str = "-y";
    pub const FLAG_DEFAULT_TOOLCHAIN: &str = "--default-toolchain";
}

pub mod tray {
    pub const MENU_QUIT: &str = "quit";
    pub const MENU_SHOW: &str = "show";
}

pub mod app {
    pub const WINDOW_MAIN: &str = "main";
    pub const TITLE: &str = "RustVerse";
    pub const FRONTEND_URL: &str = "index.html";
}

pub mod system_env {
    pub const PATH: &str = "Path";
 
   pub const CARGO_HOME: &str = "CARGO_HOME";
    pub const RUSTUP_HOME: &str = "RUSTUP_HOME";
}

pub mod page_route {
    pub const ROOT: &str = "/";
    pub const TOOLCHAINS: &str = "/toolchains";
    pub const UPDATES: &str = "/updates";
    pub const MIRRORS: &str = "/mirrors";
    pub const COMPONENTS: &str = "/components";
    pub const ABOUT: &str = "/about";
}

pub mod proxy_type {
    pub const NONE: &str = "none";
    pub const SYSTEM: &str = "system";
    pub const MANUAL: &str = "manual";
}

pub mod proxy_env_var {
    pub const HTTP_PROXY: &str = "HTTP_PROXY";
    pub const HTTPS_PROXY: &str = "HTTPS_PROXY";
    pub const HTTP_PROXY_LOWER: &str = "http_proxy";
    pub const HTTPS_PROXY_LOWER: &str = "https_proxy";
    pub const ALL_PROXY: &str = "ALL_PROXY";
    pub const ALL_PROXY_LOWER: &str = "all_proxy";
    pub const SOCKS_PROXY: &str = "SOCKS_PROXY";
    pub const SOCKS_PROXY_LOWER: &str = "socks_proxy";
    pub const SOCKS5_PROXY: &str = "SOCKS5_PROXY";
    pub const SOCKS5_PROXY_LOWER: &str = "socks5_proxy";
    pub const NO_PROXY: &str = "NO_PROXY";
    pub const NO_PROXY_LOWER: &str = "no_proxy";

    pub static ALL: &[&str] = &[
        HTTP_PROXY, HTTPS_PROXY, HTTP_PROXY_LOWER, HTTPS_PROXY_LOWER,
        ALL_PROXY, ALL_PROXY_LOWER, SOCKS_PROXY, SOCKS_PROXY_LOWER,
        SOCKS5_PROXY, SOCKS5_PROXY_LOWER, NO_PROXY, NO_PROXY_LOWER,
    ];
}

pub mod locale {
    pub const LC_C: &str = "C";
    pub const LC_ALL: &str = "LC_ALL";
}

pub mod manifest_parse {
    pub const DIST_PREFIX: &str = "static.rust-lang.org/dist/";
    pub const CHANNEL_RUST_PREFIX: &str = "channel-rust-";
    pub const TOML_SUFFIX: &str = ".toml";
    pub const DATE_VERSION_SEP: &str = "#";
    pub const FILE_ENTRY_SEP: &str = "\n";
}

pub mod system_binary {
    pub const RUSTUP: &str = "rustup";
    pub const CARGO: &str = "cargo";
    pub const CRM: &str = "crm";
    pub const POWERSHELL: &str = "powershell.exe";
    pub const CMD: &str = "cmd";
    pub const TASKKILL: &str = "taskkill";
    pub const SH: &str = "sh";

    pub const WINDOWS_EXE_SUFFIX: &str = ".exe";
}

pub mod registry_key {
    #[cfg(target_os = "windows")]
    pub const SYSTEM_ENV: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
    #[cfg(target_os = "windows")]
    pub const USER_ENV: &str = "Environment";
}

pub mod path_segment {
    pub const DOT_CARGO: &str = ".cargo";
    pub const DOT_RUSTUP: &str = ".rustup";
    pub const BIN: &str = "bin";
    pub const DATA: &str = "data";
    pub const LOGS: &str = "logs";
    pub const WEBVIEW: &str = "webview";
    pub const TEMP: &str = "temp";
}
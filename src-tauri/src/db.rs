use std::collections::HashMap;
use std::path::Path;

use redb::{Database, ReadableDatabase, ReadableTableMetadata, TableDefinition, WriteTransaction};

use crate::config::EnvVarEntryConfig;

// ---------------------------------------------------------------------------
// Table definitions
// ---------------------------------------------------------------------------

/// Simple key-value config (binaries, paths, locale, timeouts, events, parsing).
const SIMPLE: TableDefinition<&str, &str> = TableDefinition::new("config_simple");

/// Official plugin names list.
const PLUGINS: TableDefinition<&str, &[u8]> = TableDefinition::new("config_plugins");

/// Environment variable metadata entries.
const ENV_VARS: TableDefinition<&str, &[u8]> = TableDefinition::new("config_env_vars");

// ---------------------------------------------------------------------------
// Database lifecycle
// ---------------------------------------------------------------------------

/// Open an existing database or create a new one at `path`, seeding defaults.
pub fn open_or_create(path: &Path) -> Result<Database, redb::Error> {
    let db = Database::create(path)?;
    seed_defaults_if_empty(&db)?;
    Ok(db)
}

/// Write all default values when the database is freshly created (no keys yet).
fn seed_defaults_if_empty(db: &Database) -> Result<(), redb::Error> {
    let read_tx = db.begin_read()?;
    let needs_seed = match read_tx.open_table(SIMPLE) {
        Ok(table) => table.len()? == 0,
        Err(_) => true, // table does not exist yet
    };
    drop(read_tx);

    if !needs_seed {
        return Ok(());
    }

    let write_tx = db.begin_write()?;
    seed_simple_defaults(&write_tx)?;
    seed_plugin_defaults(&write_tx)?;
    seed_env_var_defaults(&write_tx)?;
    write_tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Simple key-value CRUD
// ---------------------------------------------------------------------------

/// Read a string value from the `config_simple` table.
pub fn get_simple(db: &Database, key: &str) -> Option<String> {
    let read_tx = db.begin_read().ok()?;
    let table = read_tx.open_table(SIMPLE).ok()?;
    table.get(key).ok()?.map(|v| v.value().to_string())
}

/// Write a string value to the `config_simple` table.
#[allow(dead_code)]
pub fn set_simple(db: &Database, key: &str, value: &str) -> Result<(), redb::Error> {
    let write_tx = db.begin_write()?;
    {
        let mut table = write_tx.open_table(SIMPLE)?;
        table.insert(key, value)?;
    }
    write_tx.commit()?;
    Ok(())
}

/// Read multiple string values from the `config_simple` table in a single transaction.
pub fn get_simple_batch(db: &Database, keys: &[&str]) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let Ok(read_tx) = db.begin_read() else {
        return result;
    };
    let Ok(table) = read_tx.open_table(SIMPLE) else {
        return result;
    };
    for &key in keys {
        if let Ok(Some(guard)) = table.get(key) {
            result.insert(key.to_string(), guard.value().to_string());
        }
    }
    result
}

/// Delete a key from the `config_simple` table.
#[allow(dead_code)]
pub fn delete_simple(db: &Database, key: &str) -> Result<bool, redb::Error> {
    let write_tx = db.begin_write()?;
    let removed = {
        let mut table = write_tx.open_table(SIMPLE)?;
        table.remove(key)?.is_some()
    };
    write_tx.commit()?;
    Ok(removed)
}

// ---------------------------------------------------------------------------
// Plugin names CRUD
// ---------------------------------------------------------------------------

/// Get the official plugin names list.
pub fn get_plugin_names(db: &Database) -> Vec<String> {
    let Ok(read_tx) = db.begin_read() else {
        return default_plugin_names();
    };
    let Ok(table) = read_tx.open_table(PLUGINS) else {
        return default_plugin_names();
    };
    let Ok(Some(guard)) = table.get("official") else {
        return default_plugin_names();
    };
    serde_json::from_slice(guard.value()).unwrap_or_else(|_| default_plugin_names())
}

/// Set the official plugin names list.
#[allow(dead_code)]
pub fn set_plugin_names(db: &Database, names: &[String]) -> Result<(), redb::Error> {
    let data = serde_json::to_vec(names).expect("serializing plugin names should not fail");
    let write_tx = db.begin_write()?;
    {
        let mut table = write_tx.open_table(PLUGINS)?;
        table.insert("official", data.as_slice())?;
    }
    write_tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Env vars CRUD
// ---------------------------------------------------------------------------

/// Get all environment variable entries, grouped by category.
pub fn get_env_vars(db: &Database) -> HashMap<String, HashMap<String, EnvVarEntryConfig>> {
    let mut result: HashMap<String, HashMap<String, EnvVarEntryConfig>> = HashMap::new();
    let Ok(read_tx) = db.begin_read() else {
        return default_env_vars();
    };
    let Ok(table) = read_tx.open_table(ENV_VARS) else {
        return default_env_vars();
    };

    for entry in table.range::<&str>(..).ok().into_iter().flatten() {
        let Ok(pair) = entry else { continue };
        let key = pair.0.value();
        let Ok(entry_config) = serde_json::from_slice::<EnvVarEntryConfig>(pair.1.value()) else {
            continue;
        };
        // Key format: "category::VAR_NAME"
        let (category, var_name) = key.split_once("::").unwrap_or(("", key));
        result
            .entry(category.to_string())
            .or_default()
            .insert(var_name.to_string(), entry_config);
    }

    if result.is_empty() {
        return default_env_vars();
    }
    result
}

/// Get env var entries for a specific category.
#[allow(dead_code)]
pub fn get_env_vars_by_category(
    db: &Database,
    category: &str,
) -> HashMap<String, EnvVarEntryConfig> {
    let mut all = get_env_vars(db);
    all.remove(category).unwrap_or_default()
}

/// Set a single env var entry.
#[allow(dead_code)]
pub fn set_env_var_entry(
    db: &Database,
    category: &str,
    var_name: &str,
    entry: &EnvVarEntryConfig,
) -> Result<(), redb::Error> {
    let key = format!("{category}::{var_name}");
    let data = serde_json::to_vec(entry).expect("serializing env var entry should not fail");
    let write_tx = db.begin_write()?;
    {
        let mut table = write_tx.open_table(ENV_VARS)?;
        table.insert(key.as_str(), data.as_slice())?;
    }
    write_tx.commit()?;
    Ok(())
}

/// Delete a single env var entry.
#[allow(dead_code)]
pub fn delete_env_var_entry(
    db: &Database,
    category: &str,
    var_name: &str,
) -> Result<bool, redb::Error> {
    let key = format!("{category}::{var_name}");
    let write_tx = db.begin_write()?;
    let removed = {
        let mut table = write_tx.open_table(ENV_VARS)?;
        table.remove(key.as_str())?.is_some()
    };
    write_tx.commit()?;
    Ok(removed)
}

// ---------------------------------------------------------------------------
// Batch config readers (one transaction per config section)
// ---------------------------------------------------------------------------

/// Read all app metadata values.
pub fn get_app_metadata(db: &Database) -> (String, String, String) {
    let batch = get_simple_batch(db, &["app.name", "app.version", "app.description"]);
    let name = batch
        .get("app.name")
        .cloned()
        .unwrap_or_else(default_app_name);
    let version = batch
        .get("app.version")
        .cloned()
        .unwrap_or_else(default_app_version);
    let description = batch
        .get("app.description")
        .cloned()
        .unwrap_or_else(default_app_description);
    (name, version, description)
}

/// Read all binaries config values.
pub fn get_binaries_config(db: &Database) -> (String, String) {
    let batch = get_simple_batch(db, &["binaries.rustup", "binaries.cargo"]);
    let rustup = batch
        .get("binaries.rustup")
        .cloned()
        .unwrap_or_else(default_rustup);
    let cargo = batch
        .get("binaries.cargo")
        .cloned()
        .unwrap_or_else(default_cargo);
    (rustup, cargo)
}

/// Read all events config values.
pub fn get_events_config(db: &Database) -> EventsConfigValues {
    let batch = get_simple_batch(
        db,
        &[
            "events.install_log",
            "events.install_finished",
            "events.plugin_install_log",
            "events.plugin_install_finished",
            "events.update_log",
            "events.update_finished",
        ],
    );
    EventsConfigValues {
        install_log: batch
            .get("events.install_log")
            .cloned()
            .unwrap_or_else(default_install_log),
        install_finished: batch
            .get("events.install_finished")
            .cloned()
            .unwrap_or_else(default_install_finished),
        plugin_install_log: batch
            .get("events.plugin_install_log")
            .cloned()
            .unwrap_or_else(default_plugin_install_log),
        plugin_install_finished: batch
            .get("events.plugin_install_finished")
            .cloned()
            .unwrap_or_else(default_plugin_install_finished),
        update_log: batch
            .get("events.update_log")
            .cloned()
            .unwrap_or_else(default_update_log),
        update_finished: batch
            .get("events.update_finished")
            .cloned()
            .unwrap_or_else(default_update_finished),
    }
}

/// Read all parsing config values.
pub fn get_parsing_config(db: &Database) -> ParsingConfigValues {
    let batch = get_simple_batch(
        db,
        &[
            "parsing.default_marker",
            "parsing.active_marker",
            "parsing.installed_marker",
            "parsing.no_overrides",
            "parsing.up_to_date",
            "parsing.update_available",
            "parsing.version_separator",
            "parsing.status_separator",
            "parsing.cargo_prefix",
        ],
    );
    ParsingConfigValues {
        default_marker: batch
            .get("parsing.default_marker")
            .cloned()
            .unwrap_or_else(default_default_marker),
        active_marker: batch
            .get("parsing.active_marker")
            .cloned()
            .unwrap_or_else(default_active_marker),
        installed_marker: batch
            .get("parsing.installed_marker")
            .cloned()
            .unwrap_or_else(default_installed_marker),
        no_overrides: batch
            .get("parsing.no_overrides")
            .cloned()
            .unwrap_or_else(default_no_overrides),
        up_to_date: batch
            .get("parsing.up_to_date")
            .cloned()
            .unwrap_or_else(default_up_to_date),
        update_available: batch
            .get("parsing.update_available")
            .cloned()
            .unwrap_or_else(default_update_available),
        version_separator: batch
            .get("parsing.version_separator")
            .cloned()
            .unwrap_or_else(default_version_separator),
        status_separator: batch
            .get("parsing.status_separator")
            .cloned()
            .unwrap_or_else(default_status_separator),
        cargo_prefix: batch
            .get("parsing.cargo_prefix")
            .cloned()
            .unwrap_or_else(default_cargo_prefix),
    }
}

// ---------------------------------------------------------------------------
// Typed config value structs (returned by batch readers)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EventsConfigValues {
    pub install_log: String,
    pub install_finished: String,
    pub plugin_install_log: String,
    pub plugin_install_finished: String,
    pub update_log: String,
    pub update_finished: String,
}

#[derive(Debug, Clone)]
pub struct ParsingConfigValues {
    pub default_marker: String,
    pub active_marker: String,
    pub installed_marker: String,
    pub no_overrides: String,
    pub up_to_date: String,
    pub update_available: String,
    pub version_separator: String,
    pub status_separator: String,
    pub cargo_prefix: String,
}

// ---------------------------------------------------------------------------
// TOML migration
// ---------------------------------------------------------------------------

/// Migrate data from a config.toml file into the redb database.
///
/// Only non-default values are written; keys that match defaults are skipped
/// to keep the database compact and allow future default-value changes to
/// take effect automatically.
pub fn migrate_from_toml(db: &Database, toml_path: &Path) -> Result<bool, String> {
    let content = std::fs::read_to_string(toml_path)
        .map_err(|e| format!("failed to read config.toml: {e}"))?;
    let app_config: crate::config::AppConfig =
        toml::from_str(&content).map_err(|e| format!("failed to parse config.toml: {e}"))?;

    let defaults = crate::config::AppConfig::default();
    if app_config == defaults {
        return Ok(false); // nothing to migrate
    }

    let write_tx = db.begin_write().map_err(|e| format!("db write tx: {e}"))?;

    // Simple string fields
    {
        let mut table = write_tx
            .open_table(SIMPLE)
            .map_err(|e| format!("open SIMPLE: {e}"))?;

        macro_rules! maybe_write {
            ($key:expr, $val:expr, $default:expr) => {
                if $val != $default {
                    table
                        .insert($key, $val.as_str())
                        .map_err(|e| format!("insert {key}: {e}", key = $key))?;
                }
            };
        }

        maybe_write!(
            "app.name",
            app_config.app.name,
            defaults.app.name
        );
        maybe_write!(
            "app.version",
            app_config.app.version,
            defaults.app.version
        );
        maybe_write!(
            "app.description",
            app_config.app.description,
            defaults.app.description
        );
        maybe_write!(
            "binaries.rustup",
            app_config.binaries.rustup,
            defaults.binaries.rustup
        );
        maybe_write!(
            "binaries.cargo",
            app_config.binaries.cargo,
            defaults.binaries.cargo
        );
        maybe_write!(
            "paths.cargo_bin_relative",
            app_config.paths.cargo_bin_relative,
            defaults.paths.cargo_bin_relative
        );
        maybe_write!(
            "locale.force_locale",
            app_config.locale.force_locale,
            defaults.locale.force_locale
        );
        // Locale codes and metadata (stored as JSON)
        if app_config.locale.codes != defaults.locale.codes {
            let codes_json = serde_json::to_string(&app_config.locale.codes)
                .unwrap_or_else(|_| "[]".to_string());
            table
                .insert("locale.codes", codes_json.as_str())
                .map_err(|e| format!("insert locale.codes: {e}"))?;
        }
        if app_config.locale.meta != defaults.locale.meta {
            let meta_json =
                serde_json::to_string(&app_config.locale.meta).unwrap_or_else(|_| "{}".to_string());
            table
                .insert("locale.meta", meta_json.as_str())
                .map_err(|e| format!("insert locale.meta: {e}"))?;
        }
        if app_config.timeouts.cargo_search_seconds != defaults.timeouts.cargo_search_seconds {
            table
                .insert(
                    "timeouts.cargo_search_seconds",
                    app_config
                        .timeouts
                        .cargo_search_seconds
                        .to_string()
                        .as_str(),
                )
                .map_err(|e| format!("insert timeouts.cargo_search_seconds: {e}"))?;
        }
        maybe_write!(
            "events.install_log",
            app_config.events.install_log,
            defaults.events.install_log
        );
        maybe_write!(
            "events.install_finished",
            app_config.events.install_finished,
            defaults.events.install_finished
        );
        maybe_write!(
            "events.plugin_install_log",
            app_config.events.plugin_install_log,
            defaults.events.plugin_install_log
        );
        maybe_write!(
            "events.plugin_install_finished",
            app_config.events.plugin_install_finished,
            defaults.events.plugin_install_finished
        );
        maybe_write!(
            "events.update_log",
            app_config.events.update_log,
            defaults.events.update_log
        );
        maybe_write!(
            "events.update_finished",
            app_config.events.update_finished,
            defaults.events.update_finished
        );
        maybe_write!(
            "parsing.default_marker",
            app_config.parsing.default_marker,
            defaults.parsing.default_marker
        );
        maybe_write!(
            "parsing.active_marker",
            app_config.parsing.active_marker,
            defaults.parsing.active_marker
        );
        maybe_write!(
            "parsing.installed_marker",
            app_config.parsing.installed_marker,
            defaults.parsing.installed_marker
        );
        maybe_write!(
            "parsing.no_overrides",
            app_config.parsing.no_overrides,
            defaults.parsing.no_overrides
        );
        maybe_write!(
            "parsing.up_to_date",
            app_config.parsing.up_to_date,
            defaults.parsing.up_to_date
        );
        maybe_write!(
            "parsing.update_available",
            app_config.parsing.update_available,
            defaults.parsing.update_available
        );
        maybe_write!(
            "parsing.version_separator",
            app_config.parsing.version_separator,
            defaults.parsing.version_separator
        );
        maybe_write!(
            "parsing.status_separator",
            app_config.parsing.status_separator,
            defaults.parsing.status_separator
        );
        maybe_write!(
            "parsing.cargo_prefix",
            app_config.parsing.cargo_prefix,
            defaults.parsing.cargo_prefix
        );
    }

    // Plugin names
    if app_config.plugins.official.names != defaults.plugins.official.names {
        let mut table = write_tx
            .open_table(PLUGINS)
            .map_err(|e| format!("open PLUGINS: {e}"))?;
        let data = serde_json::to_vec(&app_config.plugins.official.names)
            .map_err(|e| format!("serialize plugins: {e}"))?;
        table
            .insert("official", data.as_slice())
            .map_err(|e| format!("insert plugins: {e}"))?;
    }

    // Env vars
    {
        let mut table = write_tx
            .open_table(ENV_VARS)
            .map_err(|e| format!("open ENV_VARS: {e}"))?;

        let categories: [(&str, &HashMap<String, EnvVarEntryConfig>); 5] = [
            ("paths_cache", &app_config.env_vars.paths_cache),
            ("network_proxy", &app_config.env_vars.network_proxy),
            ("build_perf", &app_config.env_vars.build_perf),
            ("debug_diag", &app_config.env_vars.debug_diag),
            ("misc", &app_config.env_vars.misc),
        ];
        let default_categories: [(&str, &HashMap<String, EnvVarEntryConfig>); 5] = [
            ("paths_cache", &defaults.env_vars.paths_cache),
            ("network_proxy", &defaults.env_vars.network_proxy),
            ("build_perf", &defaults.env_vars.build_perf),
            ("debug_diag", &defaults.env_vars.debug_diag),
            ("misc", &defaults.env_vars.misc),
        ];

        for (i, (category, vars)) in categories.into_iter().enumerate() {
            let (_, default_vars) = default_categories[i];
            for (var_name, entry) in vars {
                // Only write if different from default
                let is_default = default_vars.get(var_name).is_some_and(|d| {
                    d.rec == entry.rec
                        && d.def == entry.def
                        && d.description == entry.description
                        && d.notes == entry.notes
                });
                if !is_default {
                    let key = format!("{category}::{var_name}");
                    let data = serde_json::to_vec(entry)
                        .map_err(|e| format!("serialize env var {key}: {e}"))?;
                    table
                        .insert(key.as_str(), data.as_slice())
                        .map_err(|e| format!("insert env var {key}: {e}"))?;
                }
            }
        }
    }

    write_tx
        .commit()
        .map_err(|e| format!("commit migration: {e}"))?;

    Ok(true)
}

// ---------------------------------------------------------------------------
// Default value functions (single source of truth)
// ---------------------------------------------------------------------------

pub fn default_app_name() -> String {
    "RustVerse".to_string()
}
pub fn default_app_version() -> String {
    "1.2.0".to_string()
}
pub fn default_app_description() -> String {
    "Rust Toolchain Visual Version Manager".to_string()
}

pub fn default_rustup() -> String {
    "rustup".to_string()
}
pub fn default_cargo() -> String {
    "cargo".to_string()
}
pub fn default_cargo_bin_relative() -> String {
    ".cargo/bin".to_string()
}
pub fn default_force_locale() -> String {
    "C".to_string()
}
pub fn default_cargo_search_seconds() -> u64 {
    30
}
pub fn default_rustup_check_seconds() -> u64 {
    30
}
pub fn default_install_log() -> String {
    "install-log".to_string()
}
pub fn default_install_finished() -> String {
    "install-finished".to_string()
}
pub fn default_plugin_install_log() -> String {
    "plugin-install-log".to_string()
}
pub fn default_plugin_install_finished() -> String {
    "plugin-install-finished".to_string()
}
pub fn default_update_log() -> String {
    "update-log".to_string()
}
pub fn default_update_finished() -> String {
    "update-finished".to_string()
}
pub fn default_default_marker() -> String {
    "(default)".to_string()
}
pub fn default_active_marker() -> String {
    "(active)".to_string()
}
pub fn default_installed_marker() -> String {
    "(installed)".to_string()
}
pub fn default_no_overrides() -> String {
    "no overrides".to_string()
}
pub fn default_up_to_date() -> String {
    "Up to date".to_string()
}
pub fn default_update_available() -> String {
    "Update available".to_string()
}
pub fn default_version_separator() -> String {
    " -> ".to_string()
}
pub fn default_status_separator() -> String {
    " - ".to_string()
}
pub fn default_cargo_prefix() -> String {
    "cargo-".to_string()
}

pub fn default_plugin_names() -> Vec<String> {
    vec![
        "cargo-clippy".to_string(),
        "cargo-fmt".to_string(),
        "cargo-miri".to_string(),
        "cargo-rustdoc".to_string(),
        "cargo-test-fixture".to_string(),
        "rustfmt".to_string(),
        "clippy".to_string(),
        "miri".to_string(),
    ]
}

macro_rules! env_var_entry {
    ($rec:expr, $def:expr, $description:expr, $notes:expr) => {
        EnvVarEntryConfig {
            rec: $rec.map(|s: &str| s.to_string()),
            def: $def.map(|s: &str| s.to_string()),
            description: $description.to_string(),
            notes: $notes.to_string(),
        }
    };
}

pub fn default_env_vars() -> HashMap<String, HashMap<String, EnvVarEntryConfig>> {
    let mut result = HashMap::new();

    // ── 基础路径与缓存优化 ──
    let mut paths_cache = HashMap::new();
    paths_cache.insert(
        "CARGO_HOME".to_string(),
        env_var_entry!(
            None,
            Some("%USERPROFILE%\\.cargo"),
            "Cargo 家目录（存放 registry、git 仓库、已编译 crate 等）",
            "推荐挪到非系统盘，避免 C 盘膨胀。重开终端或移动目录后生效。"
        ),
    );
    paths_cache.insert(
        "RUSTUP_HOME".to_string(),
        env_var_entry!(
            None,
            Some("%USERPROFILE%\\.rustup"),
            "rustup 工具链和全局配置的安装位置",
            "推荐挪到非系统盘。需配合移动现有目录或首次安装时设置。"
        ),
    );
    paths_cache.insert("CARGO_TARGET_DIR".to_string(), env_var_entry!(
        None,
        None,
        "统一存放所有项目的编译输出（target 目录）",
        "可避免每个项目生成独立 target 文件夹，节省磁盘并共享编译缓存。不同项目间可能因 feature 差异偶尔需要清理。"
    ));
    paths_cache.insert(
        "CARGO_CACHE_RUSTC_INFO".to_string(),
        env_var_entry!(
            Some("1"),
            None,
            "缓存 rustc 信息以加速下一次编译（nightly 功能）",
            "仅当使用 nightly 工具链时有效，稳定版暂不支持。"
        ),
    );
    result.insert("paths_cache".to_string(), paths_cache);

    // ── 网络与代理 ──
    let mut network_proxy = HashMap::new();
    network_proxy.insert("HTTP_PROXY".to_string(), env_var_entry!(
        None,
        Some("http://127.0.0.1:7890"),
        "为 Cargo 和 rustup 指定 HTTP 代理",
        "格式 http://127.0.0.1:7890（根据本机代理地址填写）。Windows 上通常大写即可，部分工具可能同时需要小写变量。"
    ));
    network_proxy.insert(
        "HTTPS_PROXY".to_string(),
        env_var_entry!(
            None,
            Some("https://127.0.0.1:7890"),
            "为 Cargo 和 rustup 指定 HTTPS 代理",
            "与 HTTP_PROXY 类似，用于 HTTPS 连接，值通常相同。"
        ),
    );
    network_proxy.insert(
        "NO_PROXY".to_string(),
        env_var_entry!(
            None,
            Some("localhost,127.0.0.1,.local"),
            "跳过代理的地址列表",
            "避免内部通信走代理，多个地址用逗号分隔。"
        ),
    );
    network_proxy.insert(
        "CARGO_HTTP_CAINFO".to_string(),
        env_var_entry!(
            None,
            None,
            "指定自定义 CA 证书包（如公司自签证书）",
            "指向 PEM 格式证书文件路径，解决企业环境 SSL 验证问题。"
        ),
    );
    network_proxy.insert(
        "CARGO_HTTP_CHECK_REVOKE".to_string(),
        env_var_entry!(
            None,
            Some("true"),
            "控制 Cargo 是否检查 SSL 证书吊销状态",
            "当遇到 SSL error 且确信网络无问题时，可临时设为 false。不推荐长期禁用，存在安全风险。"
        ),
    );
    network_proxy.insert(
        "CARGO_NET_RETRY".to_string(),
        env_var_entry!(
            None,
            Some("3"),
            "网络请求失败重试次数",
            "网络不稳定时可适当增大。"
        ),
    );
    network_proxy.insert(
        "CARGO_HTTP_TIMEOUT".to_string(),
        env_var_entry!(
            None,
            Some("30"),
            "HTTP 请求超时时间（秒）",
            "慢速网络环境建议调大，避免误报超时。"
        ),
    );
    result.insert("network_proxy".to_string(), network_proxy);

    // ── 编译性能与缓存加速 ──
    let mut build_perf = HashMap::new();
    build_perf.insert("RUSTC_WRAPPER".to_string(), env_var_entry!(
        None,
        Some("sccache"),
        "在调用 rustc 前先执行指定程序（常用于 sccache）",
        "安装 sccache 后设置，需确保 sccache.exe 在 PATH 中。可通过 scoop install sccache 安装。"
    ));
    build_perf.insert(
        "SCCACHE_DIR".to_string(),
        env_var_entry!(
            None,
            Some("%LOCALAPPDATA%\\Mozilla\\sccache"),
            "sccache 缓存存储目录",
            "建议放到空间较大的磁盘，集中管理缓存。"
        ),
    );
    build_perf.insert("RUSTFLAGS".to_string(), env_var_entry!(
        None,
        Some("-C link-arg=-fuse-ld=lld"),
        "传递给 rustc 的额外编译标志（加速链接）",
        "使用 LLD 链接器可显著加快链接速度。MSVC 工具链下需配合 -C linker=rust-lld，更推荐在项目 .cargo/config.toml 中针对 target 配置，避免全局环境变量冲突。"
    ));
    build_perf.insert(
        "CARGO_INCREMENTAL".to_string(),
        env_var_entry!(
            None,
            Some("1"),
            "启用/禁用增量编译",
            "默认开启，一般无需修改。设为 0 可关闭，在 CI 场景下可能减少磁盘消耗。"
        ),
    );
    build_perf.insert(
        "CARGO_JOBS".to_string(),
        env_var_entry!(
            None,
            Some("(CPU 逻辑核心数)"),
            "并行编译任务数",
            "默认等于 CPU 逻辑核心数，虚拟机或内存紧张时可调小。"
        ),
    );
    result.insert("build_perf".to_string(), build_perf);

    // ── 调试与诊断 ──
    let mut debug_diag = HashMap::new();
    debug_diag.insert(
        "RUST_BACKTRACE".to_string(),
        env_var_entry!(
            None,
            Some("1"),
            "控制 panic 时的回溯输出",
            "开发时建议设为 1 或 full，能显示完整调用栈。full 包含内联帧信息。"
        ),
    );
    debug_diag.insert(
        "RUST_LOG".to_string(),
        env_var_entry!(
            None,
            Some("debug"),
            "控制 Rust 生态工具（如 rustup、cargo、rustc）的日志级别",
            "按需设置，如 RUST_LOG=cargo::ops::resolve=trace 仅打印依赖解析日志，用于排查问题。"
        ),
    );
    debug_diag.insert("RUSTFLAGS_DEBUG".to_string(), env_var_entry!(
            None,
            Some("-C debuginfo=2"),
        "生成完整调试信息（用于调试 release 模式）",
        "若需调试 release 模式，可在 RUSTFLAGS 中加入此标志。注意与其他 RUSTFLAGS 设置合并使用。"
    ));
    debug_diag.insert(
        "CARGO_TERM_COLOR".to_string(),
        env_var_entry!(
            None,
            Some("auto"),
            "终端输出颜色",
            "默认自动检测，可强制设为 always 或 never。"
        ),
    );
    result.insert("debug_diag".to_string(), debug_diag);

    // ── 其他实用变量 ──
    let mut misc = HashMap::new();
    misc.insert(
        "CARGO_REGISTRIES_CRATES_IO_PROTOCOL".to_string(),
        env_var_entry!(
            None,
            Some("git (新版本 Cargo 已默认启用 sparse)"),
            "指定 crates.io 索引协议",
            "启用更快的稀疏索引，如遇 git 协议被墙尤其有用。可显式指定以防回退。"
        ),
    );
    misc.insert(
        "CARGO_BUILD_TARGET".to_string(),
        env_var_entry!(
            None,
            Some("x86_64-pc-windows-msvc"),
            "指定默认编译目标",
            "当需要交叉编译或固定目标平台时使用。"
        ),
    );
    misc.insert(
        "RUSTUP_DIST_SERVER".to_string(),
        env_var_entry!(
            None,
            Some("https://static.rust-lang.org"),
            "自定义 rustup 工具链下载源",
            "用于镜像加速下载工具链，国内用户推荐设置为中科大或清华镜像。"
        ),
    );
    misc.insert(
        "RUSTUP_UPDATE_ROOT".to_string(),
        env_var_entry!(
            None,
            Some("https://static.rust-lang.org/rustup"),
            "自定义 rustup 升级服务器",
            "用于镜像加速 rustup 自身升级。"
        ),
    );
    misc.insert(
        "EDITOR".to_string(),
        env_var_entry!(
            None,
            Some("code.cmd"),
            "某些 Rust 工具（如 cargo config --edit）调用的编辑器",
            "可设为 code.cmd (VS Code)、notepad++.exe 等可执行程序。"
        ),
    );
    misc.insert(
        "VISUAL".to_string(),
        env_var_entry!(
            None,
            Some("code.cmd"),
            "类似 EDITOR，某些工具优先读取 VISUAL",
            "作用与 EDITOR 相同，但优先级可能更高，建议与 EDITOR 设为一致。"
        ),
    );
    result.insert("misc".to_string(), misc);

    result
}

// ---------------------------------------------------------------------------
// Internal: seed default data into a fresh database
// ---------------------------------------------------------------------------

fn seed_simple_defaults(write_tx: &WriteTransaction) -> Result<(), redb::Error> {
    let mut table = write_tx.open_table(SIMPLE)?;
    table.insert("app.name", default_app_name().as_str())?;
    table.insert("app.version", default_app_version().as_str())?;
    table.insert("app.description", default_app_description().as_str())?;
    table.insert("binaries.rustup", default_rustup().as_str())?;
    table.insert("binaries.cargo", default_cargo().as_str())?;
    table.insert(
        "paths.cargo_bin_relative",
        default_cargo_bin_relative().as_str(),
    )?;
    table.insert("locale.force_locale", default_force_locale().as_str())?;
    table.insert(
        "timeouts.cargo_search_seconds",
        default_cargo_search_seconds().to_string().as_str(),
    )?;
    table.insert(
        "timeouts.rustup_check_seconds",
        default_rustup_check_seconds().to_string().as_str(),
    )?;
    table.insert("events.install_log", default_install_log().as_str())?;
    table.insert(
        "events.install_finished",
        default_install_finished().as_str(),
    )?;
    table.insert(
        "events.plugin_install_log",
        default_plugin_install_log().as_str(),
    )?;
    table.insert(
        "events.plugin_install_finished",
        default_plugin_install_finished().as_str(),
    )?;
    table.insert("events.update_log", default_update_log().as_str())?;
    table.insert("events.update_finished", default_update_finished().as_str())?;
    table.insert("parsing.default_marker", default_default_marker().as_str())?;
    table.insert("parsing.active_marker", default_active_marker().as_str())?;
    table.insert(
        "parsing.installed_marker",
        default_installed_marker().as_str(),
    )?;
    table.insert("parsing.no_overrides", default_no_overrides().as_str())?;
    table.insert("parsing.up_to_date", default_up_to_date().as_str())?;
    table.insert(
        "parsing.update_available",
        default_update_available().as_str(),
    )?;
    table.insert(
        "parsing.version_separator",
        default_version_separator().as_str(),
    )?;
    table.insert(
        "parsing.status_separator",
        default_status_separator().as_str(),
    )?;
    table.insert("parsing.cargo_prefix", default_cargo_prefix().as_str())?;
    Ok(())
}

fn seed_plugin_defaults(write_tx: &WriteTransaction) -> Result<(), redb::Error> {
    let mut table = write_tx.open_table(PLUGINS)?;
    let names = default_plugin_names();
    let data = serde_json::to_vec(&names).expect("serializing plugin names should not fail");
    table.insert("official", data.as_slice())?;
    Ok(())
}

fn seed_env_var_defaults(write_tx: &WriteTransaction) -> Result<(), redb::Error> {
    let mut table = write_tx.open_table(ENV_VARS)?;
    let defaults = crate::config::EnvVarsConfig::default();

    let categories: [(&str, &HashMap<String, EnvVarEntryConfig>); 5] = [
        ("paths_cache", &defaults.paths_cache),
        ("network_proxy", &defaults.network_proxy),
        ("build_perf", &defaults.build_perf),
        ("debug_diag", &defaults.debug_diag),
        ("misc", &defaults.misc),
    ];

    for (category, vars) in categories {
        for (var_name, entry) in vars {
            let key = format!("{category}::{var_name}");
            let data = serde_json::to_vec(entry).expect("serializing env var should not fail");
            table.insert(key.as_str(), data.as_slice())?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EnvVarEntryConfig;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_db() -> Database {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rustverse_test_db_{id}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.redb");
        Database::create(&path).unwrap()
    }

    #[test]
    fn test_open_or_create_seeds_defaults() {
        let dir = std::env::temp_dir().join("rustverse_test_open_create");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.redb");

        let db = open_or_create(&path).unwrap();

        // Verify simple values are seeded
        assert_eq!(
            get_simple(&db, "binaries.rustup"),
            Some("rustup".to_string())
        );
        assert_eq!(get_simple(&db, "binaries.cargo"), Some("cargo".to_string()));
        assert_eq!(
            get_simple(&db, "locale.force_locale"),
            Some("C".to_string())
        );
        assert_eq!(
            get_simple(&db, "timeouts.cargo_search_seconds"),
            Some("30".to_string())
        );

        // Verify plugin names
        let names = get_plugin_names(&db);
        assert!(names.contains(&"cargo-clippy".to_string()));
        assert!(names.contains(&"miri".to_string()));

        // Verify env vars
        let env_vars = get_env_vars(&db);
        assert!(env_vars.contains_key("paths_cache"));
        assert!(env_vars["paths_cache"].contains_key("RUSTUP_HOME"));
    }

    #[test]
    fn test_simple_crud() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        // Read default
        assert_eq!(
            get_simple(&db, "locale.force_locale"),
            Some("C".to_string())
        );

        // Update
        set_simple(&db, "locale.force_locale", "zh_CN").unwrap();
        assert_eq!(
            get_simple(&db, "locale.force_locale"),
            Some("zh_CN".to_string())
        );

        // Delete
        let removed = delete_simple(&db, "locale.force_locale").unwrap();
        assert!(removed);
        assert_eq!(get_simple(&db, "locale.force_locale"), None);

        // Delete non-existent
        let removed = delete_simple(&db, "nonexistent").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_simple_batch_read() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        let batch = get_simple_batch(&db, &["binaries.rustup", "binaries.cargo", "nonexistent"]);
        assert_eq!(batch["binaries.rustup"], "rustup");
        assert_eq!(batch["binaries.cargo"], "cargo");
        assert!(!batch.contains_key("nonexistent"));
    }

    #[test]
    fn test_plugin_names_crud() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        let mut names = get_plugin_names(&db);
        names.push("cargo-custom".to_string());
        set_plugin_names(&db, &names).unwrap();

        let loaded = get_plugin_names(&db);
        assert!(loaded.contains(&"cargo-custom".to_string()));
        assert_eq!(loaded.len(), names.len());
    }

    #[test]
    fn test_env_vars_crud() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        // Read default
        let rustup_vars = get_env_vars_by_category(&db, "paths_cache");
        assert!(rustup_vars.contains_key("RUSTUP_HOME"));

        // Add new entry
        let entry = EnvVarEntryConfig {
            rec: Some("/custom".to_string()),
            def: None,
            description: "Custom var".to_string(),
            notes: "Test note".to_string(),
        };
        set_env_var_entry(&db, "paths_cache", "CUSTOM_VAR", &entry).unwrap();

        let rustup_vars = get_env_vars_by_category(&db, "paths_cache");
        assert!(rustup_vars.contains_key("CUSTOM_VAR"));
        assert_eq!(rustup_vars["CUSTOM_VAR"].description, "Custom var");

        // Delete entry
        let removed = delete_env_var_entry(&db, "paths_cache", "CUSTOM_VAR").unwrap();
        assert!(removed);

        let rustup_vars = get_env_vars_by_category(&db, "paths_cache");
        assert!(!rustup_vars.contains_key("CUSTOM_VAR"));
    }

    #[test]
    fn test_binaries_config() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        let (rustup, cargo) = get_binaries_config(&db);
        assert_eq!(rustup, "rustup");
        assert_eq!(cargo, "cargo");
    }

    #[test]
    fn test_events_config() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        let events = get_events_config(&db);
        assert_eq!(events.install_log, "install-log");
        assert_eq!(events.update_finished, "update-finished");
    }

    #[test]
    fn test_parsing_config() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        let parsing = get_parsing_config(&db);
        assert_eq!(parsing.default_marker, "(default)");
        assert_eq!(parsing.cargo_prefix, "cargo-");
    }

    #[test]
    fn test_get_simple_missing_key_returns_none() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        assert_eq!(get_simple(&db, "nonexistent.key"), None);
    }

    #[test]
    fn test_env_vars_by_category_nonexistent() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        let result = get_env_vars_by_category(&db, "nonexistent");
        assert!(result.is_empty());
    }

    #[test]
    fn test_reopen_database_preserves_data() {
        let dir = std::env::temp_dir().join("rustverse_test_reopen");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.redb");

        {
            let db = open_or_create(&path).unwrap();
            set_simple(&db, "locale.force_locale", "zh_CN").unwrap();
        }

        // Reopen
        let db = Database::create(&path).unwrap();
        assert_eq!(
            get_simple(&db, "locale.force_locale"),
            Some("zh_CN".to_string())
        );
    }
}

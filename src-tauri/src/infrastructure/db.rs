use std::collections::HashMap;
use std::path::Path;

use redb::{
    Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition,
    WriteTransaction,
};

use crate::domain::config_keys::keys;
use crate::infrastructure::config::EnvVarEntryConfig;
use crate::infrastructure::config::defaults;

// ---------------------------------------------------------------------------
// Table definitions
// ---------------------------------------------------------------------------

/// Simple key-value config (binaries, paths, locale, timeouts, events, parsing).
const SIMPLE: TableDefinition<&str, &str> = TableDefinition::new("config_simple");

/// Official plugin names list.
const PLUGINS: TableDefinition<&str, &[u8]> = TableDefinition::new("config_plugins");

/// Environment variable metadata entries.
const ENV_VARS: TableDefinition<&str, &[u8]> = TableDefinition::new("config_env_vars");

/// Notification records — maps u64 ID → JSON payload.
const NOTIFICATIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("notifications");

/// Notification auto-increment counter (single key "next_id" → u64).
const NOTIF_COUNTER: TableDefinition<&str, u64> = TableDefinition::new("notif_counter");

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
    let write_tx = db.begin_write()?;
    {
        let table = write_tx.open_table(SIMPLE)?;
        if table.len()? > 0 {
            return Ok(());
        }
    }
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

/// Write multiple string values to the `config_simple` table in a single transaction.
pub fn set_simple_batch(db: &Database, entries: &[(&str, &str)]) -> Result<(), redb::Error> {
    let write_tx = db.begin_write()?;
    {
        let mut table = write_tx.open_table(SIMPLE)?;
        for (key, value) in entries {
            table.insert(*key, *value)?;
        }
    }
    write_tx.commit()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Plugin names CRUD
// ---------------------------------------------------------------------------

/// Get the official plugin names list.
pub fn get_plugin_names(db: &Database) -> Vec<String> {
    let Ok(read_tx) = db.begin_read() else {
        return defaults::plugin_names();
    };
    let Ok(table) = read_tx.open_table(PLUGINS) else {
        return defaults::plugin_names();
    };
    let Ok(Some(guard)) = table.get("official") else {
        return defaults::plugin_names();
    };
    serde_json::from_slice(guard.value()).unwrap_or_else(|_| defaults::plugin_names())
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
        return defaults::env_vars();
    };
    let Ok(table) = read_tx.open_table(ENV_VARS) else {
        return defaults::env_vars();
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
        return defaults::env_vars();
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
// Settings (application-level user preferences)
// ---------------------------------------------------------------------------

const SETTINGS_KEY: &str = "settings";

/// Read user settings as a JSON string from the `config_simple` table.
pub fn get_settings_json(db: &Database) -> Option<String> {
    get_simple(db, SETTINGS_KEY)
}

/// Write user settings as a JSON string to the `config_simple` table.
pub fn set_settings_json(db: &Database, json: &str) -> Result<(), redb::Error> {
    set_simple(db, SETTINGS_KEY, json)
}

// ---------------------------------------------------------------------------
// Batch config readers (one transaction per config section)
// ---------------------------------------------------------------------------

/// Read all app metadata values.
pub fn get_app_metadata(repo: &dyn ConfigRepository) -> (String, String, String) {
    // If config.toml exists, always prefer its values over database
    if has_config_file() {
        return get_app_metadata_from_config();
    }

    // Otherwise fall back to database values
    let batch = repo.get_config_batch(&[keys::APP_NAME, keys::APP_VERSION, keys::APP_DESCRIPTION]);
    (
        batch
            .get(keys::APP_NAME)
            .cloned()
            .unwrap_or_else(defaults::app_name),
        batch
            .get(keys::APP_VERSION)
            .cloned()
            .unwrap_or_else(defaults::app_version),
        batch
            .get(keys::APP_DESCRIPTION)
            .cloned()
            .unwrap_or_else(defaults::app_description),
    )
}

pub fn ensure_version_in_db(db: &Database) {
    let current_version = env!("CARGO_PKG_VERSION");
    let stored = get_simple(db, keys::APP_VERSION);
    if stored.as_deref() != Some(current_version) {
        let _ = set_simple(db, keys::APP_VERSION, current_version);
        crate::infrastructure::logger::logger().info(
            crate::domain::constants::log_module::STARTUP,
            &format!(
                "Updated stored app version: {:?} -> {}",
                stored, current_version
            ),
        );
    }
}

/// Check if config.toml or config.toml.migrated exists and is readable.
fn has_config_file() -> bool {
    let exe_dir = crate::infrastructure::app_paths::app_paths().exe_dir().clone();

    let toml_paths = [
        exe_dir.join("config.toml"),
        exe_dir.join("config.toml.migrated"),
    ];

    for toml_path in &toml_paths {
        if toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(toml_path) {
                if toml::from_str::<crate::infrastructure::config::AppConfig>(&content).is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

/// Try to read app metadata from config.toml or config.toml.migrated.
fn get_app_metadata_from_config() -> (String, String, String) {
    let exe_dir = crate::infrastructure::app_paths::app_paths().exe_dir();

    let toml_paths = [
        exe_dir.join("config.toml"),
        exe_dir.join("config.toml.migrated"),
    ];

    for toml_path in &toml_paths {
        if toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(toml_path) {
                if let Ok(config) =
                    toml::from_str::<crate::infrastructure::config::AppConfig>(&content)
                {
                    return (config.app.name, config.app.version, config.app.description);
                }
            }
        }
    }

    (
        defaults::app_name(),
        defaults::app_version(),
        defaults::app_description(),
    )
}

/// Read all binaries config values.
pub fn get_binaries_config(repo: &dyn ConfigRepository) -> (String, String) {
    let batch = repo.get_config_batch(&[keys::BIN_RUSTUP, keys::BIN_CARGO]);
    let rustup = batch
        .get(keys::BIN_RUSTUP)
        .cloned()
        .unwrap_or_else(defaults::rustup);
    let cargo = batch
        .get(keys::BIN_CARGO)
        .cloned()
        .unwrap_or_else(defaults::cargo);
    (rustup, cargo)
}

/// Read all events config values.
pub fn get_events_config(repo: &dyn ConfigRepository) -> EventsConfigValues {
    let batch = repo.get_config_batch(&[
        keys::EVENT_INSTALL_LOG,
        keys::EVENT_INSTALL_FINISHED,
        keys::EVENT_PLUGIN_INSTALL_LOG,
        keys::EVENT_PLUGIN_INSTALL_FINISHED,
        keys::EVENT_UPDATE_LOG,
        keys::EVENT_UPDATE_FINISHED,
    ]);
    EventsConfigValues {
        install_log: batch
            .get(keys::EVENT_INSTALL_LOG)
            .cloned()
            .unwrap_or_else(defaults::install_log),
        install_finished: batch
            .get(keys::EVENT_INSTALL_FINISHED)
            .cloned()
            .unwrap_or_else(defaults::install_finished),
        plugin_install_log: batch
            .get(keys::EVENT_PLUGIN_INSTALL_LOG)
            .cloned()
            .unwrap_or_else(defaults::plugin_install_log),
        plugin_install_finished: batch
            .get(keys::EVENT_PLUGIN_INSTALL_FINISHED)
            .cloned()
            .unwrap_or_else(defaults::plugin_install_finished),
        update_log: batch
            .get(keys::EVENT_UPDATE_LOG)
            .cloned()
            .unwrap_or_else(defaults::update_log),
        update_finished: batch
            .get(keys::EVENT_UPDATE_FINISHED)
            .cloned()
            .unwrap_or_else(defaults::update_finished),
    }
}

/// Read all parsing config values.
pub fn get_parsing_config(repo: &dyn ConfigRepository) -> ParsingConfigValues {
    let batch = repo.get_config_batch(&[
        keys::PARSING_DEFAULT_MARKER,
        keys::PARSING_ACTIVE_MARKER,
        keys::PARSING_INSTALLED_MARKER,
        keys::PARSING_NO_OVERRIDES,
        keys::PARSING_UP_TO_DATE,
        keys::PARSING_UPDATE_AVAILABLE,
        keys::PARSING_VERSION_SEPARATOR,
        keys::PARSING_STATUS_SEPARATOR,
        keys::PARSING_CARGO_PREFIX,
    ]);
    ParsingConfigValues {
        default_marker: batch
            .get(keys::PARSING_DEFAULT_MARKER)
            .cloned()
            .unwrap_or_else(defaults::default_marker),
        active_marker: batch
            .get(keys::PARSING_ACTIVE_MARKER)
            .cloned()
            .unwrap_or_else(defaults::active_marker),
        installed_marker: batch
            .get(keys::PARSING_INSTALLED_MARKER)
            .cloned()
            .unwrap_or_else(defaults::installed_marker),
        no_overrides: batch
            .get(keys::PARSING_NO_OVERRIDES)
            .cloned()
            .unwrap_or_else(defaults::no_overrides),
        up_to_date: batch
            .get(keys::PARSING_UP_TO_DATE)
            .cloned()
            .unwrap_or_else(defaults::up_to_date),
        update_available: batch
            .get(keys::PARSING_UPDATE_AVAILABLE)
            .cloned()
            .unwrap_or_else(defaults::update_available),
        version_separator: batch
            .get(keys::PARSING_VERSION_SEPARATOR)
            .cloned()
            .unwrap_or_else(defaults::version_separator),
        status_separator: batch
            .get(keys::PARSING_STATUS_SEPARATOR)
            .cloned()
            .unwrap_or_else(defaults::status_separator),
        cargo_prefix: batch
            .get(keys::PARSING_CARGO_PREFIX)
            .cloned()
            .unwrap_or_else(defaults::cargo_prefix),
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
    let app_config: crate::infrastructure::config::AppConfig =
        toml::from_str(&content).map_err(|e| format!("failed to parse config.toml: {e}"))?;

    let defaults = crate::infrastructure::config::AppConfig::default();
    let is_default = app_config == defaults;

    let write_tx = db.begin_write().map_err(|e| format!("db write tx: {e}"))?;

    // Always write app metadata from config.toml to ensure database stays in sync
    {
        let mut table = write_tx
            .open_table(SIMPLE)
            .map_err(|e| format!("open SIMPLE: {e}"))?;

        table
            .insert(keys::APP_NAME, app_config.app.name.as_str())
            .map_err(|e| format!("insert app.name: {e}"))?;
        table
            .insert(keys::APP_VERSION, app_config.app.version.as_str())
            .map_err(|e| format!("insert app.version: {e}"))?;
        table
            .insert(keys::APP_DESCRIPTION, app_config.app.description.as_str())
            .map_err(|e| format!("insert app.description: {e}"))?;
    }

    if is_default {
        write_tx.commit().map_err(|e| format!("commit: {e}"))?;
        return Ok(false); // nothing else to migrate
    }

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
            keys::BIN_RUSTUP,
            app_config.binaries.rustup,
            defaults.binaries.rustup
        );
        maybe_write!(
            keys::BIN_CARGO,
            app_config.binaries.cargo,
            defaults.binaries.cargo
        );
        maybe_write!(
            keys::PATHS_CARGO_BIN_RELATIVE,
            app_config.paths.cargo_bin_relative,
            defaults.paths.cargo_bin_relative
        );
        maybe_write!(
            keys::LOCALE_FORCE,
            app_config.locale.force_locale,
            defaults.locale.force_locale
        );
        // Locale codes and metadata (stored as JSON)
        if app_config.locale.codes != defaults.locale.codes {
            let codes_json = serde_json::to_string(&app_config.locale.codes)
                .unwrap_or_else(|_| "[]".to_string());
            table
                .insert(keys::LOCALE_CODES, codes_json.as_str())
                .map_err(|e| format!("insert locale.codes: {e}"))?;
        }
        if app_config.locale.meta != defaults.locale.meta {
            let meta_json =
                serde_json::to_string(&app_config.locale.meta).unwrap_or_else(|_| "{}".to_string());
            table
                .insert(keys::LOCALE_META, meta_json.as_str())
                .map_err(|e| format!("insert locale.meta: {e}"))?;
        }
        if app_config.timeouts.cargo_search_seconds != defaults.timeouts.cargo_search_seconds {
            table
                .insert(
                    keys::TIMEOUT_CARGO_SEARCH,
                    app_config
                        .timeouts
                        .cargo_search_seconds
                        .to_string()
                        .as_str(),
                )
                .map_err(|e| format!("insert timeouts.cargo_search_seconds: {e}"))?;
        }
        maybe_write!(
            keys::EVENT_INSTALL_LOG,
            app_config.events.install_log,
            defaults.events.install_log
        );
        maybe_write!(
            keys::EVENT_INSTALL_FINISHED,
            app_config.events.install_finished,
            defaults.events.install_finished
        );
        maybe_write!(
            keys::EVENT_PLUGIN_INSTALL_LOG,
            app_config.events.plugin_install_log,
            defaults.events.plugin_install_log
        );
        maybe_write!(
            keys::EVENT_PLUGIN_INSTALL_FINISHED,
            app_config.events.plugin_install_finished,
            defaults.events.plugin_install_finished
        );
        maybe_write!(
            keys::EVENT_UPDATE_LOG,
            app_config.events.update_log,
            defaults.events.update_log
        );
        maybe_write!(
            keys::EVENT_UPDATE_FINISHED,
            app_config.events.update_finished,
            defaults.events.update_finished
        );
        maybe_write!(
            keys::PARSING_DEFAULT_MARKER,
            app_config.parsing.default_marker,
            defaults.parsing.default_marker
        );
        maybe_write!(
            keys::PARSING_ACTIVE_MARKER,
            app_config.parsing.active_marker,
            defaults.parsing.active_marker
        );
        maybe_write!(
            keys::PARSING_INSTALLED_MARKER,
            app_config.parsing.installed_marker,
            defaults.parsing.installed_marker
        );
        maybe_write!(
            keys::PARSING_NO_OVERRIDES,
            app_config.parsing.no_overrides,
            defaults.parsing.no_overrides
        );
        maybe_write!(
            keys::PARSING_UP_TO_DATE,
            app_config.parsing.up_to_date,
            defaults.parsing.up_to_date
        );
        maybe_write!(
            keys::PARSING_UPDATE_AVAILABLE,
            app_config.parsing.update_available,
            defaults.parsing.update_available
        );
        maybe_write!(
            keys::PARSING_VERSION_SEPARATOR,
            app_config.parsing.version_separator,
            defaults.parsing.version_separator
        );
        maybe_write!(
            keys::PARSING_STATUS_SEPARATOR,
            app_config.parsing.status_separator,
            defaults.parsing.status_separator
        );
        maybe_write!(
            keys::PARSING_CARGO_PREFIX,
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
// Internal: seed default data into a fresh database
// ---------------------------------------------------------------------------

fn seed_simple_defaults(write_tx: &WriteTransaction) -> Result<(), redb::Error> {
    let mut table = write_tx.open_table(SIMPLE)?;
    table.insert(keys::APP_NAME, defaults::app_name().as_str())?;
    table.insert(keys::APP_VERSION, defaults::app_version().as_str())?;
    table.insert(keys::APP_DESCRIPTION, defaults::app_description().as_str())?;
    table.insert(keys::BIN_RUSTUP, defaults::rustup().as_str())?;
    table.insert(keys::BIN_CARGO, defaults::cargo().as_str())?;
    table.insert(
        keys::PATHS_CARGO_BIN_RELATIVE,
        defaults::cargo_bin_relative().as_str(),
    )?;
    table.insert(keys::LOCALE_FORCE, defaults::force_locale().as_str())?;
    table.insert(
        keys::TIMEOUT_CARGO_SEARCH,
        defaults::cargo_search_seconds().to_string().as_str(),
    )?;
    table.insert(
        keys::TIMEOUT_RUSTUP_CHECK,
        defaults::rustup_check_seconds().to_string().as_str(),
    )?;
    table.insert(keys::EVENT_INSTALL_LOG, defaults::install_log().as_str())?;
    table.insert(
        keys::EVENT_INSTALL_FINISHED,
        defaults::install_finished().as_str(),
    )?;
    table.insert(
        keys::EVENT_PLUGIN_INSTALL_LOG,
        defaults::plugin_install_log().as_str(),
    )?;
    table.insert(
        keys::EVENT_PLUGIN_INSTALL_FINISHED,
        defaults::plugin_install_finished().as_str(),
    )?;
    table.insert(keys::EVENT_UPDATE_LOG, defaults::update_log().as_str())?;
    table.insert(
        keys::EVENT_UPDATE_FINISHED,
        defaults::update_finished().as_str(),
    )?;
    table.insert(
        keys::PARSING_DEFAULT_MARKER,
        defaults::default_marker().as_str(),
    )?;
    table.insert(
        keys::PARSING_ACTIVE_MARKER,
        defaults::active_marker().as_str(),
    )?;
    table.insert(
        keys::PARSING_INSTALLED_MARKER,
        defaults::installed_marker().as_str(),
    )?;
    table.insert(
        keys::PARSING_NO_OVERRIDES,
        defaults::no_overrides().as_str(),
    )?;
    table.insert(keys::PARSING_UP_TO_DATE, defaults::up_to_date().as_str())?;
    table.insert(
        keys::PARSING_UPDATE_AVAILABLE,
        defaults::update_available().as_str(),
    )?;
    table.insert(
        keys::PARSING_VERSION_SEPARATOR,
        defaults::version_separator().as_str(),
    )?;
    table.insert(
        keys::PARSING_STATUS_SEPARATOR,
        defaults::status_separator().as_str(),
    )?;
    table.insert(
        keys::PARSING_CARGO_PREFIX,
        defaults::cargo_prefix().as_str(),
    )?;
    Ok(())
}

fn seed_plugin_defaults(write_tx: &WriteTransaction) -> Result<(), redb::Error> {
    let mut table = write_tx.open_table(PLUGINS)?;
    let names = defaults::plugin_names();
    let data = serde_json::to_vec(&names).expect("serializing plugin names should not fail");
    table.insert("official", data.as_slice())?;
    Ok(())
}

fn seed_env_var_defaults(write_tx: &WriteTransaction) -> Result<(), redb::Error> {
    let table = write_tx.open_table(ENV_VARS)?;
    // Guard: if env vars are already initialized, skip re-seeding to
    // preserve any user customizations (e.g. descriptions, notes).
    if table.len()? > 0 {
        return Ok(());
    }
    drop(table);
    let mut table = write_tx.open_table(ENV_VARS)?;
    let defaults = crate::infrastructure::config::EnvVarsConfig::default();

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
// RedbDataStore — implements domain repository traits on top of redb
// ---------------------------------------------------------------------------

use std::sync::Arc;

use crate::domain::repository::{
    ConfigRepository, DataStore, EnvVarRepository, NotificationRepository, PluginRepository,
    RepositoryError, SettingsRepository,
};

/// Concrete redb-backed implementation of all repository traits.
///
/// Created once in `lib.rs` and shared via `Arc<dyn DataStore>` throughout the application.
#[derive(Clone)]
pub struct RedbDataStore {
    db: Arc<redb::Database>,
}

impl RedbDataStore {
    pub fn new(db: Arc<redb::Database>) -> Self {
        Self { db }
    }

    #[allow(dead_code)]
    pub fn inner_db(&self) -> Arc<redb::Database> {
        Arc::clone(&self.db)
    }
}

impl ConfigRepository for RedbDataStore {
    fn get_config(&self, key: &str) -> Option<String> {
        get_simple(&*self.db, key)
    }

    fn set_config(&self, key: &str, value: &str) -> Result<(), RepositoryError> {
        set_simple(&*self.db, key, value).map_err(|e| RepositoryError::Database(e.to_string()))
    }

    fn get_config_batch(&self, keys: &[&str]) -> std::collections::HashMap<String, String> {
        get_simple_batch(&*self.db, keys)
    }

    fn set_config_batch(&self, entries: &[(&str, &str)]) -> Result<(), RepositoryError> {
        set_simple_batch(&*self.db, entries).map_err(|e| RepositoryError::Database(e.to_string()))
    }
}

impl EnvVarRepository for RedbDataStore {
    fn get_env_var_metas(
        &self,
    ) -> std::collections::HashMap<String, std::collections::HashMap<String, EnvVarEntryConfig>>
    {
        get_env_vars(&*self.db)
    }

    fn set_env_var_meta(
        &self,
        category: &str,
        name: &str,
        entry: &EnvVarEntryConfig,
    ) -> Result<(), RepositoryError> {
        set_env_var_entry(&*self.db, category, name, entry)
            .map_err(|e| RepositoryError::Database(e.to_string()))
    }

    fn delete_env_var_meta(&self, category: &str, name: &str) -> Result<bool, RepositoryError> {
        delete_env_var_entry(&*self.db, category, name)
            .map_err(|e| RepositoryError::Database(e.to_string()))
    }
}

impl PluginRepository for RedbDataStore {
    fn get_plugin_names(&self) -> Vec<String> {
        get_plugin_names(&*self.db)
    }

    fn set_plugin_names(&self, names: &[String]) -> Result<(), RepositoryError> {
        set_plugin_names(&*self.db, names).map_err(|e| RepositoryError::Database(e.to_string()))
    }
}

impl SettingsRepository for RedbDataStore {
    fn get_settings(&self) -> Option<String> {
        get_settings_json(&*self.db)
    }

    fn set_settings(&self, json: &str) -> Result<(), RepositoryError> {
        set_settings_json(&*self.db, json).map_err(|e| RepositoryError::Database(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Notification CRUD (infrastructure layer — DB operations for notifications)
// ---------------------------------------------------------------------------

use crate::domain::base::time::chrono_now_ms;
use crate::domain::notification::{NewNotification, Notification};

/// Ensure the notifications table exists (no-op in redb — tables are created on first use).
#[allow(dead_code)]
pub fn notification_ensure_table(_db: &Database) -> Result<(), String> {
    Ok(())
}

/// Insert a new notification, assigning an auto-increment ID.
///
/// `sound_enabled` and `default_priority` are snapshots of the current user
/// settings at notification creation time.  Callers that have access to the
/// `UserSettings` should pass real values; legacy callers (e.g. the
/// `notify_create` Tauri command) may pass `false` / `""`.
pub fn insert_notification(
    db: &Database,
    new: &NewNotification,
    sound_enabled: bool,
    default_priority: &str,
) -> Result<u64, String> {
    let write_tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut counter_table = write_tx
            .open_table(NOTIF_COUNTER)
            .map_err(|e| e.to_string())?;
        let next_id = counter_table
            .get("next_id")
            .map_err(|e| e.to_string())?
            .map(|g| g.value())
            .unwrap_or(1);

        let notification = Notification {
            id: next_id,
            category: new.category.clone(),
            priority: new.priority.clone(),
            title: new.title.clone(),
            body: new.body.clone(),
            notif_key: new.notif_key.clone(),
            params_json: new.params_json.clone(),
            action_route: new.action_route.clone(),
            is_read: false,
            sound_enabled,
            default_priority: default_priority.to_string(),
            created_at: chrono_now_ms(),
        };

        let json = serde_json::to_string(&notification).map_err(|e| e.to_string())?;
        let mut notif_table = write_tx
            .open_table(NOTIFICATIONS)
            .map_err(|e| e.to_string())?;
        notif_table
            .insert(next_id, json.as_bytes())
            .map_err(|e| e.to_string())?;

        counter_table
            .insert("next_id", next_id + 1)
            .map_err(|e| e.to_string())?;
    }
    write_tx.commit().map_err(|e| e.to_string())?;

    // Return the assigned ID
    let read_tx = db.begin_read().map_err(|e| e.to_string())?;
    let counter_table = read_tx
        .open_table(NOTIF_COUNTER)
        .map_err(|e| e.to_string())?;
    Ok(counter_table
        .get("next_id")
        .map_err(|e| e.to_string())?
        .map(|g| g.value())
        .unwrap_or(1)
        - 1)
}

/// List all notifications.
pub fn list_notifications(db: &Database) -> Result<Vec<Notification>, String> {
    let read_tx = db.begin_read().map_err(|e| e.to_string())?;
    let Ok(table) = read_tx.open_table(NOTIFICATIONS) else {
        return Ok(vec![]);
    };
    let mut result = vec![];
    for res in table.iter().map_err(|e| e.to_string())? {
        let (_id, guard) = res.map_err(|e| e.to_string())?;
        let bytes = guard.value();
        let notif: Notification = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        result.push(notif);
    }
    Ok(result)
}

/// Mark a notification as read.
pub fn mark_read(db: &Database, id: u64) -> Result<(), String> {
    update_notification_field(db, id, |n| n.is_read = true)
}

/// Mark a notification as unread.
pub fn mark_unread(db: &Database, id: u64) -> Result<(), String> {
    update_notification_field(db, id, |n| n.is_read = false)
}

/// Delete a notification by ID.
pub fn delete_notification(db: &Database, id: u64) -> Result<(), String> {
    let write_tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_tx
            .open_table(NOTIFICATIONS)
            .map_err(|e| e.to_string())?;
        table.remove(id).map_err(|e| e.to_string())?;
    }
    write_tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete all notifications.
pub fn delete_all_notifications(db: &Database) -> Result<(), String> {
    let write_tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_tx
            .open_table(NOTIFICATIONS)
            .map_err(|e| e.to_string())?;
        let keys: Vec<u64> = table
            .iter()
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .map(|(k, _)| k.value())
            .collect();
        for key in keys {
            table.remove(key).map_err(|e| e.to_string())?;
        }
    }
    write_tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

/// Get unread notification count.
pub fn unread_count(db: &Database) -> Result<u64, String> {
    let notifications = list_notifications(db)?;
    Ok(notifications.iter().filter(|n| !n.is_read).count() as u64)
}

/// Delete all read notifications created before `cutoff_ms`.
pub fn delete_read_before(db: &Database, cutoff_ms: i64) -> Result<u64, String> {
    // Phase 1: collect IDs in a read transaction — no modifications so no cursor invalidation.
    let to_delete: Vec<u64> = {
        let read_tx = db.begin_read().map_err(|e| e.to_string())?;
        let table = read_tx
            .open_table(NOTIFICATIONS)
            .map_err(|e| e.to_string())?;
        table
            .iter()
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .filter_map(|(k, v)| {
                let id = k.value();
                let bytes = v.value();
                let notif: Notification = serde_json::from_slice(bytes).ok()?;
                if notif.is_read && notif.created_at < cutoff_ms {
                    Some(id)
                } else {
                    None
                }
            })
            .collect()
    };

    // Phase 2: delete in a separate write transaction so tree mutations don't
    // invalidate the Phase-1 read cursor.
    let deleted = to_delete.len() as u64;
    if deleted == 0 {
        return Ok(0);
    }
    let write_tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_tx
            .open_table(NOTIFICATIONS)
            .map_err(|e| e.to_string())?;
        for id in &to_delete {
            table.remove(*id).map_err(|e| e.to_string())?;
        }
    }
    write_tx.commit().map_err(|e| e.to_string())?;
    Ok(deleted)
}

/// Helper: update a single notification field by loading, modifying, and re-saving.
fn update_notification_field(
    db: &Database,
    id: u64,
    f: impl FnOnce(&mut Notification),
) -> Result<(), String> {
    // Read phase
    let notif = {
        let read_tx = db.begin_read().map_err(|e| e.to_string())?;
        let table = read_tx
            .open_table(NOTIFICATIONS)
            .map_err(|e| e.to_string())?;
        let guard = table.get(id).map_err(|e| e.to_string())?;
        let Some(guard) = guard else {
            return Err(format!("Notification {id} not found"));
        };
        let bytes: Vec<u8> = guard.value().to_vec();
        serde_json::from_slice::<Notification>(&bytes).map_err(|e| e.to_string())?
    };

    // Write phase
    let mut notif = notif;
    f(&mut notif);
    let json = serde_json::to_string(&notif).map_err(|e| e.to_string())?;
    let write_tx = db.begin_write().map_err(|e| e.to_string())?;
    {
        let mut table = write_tx
            .open_table(NOTIFICATIONS)
            .map_err(|e| e.to_string())?;
        table
            .insert(id, json.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    write_tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Repository impls
// ---------------------------------------------------------------------------

impl NotificationRepository for RedbDataStore {
    fn notification_ensure_table(&self) -> Result<(), RepositoryError> {
        notification_ensure_table(&*self.db).map_err(|e| RepositoryError::Database(e))
    }

    fn notification_insert(&self, json: &str) -> Result<u64, RepositoryError> {
        let new: crate::domain::notification::NewNotification = serde_json::from_str(json)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        insert_notification(&*self.db, &new, false, "")
            .map_err(|e| RepositoryError::Database(e))
    }

    fn notification_insert_with_settings(
        &self,
        json: &str,
        sound_enabled: bool,
        default_priority: &str,
    ) -> Result<u64, RepositoryError> {
        let new: crate::domain::notification::NewNotification = serde_json::from_str(json)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        insert_notification(&*self.db, &new, sound_enabled, default_priority)
            .map_err(|e| RepositoryError::Database(e))
    }

    fn notification_list(&self) -> Result<Vec<(u64, String)>, RepositoryError> {
        list_notifications(&*self.db)
            .map(|list| {
                list.into_iter()
                    .map(|n| {
                        let json = serde_json::to_string(&n).unwrap_or_default();
                        (n.id, json)
                    })
                    .collect()
            })
            .map_err(|e| RepositoryError::Database(e))
    }

    fn notification_mark_read(&self, id: u64) -> Result<(), RepositoryError> {
        mark_read(&*self.db, id).map_err(|e| RepositoryError::Database(e))
    }

    fn notification_mark_unread(&self, id: u64) -> Result<(), RepositoryError> {
        mark_unread(&*self.db, id).map_err(|e| RepositoryError::Database(e))
    }

    fn notification_delete(&self, id: u64) -> Result<(), RepositoryError> {
        delete_notification(&*self.db, id).map_err(|e| RepositoryError::Database(e))
    }

    fn notification_delete_all(&self) -> Result<(), RepositoryError> {
        delete_all_notifications(&*self.db).map_err(|e| RepositoryError::Database(e))
    }

    fn notification_unread_count(&self) -> Result<u64, RepositoryError> {
        unread_count(&*self.db).map_err(|e| RepositoryError::Database(e))
    }

    fn notification_delete_read_before(&self, cutoff_ms: i64) -> Result<u64, RepositoryError> {
        delete_read_before(&*self.db, cutoff_ms).map_err(|e| RepositoryError::Database(e))
    }
}

impl DataStore for RedbDataStore {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::EnvVarEntryConfig;
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
            get_simple(&db, keys::BIN_RUSTUP),
            Some("rustup".to_string())
        );
        assert_eq!(get_simple(&db, keys::BIN_CARGO), Some("cargo".to_string()));
        assert_eq!(get_simple(&db, keys::LOCALE_FORCE), Some("C".to_string()));
        assert_eq!(
            get_simple(&db, keys::TIMEOUT_CARGO_SEARCH),
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
        assert_eq!(get_simple(&db, keys::LOCALE_FORCE), Some("C".to_string()));

        // Update
        set_simple(&db, keys::LOCALE_FORCE, "zh_CN").unwrap();
        assert_eq!(
            get_simple(&db, keys::LOCALE_FORCE),
            Some("zh_CN".to_string())
        );

        // Delete
        let removed = delete_simple(&db, keys::LOCALE_FORCE).unwrap();
        assert!(removed);
        assert_eq!(get_simple(&db, keys::LOCALE_FORCE), None);

        // Delete non-existent
        let removed = delete_simple(&db, "nonexistent").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_simple_batch_read() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();

        let batch = get_simple_batch(&db, &[keys::BIN_RUSTUP, keys::BIN_CARGO, "nonexistent"]);
        assert_eq!(batch[keys::BIN_RUSTUP], "rustup");
        assert_eq!(batch[keys::BIN_CARGO], "cargo");
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
        let store = RedbDataStore::new(Arc::new(db));

        let (rustup, cargo) = get_binaries_config(&store);
        assert_eq!(rustup, "rustup");
        assert_eq!(cargo, "cargo");
    }

    #[test]
    fn test_events_config() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();
        let store = RedbDataStore::new(Arc::new(db));

        let events = get_events_config(&store);
        assert_eq!(events.install_log, "install-log");
        assert_eq!(events.update_finished, "update-finished");
    }

    #[test]
    fn test_parsing_config() {
        let db = test_db();
        seed_defaults_if_empty(&db).unwrap();
        let store = RedbDataStore::new(Arc::new(db));

        let parsing = get_parsing_config(&store);
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
            set_simple(&db, keys::LOCALE_FORCE, "zh_CN").unwrap();
        }

        // Reopen
        let db = Database::create(&path).unwrap();
        assert_eq!(
            get_simple(&db, keys::LOCALE_FORCE),
            Some("zh_CN".to_string())
        );
    }
}

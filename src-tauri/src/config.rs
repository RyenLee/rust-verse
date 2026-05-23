use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Top-level application configuration.
///
/// Struct definitions are retained for serialization to the frontend
/// and as the data model for `EnvVarEntryConfig` used by `db.rs`.
/// Actual data is now stored in the redb database.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub app: AppMetadataConfig,
    #[serde(default)]
    pub binaries: BinariesConfig,
    #[serde(default)]
    pub paths: PathsConfig,
    #[serde(default)]
    pub locale: LocaleConfig,
    #[serde(default)]
    pub timeouts: TimeoutsConfig,
    #[serde(default)]
    pub events: EventsConfig,
    #[serde(default)]
    pub plugins: PluginsConfig,
    #[serde(default)]
    pub parsing: ParsingConfig,
    #[serde(default)]
    pub env_vars: EnvVarsConfig,
}

/// Application metadata (name, version, description) from config.toml [app].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AppMetadataConfig {
    #[serde(default = "crate::db::default_app_name")]
    pub name: String,
    #[serde(default = "crate::db::default_app_version")]
    pub version: String,
    #[serde(default = "crate::db::default_app_description")]
    pub description: String,
}

impl Default for AppMetadataConfig {
    fn default() -> Self {
        Self {
            name: crate::db::default_app_name(),
            version: crate::db::default_app_version(),
            description: crate::db::default_app_description(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BinariesConfig {
    #[serde(default = "crate::db::default_rustup")]
    pub rustup: String,
    #[serde(default = "crate::db::default_cargo")]
    pub cargo: String,
}

impl Default for BinariesConfig {
    fn default() -> Self {
        Self {
            rustup: crate::db::default_rustup(),
            cargo: crate::db::default_cargo(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PathsConfig {
    #[serde(default = "crate::db::default_cargo_bin_relative")]
    pub cargo_bin_relative: String,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            cargo_bin_relative: crate::db::default_cargo_bin_relative(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LocaleConfig {
    #[serde(default = "crate::db::default_force_locale")]
    pub force_locale: String,
    /// List of available locale codes from config.toml (build-time generated).
    #[serde(default)]
    pub codes: Vec<String>,
    /// Metadata for each locale code (name, english_name).
    #[serde(default)]
    pub meta: HashMap<String, LocaleMeta>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LocaleMeta {
    pub name: String,
    pub english_name: String,
}

impl Default for LocaleConfig {
    fn default() -> Self {
        Self {
            force_locale: crate::db::default_force_locale(),
            codes: vec!["en".to_string()],
            meta: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TimeoutsConfig {
    #[serde(default = "crate::db::default_cargo_search_seconds")]
    pub cargo_search_seconds: u64,
}

impl Default for TimeoutsConfig {
    fn default() -> Self {
        Self {
            cargo_search_seconds: crate::db::default_cargo_search_seconds(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EventsConfig {
    #[serde(default = "crate::db::default_install_log")]
    pub install_log: String,
    #[serde(default = "crate::db::default_install_finished")]
    pub install_finished: String,
    #[serde(default = "crate::db::default_plugin_install_log")]
    pub plugin_install_log: String,
    #[serde(default = "crate::db::default_plugin_install_finished")]
    pub plugin_install_finished: String,
    #[serde(default = "crate::db::default_update_log")]
    pub update_log: String,
    #[serde(default = "crate::db::default_update_finished")]
    pub update_finished: String,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            install_log: crate::db::default_install_log(),
            install_finished: crate::db::default_install_finished(),
            plugin_install_log: crate::db::default_plugin_install_log(),
            plugin_install_finished: crate::db::default_plugin_install_finished(),
            update_log: crate::db::default_update_log(),
            update_finished: crate::db::default_update_finished(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PluginsConfig {
    #[serde(default)]
    pub official: OfficialPluginsConfig,
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            official: OfficialPluginsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OfficialPluginsConfig {
    #[serde(default = "crate::db::default_plugin_names")]
    pub names: Vec<String>,
}

impl Default for OfficialPluginsConfig {
    fn default() -> Self {
        Self {
            names: crate::db::default_plugin_names(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ParsingConfig {
    #[serde(default = "crate::db::default_default_marker")]
    pub default_marker: String,
    #[serde(default = "crate::db::default_active_marker")]
    pub active_marker: String,
    #[serde(default = "crate::db::default_installed_marker")]
    pub installed_marker: String,
    #[serde(default = "crate::db::default_no_overrides")]
    pub no_overrides: String,
    #[serde(default = "crate::db::default_up_to_date")]
    pub up_to_date: String,
    #[serde(default = "crate::db::default_update_available")]
    pub update_available: String,
    #[serde(default = "crate::db::default_version_separator")]
    pub version_separator: String,
    #[serde(default = "crate::db::default_status_separator")]
    pub status_separator: String,
    #[serde(default = "crate::db::default_cargo_prefix")]
    pub cargo_prefix: String,
}

impl Default for ParsingConfig {
    fn default() -> Self {
        Self {
            default_marker: crate::db::default_default_marker(),
            active_marker: crate::db::default_active_marker(),
            installed_marker: crate::db::default_installed_marker(),
            no_overrides: crate::db::default_no_overrides(),
            up_to_date: crate::db::default_up_to_date(),
            update_available: crate::db::default_update_available(),
            version_separator: crate::db::default_version_separator(),
            status_separator: crate::db::default_status_separator(),
            cargo_prefix: crate::db::default_cargo_prefix(),
        }
    }
}

/// Metadata for a single environment variable entry in config.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EnvVarEntryConfig {
    /// Recommended value
    pub rec: Option<String>,
    /// Default value
    pub def: Option<String>,
    /// Short description of what the variable does
    pub description: String,
    /// Important notes / warnings
    pub notes: String,
}

/// Environment variables config, keyed by category.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EnvVarsConfig {
    #[serde(default, rename = "paths_cache")]
    pub paths_cache: HashMap<String, EnvVarEntryConfig>,
    #[serde(default, rename = "network_proxy")]
    pub network_proxy: HashMap<String, EnvVarEntryConfig>,
    #[serde(default, rename = "build_perf")]
    pub build_perf: HashMap<String, EnvVarEntryConfig>,
    #[serde(default, rename = "debug_diag")]
    pub debug_diag: HashMap<String, EnvVarEntryConfig>,
    #[serde(default, rename = "misc")]
    pub misc: HashMap<String, EnvVarEntryConfig>,
}

impl Default for EnvVarsConfig {
    fn default() -> Self {
        crate::db::default_env_vars().into()
    }
}

impl From<HashMap<String, HashMap<String, EnvVarEntryConfig>>> for EnvVarsConfig {
    fn from(map: HashMap<String, HashMap<String, EnvVarEntryConfig>>) -> Self {
        Self {
            paths_cache: map.get("paths_cache").cloned().unwrap_or_default(),
            network_proxy: map.get("network_proxy").cloned().unwrap_or_default(),
            build_perf: map.get("build_perf").cloned().unwrap_or_default(),
            debug_diag: map.get("debug_diag").cloned().unwrap_or_default(),
            misc: map.get("misc").cloned().unwrap_or_default(),
        }
    }
}

/// Reconstruct the full `AppConfig` from the redb database.
///
/// Used by the `get_config` Tauri command to expose config to the frontend.
pub fn build_app_config_from_db(db: &redb::Database) -> AppConfig {
    let (rustup, cargo) = crate::db::get_binaries_config(db);
    let events = crate::db::get_events_config(db);
    let parsing = crate::db::get_parsing_config(db);
    let app_meta = crate::db::get_app_metadata(db);

    AppConfig {
        app: AppMetadataConfig {
            name: app_meta.0,
            version: app_meta.1,
            description: app_meta.2,
        },
        binaries: BinariesConfig { rustup, cargo },
        paths: PathsConfig {
            cargo_bin_relative: crate::db::get_simple(db, "paths.cargo_bin_relative")
                .unwrap_or_else(crate::db::default_cargo_bin_relative),
        },
        locale: LocaleConfig {
            force_locale: crate::db::get_simple(db, "locale.force_locale")
                .unwrap_or_else(crate::db::default_force_locale),
            codes: crate::db::get_simple(db, "locale.codes")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| vec!["en".to_string()]),
            meta: crate::db::get_simple(db, "locale.meta")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        },
        timeouts: TimeoutsConfig {
            cargo_search_seconds: crate::db::get_simple(db, "timeouts.cargo_search_seconds")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(crate::db::default_cargo_search_seconds),
        },
        events: EventsConfig {
            install_log: events.install_log,
            install_finished: events.install_finished,
            plugin_install_log: events.plugin_install_log,
            plugin_install_finished: events.plugin_install_finished,
            update_log: events.update_log,
            update_finished: events.update_finished,
        },
        plugins: PluginsConfig {
            official: OfficialPluginsConfig {
                names: crate::db::get_plugin_names(db),
            },
        },
        parsing: ParsingConfig {
            default_marker: parsing.default_marker,
            active_marker: parsing.active_marker,
            installed_marker: parsing.installed_marker,
            no_overrides: parsing.no_overrides,
            up_to_date: parsing.up_to_date,
            update_available: parsing.update_available,
            version_separator: parsing.version_separator,
            status_separator: parsing.status_separator,
            cargo_prefix: parsing.cargo_prefix,
        },
        env_vars: crate::db::get_env_vars(db).into(),
    }
}

/// Tauri command to expose the current config to the frontend.
#[tauri::command]
pub fn get_config(state: tauri::State<'_, crate::state::AppState>) -> AppConfig {
    build_app_config_from_db(&state.db)
}

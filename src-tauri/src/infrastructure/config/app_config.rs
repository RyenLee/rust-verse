use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::defaults;

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
    #[serde(default = "super::defaults::app_name")]
    pub name: String,
    #[serde(default = "super::defaults::app_version")]
    pub version: String,
    #[serde(default = "super::defaults::app_description")]
    pub description: String,
}

impl Default for AppMetadataConfig {
    fn default() -> Self {
        Self {
            name: defaults::app_name(),
            version: defaults::app_version(),
            description: defaults::app_description(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BinariesConfig {
    #[serde(default = "super::defaults::rustup")]
    pub rustup: String,
    #[serde(default = "super::defaults::cargo")]
    pub cargo: String,
}

impl Default for BinariesConfig {
    fn default() -> Self {
        Self {
            rustup: defaults::rustup(),
            cargo: defaults::cargo(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PathsConfig {
    #[serde(default = "super::defaults::cargo_bin_relative")]
    pub cargo_bin_relative: String,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            cargo_bin_relative: defaults::cargo_bin_relative(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LocaleConfig {
    #[serde(default = "super::defaults::force_locale")]
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
            force_locale: defaults::force_locale(),
            codes: vec!["en".to_string()],
            meta: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TimeoutsConfig {
    #[serde(default = "super::defaults::cargo_search_seconds")]
    pub cargo_search_seconds: u64,
}

impl Default for TimeoutsConfig {
    fn default() -> Self {
        Self {
            cargo_search_seconds: defaults::cargo_search_seconds(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct EventsConfig {
    #[serde(default = "super::defaults::install_log")]
    pub install_log: String,
    #[serde(default = "super::defaults::install_finished")]
    pub install_finished: String,
    #[serde(default = "super::defaults::plugin_install_log")]
    pub plugin_install_log: String,
    #[serde(default = "super::defaults::plugin_install_finished")]
    pub plugin_install_finished: String,
    #[serde(default = "super::defaults::update_log")]
    pub update_log: String,
    #[serde(default = "super::defaults::update_finished")]
    pub update_finished: String,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            install_log: defaults::install_log(),
            install_finished: defaults::install_finished(),
            plugin_install_log: defaults::plugin_install_log(),
            plugin_install_finished: defaults::plugin_install_finished(),
            update_log: defaults::update_log(),
            update_finished: defaults::update_finished(),
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
    #[serde(default = "super::defaults::plugin_names")]
    pub names: Vec<String>,
}

impl Default for OfficialPluginsConfig {
    fn default() -> Self {
        Self {
            names: defaults::plugin_names(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ParsingConfig {
    #[serde(default = "super::defaults::default_marker")]
    pub default_marker: String,
    #[serde(default = "super::defaults::active_marker")]
    pub active_marker: String,
    #[serde(default = "super::defaults::installed_marker")]
    pub installed_marker: String,
    #[serde(default = "super::defaults::no_overrides")]
    pub no_overrides: String,
    #[serde(default = "super::defaults::up_to_date")]
    pub up_to_date: String,
    #[serde(default = "super::defaults::update_available")]
    pub update_available: String,
    #[serde(default = "super::defaults::version_separator")]
    pub version_separator: String,
    #[serde(default = "super::defaults::status_separator")]
    pub status_separator: String,
    #[serde(default = "super::defaults::cargo_prefix")]
    pub cargo_prefix: String,
}

impl Default for ParsingConfig {
    fn default() -> Self {
        Self {
            default_marker: defaults::default_marker(),
            active_marker: defaults::active_marker(),
            installed_marker: defaults::installed_marker(),
            no_overrides: defaults::no_overrides(),
            up_to_date: defaults::up_to_date(),
            update_available: defaults::update_available(),
            version_separator: defaults::version_separator(),
            status_separator: defaults::status_separator(),
            cargo_prefix: defaults::cargo_prefix(),
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
        defaults::env_vars().into()
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

/// Reconstruct the full `AppConfig` from the store.
///
/// Used by the `get_config` Tauri command to expose config to the frontend.
pub fn build_app_config_from_db(repo: &dyn crate::domain::repository::DataStore) -> AppConfig {
    let (rustup, cargo) = crate::infrastructure::db::get_binaries_config(repo);
    let events = crate::infrastructure::db::get_events_config(repo);
    let parsing = crate::infrastructure::db::get_parsing_config(repo);
    let app_meta = crate::infrastructure::db::get_app_metadata(repo);

    AppConfig {
        app: AppMetadataConfig {
            name: app_meta.0,
            version: app_meta.1,
            description: app_meta.2,
        },
        binaries: BinariesConfig { rustup, cargo },
        paths: PathsConfig {
            cargo_bin_relative: repo
                .get_config("paths.cargo_bin_relative")
                .unwrap_or_else(defaults::cargo_bin_relative),
        },
        locale: LocaleConfig {
            force_locale: repo
                .get_config("locale.force_locale")
                .unwrap_or_else(defaults::force_locale),
            codes: repo
                .get_config("locale.codes")
                .and_then(|s: String| serde_json::from_str::<Vec<String>>(&s).ok())
                .unwrap_or_else(|| vec!["en".to_string()]),
            meta: repo
                .get_config("locale.meta")
                .and_then(|s: String| serde_json::from_str::<HashMap<String, LocaleMeta>>(&s).ok())
                .unwrap_or_default(),
        },
        timeouts: TimeoutsConfig {
            cargo_search_seconds: repo
                .get_config("timeouts.cargo_search_seconds")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(defaults::cargo_search_seconds),
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
                names: repo.get_plugin_names(),
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
        env_vars: repo.get_env_var_metas().into(),
    }
}

/// Tauri command to expose the current config to the frontend.
#[tauri::command]
pub fn get_config(state: tauri::State<'_, crate::state::AppState>) -> AppConfig {
    build_app_config_from_db(&*state.store)
}

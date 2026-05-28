//! Locale scanning and caching business logic.
//!
//! Handles filesystem-based locale discovery, caching with TTL,
//! and fallback logic for when the database has no locale data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use crate::domain::config_keys::keys;
use crate::domain::error::AppResult;
use crate::infrastructure::logger;
use tauri::Manager;

/// Metadata for a discovered locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleInfo {
    pub code: String,
    pub name: String,
    pub english_name: String,
}

/// Well-known locale display names. Used as fallback when metadata.json is absent.
static LOCALE_NAMES: &[(&str, &str, &str)] = &[
    ("en", "English", "English"),
    ("zh-CN", "简体中文", "Chinese Simplified"),
    ("zh-TW", "繁體中文", "Chinese Traditional"),
    ("ja", "日本語", "Japanese"),
    ("ko", "한국어", "Korean"),
    ("fr", "Français", "French"),
    ("de", "Deutsch", "German"),
    ("es", "Español", "Spanish"),
    ("pt-BR", "Português (Brasil)", "Portuguese (Brazil)"),
    ("ru", "Русский", "Russian"),
    ("it", "Italiano", "Italian"),
    ("ar", "العربية", "Arabic"),
];

#[derive(Debug, Deserialize)]
struct LocaleMetadata {
    name: String,
    english_name: String,
}

/// Validate a folder name as a BCP 47 compatible locale code.
pub fn is_valid_locale_code(code: &str) -> bool {
    let parts: Vec<&str> = code.split('-').collect();
    if parts.is_empty() || parts.len() > 2 {
        return false;
    }
    let lang = parts[0];
    if lang.len() < 2 || lang.len() > 3 || !lang.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    if let Some(region) = parts.get(1) {
        if region.len() < 2 || region.len() > 4 || !region.chars().all(|c| c.is_ascii_uppercase()) {
            return false;
        }
    }
    true
}

/// Cached scan result to avoid repeated filesystem operations.
pub struct ScanCache {
    pub locales: Vec<LocaleInfo>,
    pub scanned_at: Instant,
}

/// State holding the scan cache, protected by a Mutex.
pub struct LocaleScanState {
    pub cache: Mutex<Option<ScanCache>>,
}

impl LocaleScanState {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(None),
        }
    }
}

/// Cache duration in seconds (5 minutes).
const CACHE_TTL_SECS: u64 = 300;

/// Find the locales directory in the frontend source.
pub fn find_locales_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let dev_path = cwd.join("src").join("locales");
        if dev_path.is_dir() {
            return Some(dev_path);
        }
        if let Some(parent) = cwd.parent() {
            let dev_path = parent.join("src").join("locales");
            if dev_path.is_dir() {
                return Some(dev_path);
            }
            if let Some(grandparent) = parent.parent() {
                let dev_path = grandparent.join("src").join("locales");
                if dev_path.is_dir() {
                    return Some(dev_path);
                }
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let dev_path = exe_dir.join("src").join("locales");
            if dev_path.is_dir() {
                return Some(dev_path);
            }
            if let Some(parent) = exe_dir.parent() {
                let dev_path = parent.join("src").join("locales");
                if dev_path.is_dir() {
                    return Some(dev_path);
                }
            }
            let resource_path = exe_dir.join("resources").join("locales");
            if resource_path.is_dir() {
                return Some(resource_path);
            }
        }
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        let res_path = resource_dir.join("locales");
        if res_path.is_dir() {
            return Some(res_path);
        }
    }
    None
}

/// Scan the locales directory and return a list of available locales.
pub fn scan_locales_dir(locales_dir: &PathBuf) -> Vec<LocaleInfo> {
    let mut locales = Vec::new();
    let entries = match fs::read_dir(locales_dir) {
        Ok(entries) => entries,
        Err(e) => {
            logger::logger().warn("locale", &format!("failed to read locales directory: {e}"));
            return locales;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if !is_valid_locale_code(&folder_name) {
            continue;
        }
        let index_file = path.join("index.ts");
        if !index_file.exists() {
            continue;
        }
        let (name, english_name) = match read_locale_metadata(&path) {
            Some(meta) => (meta.name, meta.english_name),
            None => LOCALE_NAMES
                .iter()
                .find(|(code, _, _)| *code == folder_name)
                .map(|(_, n, en)| (n.to_string(), en.to_string()))
                .unwrap_or_else(|| (folder_name.clone(), folder_name.clone())),
        };
        locales.push(LocaleInfo {
            code: folder_name,
            name,
            english_name,
        });
    }
    locales.sort_by(|a, b| a.code.cmp(&b.code));
    locales
}

fn read_locale_metadata(locale_dir: &PathBuf) -> Option<LocaleMetadata> {
    let meta_path = locale_dir.join("metadata.json");
    let content = fs::read_to_string(&meta_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Get the list of available locales from the database, falling back to filesystem scan.
pub fn get_locales_from_config_or_db(
    app: &tauri::AppHandle,
    scan_state: &LocaleScanState,
    force_refresh: bool,
) -> Vec<LocaleInfo> {
    if !force_refresh {
        let cache = scan_state.cache.lock().unwrap();
        if let Some(ref cached) = *cache {
            if cached.scanned_at.elapsed().as_secs() < CACHE_TTL_SECS {
                return cached.locales.clone();
            }
        }
    }
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        let store = &*state.store;
        let batch = store.get_config_batch(&[keys::LOCALE_CODES, keys::LOCALE_META]);
        let codes: Vec<String> = batch
            .get(keys::LOCALE_CODES)
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();
        let meta: std::collections::HashMap<String, crate::infrastructure::config::LocaleMeta> =
            batch.get(keys::LOCALE_META)
                .and_then(|s| serde_json::from_str::<HashMap<String, crate::infrastructure::config::LocaleMeta>>(s).ok())
                .unwrap_or_default();
        let locales: Vec<LocaleInfo> = codes
            .into_iter()
            .map(|code| {
                let meta_info = meta.get(&code);
                LocaleInfo {
                    code: code.clone(),
                    name: meta_info
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| code.clone()),
                    english_name: meta_info
                        .map(|m| m.english_name.clone())
                        .unwrap_or_else(|| code.clone()),
                }
            })
            .collect();
        let mut cache = scan_state.cache.lock().unwrap();
        *cache = Some(ScanCache {
            locales: locales.clone(),
            scanned_at: Instant::now(),
        });
        return locales;
    }

    if force_refresh {
        if let Some(dir) = find_locales_dir(app) {
            let locales = scan_locales_dir(&dir);
            if !locales.is_empty() {
                let mut cache = scan_state.cache.lock().unwrap();
                *cache = Some(ScanCache {
                    locales: locales.clone(),
                    scanned_at: Instant::now(),
                });
                return locales;
            }
        }
    }

    vec![LocaleInfo {
        code: "en".to_string(),
        name: "English".to_string(),
        english_name: "English".to_string(),
    }]
}

/// Get locale config path in the app data directory.
pub fn locale_config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("locale.json"))
}

/// Locale configuration persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleConfigFile {
    pub current_locale: String,
    pub last_modified: String,
}

impl Default for LocaleConfigFile {
    fn default() -> Self {
        Self {
            current_locale: "en".to_string(),
            last_modified: String::new(),
        }
    }
}

/// Load locale config from disk, or return default if not found.
pub fn load_locale_config(app: &tauri::AppHandle) -> LocaleConfigFile {
    let path = match locale_config_path(app) {
        Some(p) => p,
        None => return LocaleConfigFile::default(),
    };
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(config) = serde_json::from_str(&content) {
            return config;
        }
    }
    LocaleConfigFile::default()
}

/// Save locale config to disk.
pub fn save_locale_config(app: &tauri::AppHandle, config: &LocaleConfigFile) -> AppResult<()> {
    let path = locale_config_path(app).ok_or_else(|| {
        crate::domain::error::AppError::Config("Cannot determine app data directory".to_string())
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            crate::domain::error::AppError::Config(format!("Failed to create app data dir: {e}"))
        })?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| {
        crate::domain::error::AppError::Config(format!("Failed to serialize locale config: {e}"))
    })?;
    fs::write(&path, content).map_err(|e| {
        crate::domain::error::AppError::Config(format!("Failed to write locale config: {e}"))
    })?;
    Ok(())
}

/// Get current timestamp as RFC 3339-like string.
pub fn chrono_now_rfc3339() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s", duration.as_secs())
}

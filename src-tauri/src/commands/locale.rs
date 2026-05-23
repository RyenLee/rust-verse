use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::error::{AppError, AppResult};

/// Metadata for a discovered locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleInfo {
    /// Locale code (e.g. "en", "zh-CN")
    pub code: String,
    /// Native display name (e.g. "English", "简体中文")
    pub name: String,
    /// English display name (e.g. "English", "Chinese Simplified")
    pub english_name: String,
}

/// Cached scan result to avoid repeated filesystem operations.
struct ScanCache {
    locales: Vec<LocaleInfo>,
    scanned_at: Instant,
}

/// State holding the scan cache, protected by a Mutex.
pub struct LocaleScanState {
    cache: Mutex<Option<ScanCache>>,
}

impl LocaleScanState {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(None),
        }
    }
}

/// Well-known locale display names. Used as fallback when metadata.json is absent.
/// New locales can either add an entry here or provide a metadata.json in their folder.
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

/// metadata.json structure that can be placed in each locale folder.
#[derive(Debug, Deserialize)]
struct LocaleMetadata {
    /// Native display name
    name: String,
    /// English display name
    english_name: String,
}

/// Validate a folder name as a BCP 47 compatible locale code.
fn is_valid_locale_code(code: &str) -> bool {
    // Quick check without regex dependency
    let parts: Vec<&str> = code.split('-').collect();
    if parts.is_empty() || parts.len() > 2 {
        return false;
    }
    // Language part: 2-3 lowercase letters
    let lang = parts[0];
    if lang.len() < 2 || lang.len() > 3 || !lang.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    // Region part (optional): 2-4 uppercase letters
    if let Some(region) = parts.get(1) {
        if region.len() < 2 || region.len() > 4 || !region.chars().all(|c| c.is_ascii_uppercase()) {
            return false;
        }
    }
    true
}

/// Find the locales directory in the frontend source.
fn find_locales_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    // In development: scan src/locales relative to the project root
    // In production: scan the bundled resources
    // Try multiple strategies:

    // Strategy 1: Development mode - search upward for src/locales
    // CWD in dev mode is typically src-tauri/, so we need to go up
    if let Ok(cwd) = std::env::current_dir() {
        // Check CWD itself
        let dev_path = cwd.join("src").join("locales");
        if dev_path.is_dir() {
            return Some(dev_path);
        }
        // Check parent (CWD is src-tauri/, parent is project root)
        if let Some(parent) = cwd.parent() {
            let dev_path = parent.join("src").join("locales");
            if dev_path.is_dir() {
                return Some(dev_path);
            }
            // Check grandparent (in case of nested workspace)
            if let Some(grandparent) = parent.parent() {
                let dev_path = grandparent.join("src").join("locales");
                if dev_path.is_dir() {
                    return Some(dev_path);
                }
            }
        }
    }

    // Strategy 2: Relative to the executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // Try src/locales relative to exe (dev mode from project root)
            let dev_path = exe_dir.join("src").join("locales");
            if dev_path.is_dir() {
                return Some(dev_path);
            }
            // Try parent of exe dir
            if let Some(parent) = exe_dir.parent() {
                let dev_path = parent.join("src").join("locales");
                if dev_path.is_dir() {
                    return Some(dev_path);
                }
            }
            // Try bundled resources path
            let resource_path = exe_dir.join("resources").join("locales");
            if resource_path.is_dir() {
                return Some(resource_path);
            }
        }
    }

    // Strategy 3: Use Tauri's resource directory
    if let Ok(resource_dir) = app.path().resource_dir() {
        let res_path = resource_dir.join("locales");
        if res_path.is_dir() {
            return Some(res_path);
        }
    }

    None
}

/// Scan the locales directory and return a list of available locales.
fn scan_locales_dir(locales_dir: &PathBuf) -> Vec<LocaleInfo> {
    let mut locales = Vec::new();

    let entries = match fs::read_dir(locales_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Warning: failed to read locales directory: {e}");
            return locales;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Only process directories
        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Validate locale code format
        if !is_valid_locale_code(&folder_name) {
            eprintln!(
                "Warning: skipping locale folder '{}' - name does not match BCP 47 format (e.g. 'en', 'zh-CN')",
                folder_name
            );
            continue;
        }

        // Check for index.ts (required entry point)
        let index_file = path.join("index.ts");
        if !index_file.exists() {
            eprintln!(
                "Warning: skipping locale folder '{}' - missing index.ts entry file",
                folder_name
            );
            continue;
        }

        // Try to read metadata.json for display names
        let (name, english_name) = match read_locale_metadata(&path) {
            Some(meta) => (meta.name, meta.english_name),
            None => {
                // Fallback to well-known names
                let (n, en) = LOCALE_NAMES
                    .iter()
                    .find(|(code, _, _)| *code == folder_name)
                    .map(|(_, n, en)| (n.to_string(), en.to_string()))
                    .unwrap_or_else(|| {
                        // Last resort: use the code itself
                        (folder_name.clone(), folder_name.clone())
                    });
                (n, en)
            }
        };

        locales.push(LocaleInfo {
            code: folder_name,
            name,
            english_name,
        });
    }

    // Sort by code for consistent ordering
    locales.sort_by(|a, b| a.code.cmp(&b.code));

    locales
}

/// Read metadata.json from a locale folder if it exists.
fn read_locale_metadata(locale_dir: &PathBuf) -> Option<LocaleMetadata> {
    let meta_path = locale_dir.join("metadata.json");
    let content = fs::read_to_string(&meta_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Cache duration in seconds (5 minutes).
const CACHE_TTL_SECS: u64 = 300;

/// Get the list of available locales from the database (populated from config.toml at build time).
///
/// Falls back to filesystem scanning only when:
/// - force_refresh is true AND
/// - the database has no locale data (fresh install without migration)
///
/// This ensures locale info is always available without runtime filesystem access.
fn get_locales_from_config_or_db(
    app: &tauri::AppHandle,
    scan_state: &LocaleScanState,
    force_refresh: bool,
) -> Vec<LocaleInfo> {
    // Check cache first
    if !force_refresh {
        let cache = scan_state.cache.lock().unwrap();
        if let Some(ref cached) = *cache {
            if cached.scanned_at.elapsed().as_secs() < CACHE_TTL_SECS {
                return cached.locales.clone();
            }
        }
    }

    // Try to get locale info from the database (populated from config.toml at build time)
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        let db = &state.db;
        let codes: Vec<String> = crate::db::get_simple(db, "locale.codes")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let meta: std::collections::HashMap<String, crate::config::LocaleMeta> =
            crate::db::get_simple(db, "locale.meta")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

        if !codes.is_empty() {
            let locales: Vec<LocaleInfo> = codes
                .into_iter()
                .map(|code| {
                    let meta_info = meta.get(&code);
                    LocaleInfo {
                        code: code.clone(),
                        name: meta_info.map(|m| m.name.clone()).unwrap_or_else(|| code.clone()),
                        english_name: meta_info
                            .map(|m| m.english_name.clone())
                            .unwrap_or_else(|| code.clone()),
                    }
                })
                .collect();

            // Update cache
            let mut cache = scan_state.cache.lock().unwrap();
            *cache = Some(ScanCache {
                locales: locales.clone(),
                scanned_at: Instant::now(),
            });

            return locales;
        }
    }

    // Fallback: filesystem scan (only in dev mode with force_refresh, or if migration hasn't happened)
    if force_refresh {
        let locales_dir = find_locales_dir(app);
        if let Some(dir) = locales_dir {
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

    // Ultimate fallback: well-known default
    vec![LocaleInfo {
        code: "en".to_string(),
        name: "English".to_string(),
        english_name: "English".to_string(),
    }]
}

/// Get the list of available locales, using cache when available.
fn get_available_locales_inner(
    app: &tauri::AppHandle,
    scan_state: &LocaleScanState,
    force_refresh: bool,
) -> Vec<LocaleInfo> {
    get_locales_from_config_or_db(app, scan_state, force_refresh)
}

/// Tauri command: Get the list of available locales.
/// Results are cached for 5 minutes unless force_refresh is true.
#[tauri::command]
pub fn list_available_locales(
    app: tauri::AppHandle,
    state: tauri::State<'_, LocaleScanState>,
    force_refresh: Option<bool>,
) -> Vec<LocaleInfo> {
    get_available_locales_inner(&app, &state, force_refresh.unwrap_or(false))
}

/// Tauri command: Validate a locale code against discovered locales.
#[tauri::command]
pub fn validate_locale(
    app: tauri::AppHandle,
    state: tauri::State<'_, LocaleScanState>,
    locale: String,
) -> bool {
    let locales = get_available_locales_inner(&app, &state, false);
    locales.iter().any(|l| l.code == locale)
}

/// Locale configuration persisted to $APPDATA/locale.json
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

/// Get the path to the locale config file in the app data directory.
fn locale_config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("locale.json"))
}

/// Load locale config from disk, or return default if not found.
fn load_locale_config(app: &tauri::AppHandle) -> LocaleConfigFile {
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
fn save_locale_config(app: &tauri::AppHandle, config: &LocaleConfigFile) -> AppResult<()> {
    let path = locale_config_path(app)
        .ok_or_else(|| AppError::Config("Cannot determine app data directory".to_string()))?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::Config(format!("Failed to create app data dir: {e}")))?;
    }

    let content = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::Config(format!("Failed to serialize locale config: {e}")))?;

    fs::write(&path, content)
        .map_err(|e| AppError::Config(format!("Failed to write locale config: {e}")))?;

    Ok(())
}

/// Get the current persisted locale.
#[tauri::command]
pub fn get_locale(app: tauri::AppHandle) -> String {
    let config = load_locale_config(&app);
    config.current_locale
}

/// Set and persist the locale preference.
/// Validates the locale against discovered locales from the filesystem.
#[tauri::command]
pub fn set_locale(
    app: tauri::AppHandle,
    state: tauri::State<'_, LocaleScanState>,
    locale: String,
) -> AppResult<()> {
    // Validate locale code against dynamically discovered locales
    if !validate_locale(app.clone(), state, locale.clone()) {
        return Err(AppError::Config(format!(
            "Unsupported locale: {locale}. Ensure the locale folder exists in src/locales/ with a valid index.ts file."
        )));
    }

    let config = LocaleConfigFile {
        current_locale: locale,
        last_modified: chrono_now_rfc3339(),
    };

    save_locale_config(&app, &config)
}

/// Get current timestamp as RFC 3339 string (no external chrono dependency needed).
fn chrono_now_rfc3339() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_locale_codes() {
        assert!(is_valid_locale_code("en"));
        assert!(is_valid_locale_code("zh-CN"));
        assert!(is_valid_locale_code("pt-BR"));
        assert!(is_valid_locale_code("fr-CA"));
        assert!(is_valid_locale_code("ja"));
        assert!(is_valid_locale_code("ko"));
    }

    #[test]
    fn test_invalid_locale_codes() {
        assert!(!is_valid_locale_code(""));
        assert!(!is_valid_locale_code("EN"));
        assert!(!is_valid_locale_code("zh-cn"));
        assert!(!is_valid_locale_code("a"));
        assert!(!is_valid_locale_code("abcd"));
        assert!(!is_valid_locale_code("zh-CN-extra"));
        assert!(!is_valid_locale_code("12-34"));
        assert!(!is_valid_locale_code(".hidden"));
        assert!(!is_valid_locale_code("node_modules"));
    }
}

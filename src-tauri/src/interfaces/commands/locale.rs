//! Locale management commands — thin forwarding layer.
//!
//! These commands delegate all business logic to `application::locale`.

use crate::application::locale as locale_svc;
use crate::domain::error::AppResult;

// Re-export for backward compatibility
#[allow(unused_imports)]
pub use crate::application::locale::{LocaleInfo, LocaleScanState, is_valid_locale_code};

/// Tauri command: Get the list of available locales.
/// Results are cached for 5 minutes unless force_refresh is true.
#[tauri::command]
pub fn list_available_locales(
    app: tauri::AppHandle,
    state: tauri::State<'_, LocaleScanState>,
    force_refresh: Option<bool>,
) -> Vec<LocaleInfo> {
    locale_svc::get_locales_from_config_or_db(&app, &state, force_refresh.unwrap_or(false))
}

/// Tauri command: Validate a locale code against discovered locales.
#[tauri::command]
pub fn validate_locale(
    app: tauri::AppHandle,
    state: tauri::State<'_, LocaleScanState>,
    locale: String,
) -> bool {
    let locales = locale_svc::get_locales_from_config_or_db(&app, &state, false);
    locales.iter().any(|l| l.code == locale)
}

/// Get the current persisted locale.
#[tauri::command]
pub fn get_locale(app: tauri::AppHandle) -> String {
    let config = locale_svc::load_locale_config(&app);
    config.current_locale
}

/// Set and persist the locale preference.
/// Also updates `AppState.locale` so that `notifier::notify()` renders
/// notification messages in the correct language.
#[tauri::command]
pub fn set_locale(
    app: tauri::AppHandle,
    scan_state: tauri::State<'_, LocaleScanState>,
    app_state: tauri::State<'_, crate::state::AppState>,
    locale: String,
) -> AppResult<()> {
    if !validate_locale(app.clone(), scan_state, locale.clone()) {
        return Err(crate::domain::error::AppError::Config(format!(
            "Unsupported locale: {locale}. Ensure the locale folder exists in src/locales/ with a valid index.ts file."
        )));
    }

    // Sync runtime locale so notifier renders messages in the right language
    if let Ok(mut current) = app_state.locale.lock() {
        *current = locale.clone();
    }

    let config = locale_svc::LocaleConfigFile {
        current_locale: locale,
        last_modified: locale_svc::chrono_now_rfc3339(),
    };

    locale_svc::save_locale_config(&app, &config)
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

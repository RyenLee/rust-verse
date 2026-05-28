use crate::domain::base::time::chrono_now_ms;
use crate::domain::settings::UserSettings;
use crate::infrastructure::config::{AppConfig, build_app_config_from_db};
use crate::infrastructure::proxy;
use crate::state::AppState;

/// Expose the current config to the frontend.
#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> AppConfig {
    build_app_config_from_db(&*state.store)
}

/// Get user settings from the store.
#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Result<UserSettings, String> {
    let json = (&*state.store).get_settings().ok_or("Settings not found")?;
    serde_json::from_str(&json).map_err(|e| format!("Failed to parse settings: {e}"))
}

/// Save user settings to the store.
/// Also triggers notification auto-cleanup if the auto_cleanup_minutes setting is active.
#[tauri::command]
pub fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: UserSettings,
) -> Result<(), String> {
    settings.validate()?;
    let json = serde_json::to_string(&settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    (&*state.store)
        .set_settings(&json)
        .map_err(|e| format!("Failed to save settings: {e}"))?;

    // Trigger auto-cleanup for notifications if the setting is active
    let minutes = settings.notifications.auto_cleanup_minutes;
    if minutes > 0 {
        let cutoff_ms = chrono_now_ms() - (minutes as i64) * 60 * 1000;
        // Fire-and-forget — don't block on cleanup
        let _ = (&*state.store).notification_delete_read_before(cutoff_ms);
    }

    // Invalidate the proxy cache so next subprocess execution picks up
    // the updated proxy settings.
    proxy::invalidate_cache();

    Ok(())
}

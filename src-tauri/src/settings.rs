//! User settings persistence layer.
//!
//! **DDD refactored**: `UserSettings` domain type now lives in
//! `domain::settings`.  This module retains DB I/O functions and
//! re-exports the domain type for backward compatibility.

use crate::domain::repository::SettingsRepository;

// Re-export for backward compatibility.
pub use crate::domain::settings::UserSettings;

// ── Database I/O ──

/// Read user settings from the data store.
/// Returns the safe (all-off) defaults if no settings have been saved yet.
pub fn get_settings_inner(repo: &dyn SettingsRepository) -> Result<UserSettings, String> {
    match repo.get_settings() {
        Some(json) => {
            let settings: UserSettings = serde_json::from_str(&json)
                .map_err(|e| format!("Settings data is corrupted – failed to parse: {e}"))?;
            // Guard: if the stored data is somehow invalid, fall back to verified defaults.
            settings.validate()?;
            Ok(settings)
        }
        None => Ok(UserSettings::default()),
    }
}

/// Write user settings to the data store as an atomic JSON blob.
/// Validation runs *before* the write so that bad data never reaches disk.
pub fn save_settings_inner(repo: &dyn SettingsRepository, settings: &UserSettings) -> Result<(), String> {
    settings.validate()?;
    let json = serde_json::to_string(settings)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    repo.set_settings(&json)
        .map_err(|e| format!("Database write failed – your data has not been saved: {e}"))?;

    // Verify the write was durable by re-reading immediately.
    match repo.get_settings() {
        Some(stored) if stored == json => Ok(()),
        Some(_) => Err("Data verification failed – the stored value does not match what was written. Please try again.".to_string()),
        None => Err("Data verification failed – the settings could not be read back after writing.".to_string()),
    }
}
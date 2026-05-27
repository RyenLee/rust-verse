//! Environment variable persistence commands — thin forwarding layer.

use tauri::AppHandle;

use crate::application::persist as persist_svc;
use crate::domain::error::AppResult;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::infrastructure::notifier;

/// Persist an environment variable to the system.
#[tauri::command]
pub async fn persist_env_var(app: AppHandle, name: String, value: String) -> AppResult<()> {
    persist_svc::persist_env_var(&name, &value)?;

    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::EnvVarPersisted,
        &[("name", &name), ("value", &value)],
        Some("/env-vars"),
    );

    Ok(())
}

/// Remove a persisted environment variable from the system.
#[tauri::command]
pub async fn remove_persisted_env_var(app: AppHandle, name: String) -> AppResult<()> {
    persist_svc::remove_persisted_env_var(&name)?;

    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::PersistVarRemoved,
        &[("name", &name)],
        Some("/env-vars"),
    );

    Ok(())
}

/// Check if an environment variable is persisted at system level.
#[tauri::command]
pub async fn is_env_var_persisted(name: String) -> AppResult<bool> {
    persist_svc::is_env_var_persisted(&name)
}

/// List all persisted Rust environment variables.
#[tauri::command]
pub async fn list_persisted_env_vars() -> AppResult<Vec<String>> {
    persist_svc::list_persisted_env_vars()
}

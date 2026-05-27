//! Environment variable commands — thin forwarding layer.
//!
//! **Design principle**: commands only:
//!   a) Deserialize input parameters
//!   b) Call appropriate services from `services/`
//!   c) Handle error conditions
//!   d) Return formatted results
//!
//! No business logic lives here.

use std::env;

use tauri::{AppHandle, State};

use crate::application::env_var as env_var_svc;
use crate::domain::error::{AppError, AppResult};
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export types for backward compatibility with lib.rs imports
#[allow(unused_imports)]
pub use crate::domain::entity::{EnvVarEntry, EnvVarInfo, EnvVarMeta};

/// List all known Rust environment variables with their current values.
#[tauri::command]
pub fn list_env_vars(state: State<'_, AppState>) -> Vec<EnvVarInfo> {
    let metas = env_var_svc::build_env_var_metas_from_db(&*state.store);

    metas
        .into_iter()
        .map(|meta| {
            let value = env::var(&meta.name).unwrap_or_default();
            let is_set = env::var(&meta.name).is_ok();
            EnvVarInfo {
                meta,
                value,
                is_set,
            }
        })
        .collect()
}

/// Get the current value of a specific environment variable.
#[tauri::command]
pub fn get_env_var(name: String) -> EnvVarEntry {
    let value = env::var(&name).unwrap_or_default();
    let is_set = env::var(&name).is_ok();
    EnvVarEntry {
        name,
        value,
        is_set,
    }
}

/// Set an environment variable for the current process.
///
/// Note: this only affects the current application process and its children.
/// It does NOT persist across application restarts or set system-level env vars.
///
/// Special variables like `RUST_LOG` will trigger additional actions (e.g. log level change).
#[tauri::command]
pub fn set_env_var(app: AppHandle, name: String, value: String) -> AppResult<EnvVarEntry> {
    if name.is_empty() {
        return Err(AppError::Config(
            "Variable name cannot be empty".to_string(),
        ));
    }
    if name.contains('=') || name.contains('\0') {
        return Err(AppError::Config(
            "Variable name contains invalid characters".to_string(),
        ));
    }

    unsafe {
        env::set_var(&name, &value);
    }

    env_var_svc::handle_special_env_var_set(&name, &value);

    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::EnvVarSet,
        &[("name", &name), ("value", &value)],
        Some("/env-vars"),
    );

    Ok(EnvVarEntry {
        name,
        value,
        is_set: true,
    })
}

/// Remove (unset) an environment variable from the current process.
///
/// Special variables like `RUST_LOG` will trigger additional actions (e.g. reset log level to ERROR).
#[tauri::command]
pub fn remove_env_var(app: AppHandle, name: String) -> AppResult<EnvVarEntry> {
    if name.is_empty() {
        return Err(AppError::Config(
            "Variable name cannot be empty".to_string(),
        ));
    }

    unsafe {
        env::remove_var(&name);
    }

    env_var_svc::handle_special_env_var_remove(&name);

    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::EnvVarRemoved,
        &[("name", &name)],
        Some("/env-vars"),
    );

    Ok(EnvVarEntry {
        name,
        value: String::new(),
        is_set: false,
    })
}

/// Update environment variable metadata in the database.
///
/// If `old_category` is provided and differs from `category`, the old entry
/// is deleted first (handles category change / rename).
#[tauri::command]
pub fn update_env_var_meta(
    state: State<'_, AppState>,
    category: String,
    name: String,
    description: String,
    rec: Option<String>,
    def: Option<String>,
    notes: String,
    old_category: Option<String>,
    old_name: Option<String>,
) -> AppResult<()> {
    if name.is_empty() {
        return Err(AppError::Config(
            "Variable name cannot be empty".to_string(),
        ));
    }

    let entry = crate::infrastructure::config::EnvVarEntryConfig {
        rec,
        def,
        description,
        notes,
    };

    // If category or name changed, delete the old entry first
    if let Some(old_cat) = old_category {
        let old_n = old_name.as_deref().unwrap_or(&name);
        if old_cat != category || old_n != name {
            let _ = (&*state.store).delete_env_var_meta(&old_cat, old_n);
        }
    }

    (&*state.store)
        .set_env_var_meta(&category, &name, &entry)
        .map_err(|e| AppError::Config(format!("failed to update env var meta: {e}")))?;

    Ok(())
}

/// Delete an environment variable metadata entry from the database.
#[tauri::command]
pub fn delete_env_var_meta(
    state: State<'_, AppState>,
    category: String,
    name: String,
) -> AppResult<()> {
    (&*state.store)
        .delete_env_var_meta(&category, &name)
        .map_err(|e| AppError::Config(format!("failed to delete env var meta: {e}")))?;
    Ok(())
}

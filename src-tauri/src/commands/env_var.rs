use serde::Serialize;
use std::env;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use tauri::State;

/// A single environment variable entry.
#[derive(Debug, Clone, Serialize)]
pub struct EnvVarEntry {
    /// Variable name, e.g. "RUSTUP_HOME"
    name: String,
    /// Current value (empty string if not set)
    value: String,
    /// Whether the variable is currently set
    is_set: bool,
}

/// Metadata for a known Rust environment variable.
#[derive(Debug, Clone, Serialize)]
pub struct EnvVarMeta {
    /// Variable name
    name: String,
    /// Category key (e.g. "paths_cache", "network_proxy", etc.)
    category: String,
    /// Short description of what the variable does
    description: String,
    /// Recommended value
    rec: Option<String>,
    /// Default value
    def: Option<String>,
    /// Important notes / warnings
    notes: String,
}

/// Full info for a Rust env var: metadata + current value.
#[derive(Debug, Clone, Serialize)]
pub struct EnvVarInfo {
    #[serde(flatten)]
    meta: EnvVarMeta,
    value: String,
    is_set: bool,
}

/// Build EnvVarMeta list from the redb database.
///
/// Returns entries in a deterministic order: categories follow the
/// canonical order (rustup → cargo → rustc), and variables within each
/// category are sorted alphabetically by name.
fn build_env_var_metas_from_db(db: &redb::Database) -> Vec<EnvVarMeta> {
    let env_vars = crate::db::get_env_vars(db);
    let mut metas = Vec::new();

    // Fixed category order for stable output
    let category_order: [&str; 5] = [
        "paths_cache",
        "network_proxy",
        "build_perf",
        "debug_diag",
        "misc",
    ];

    // First, emit categories in the fixed order
    for cat in &category_order {
        if let Some(vars) = env_vars.get(*cat) {
            let mut sorted: Vec<_> = vars.iter().collect();
            sorted.sort_by_key(|(name, _)| *name);
            for (name, entry) in sorted {
                metas.push(EnvVarMeta {
                    name: name.clone(),
                    category: cat.to_string(),
                    description: entry.description.clone(),
                    rec: entry.rec.clone(),
                    def: entry.def.clone(),
                    notes: entry.notes.clone(),
                });
            }
        }
    }

    // Then, emit any extra categories not in the canonical list (sorted)
    let mut extra_cats: Vec<_> = env_vars
        .keys()
        .filter(|c| !category_order.contains(&c.as_str()))
        .collect();
    extra_cats.sort();
    for cat in extra_cats {
        if let Some(vars) = env_vars.get(cat.as_str()) {
            let mut sorted: Vec<_> = vars.iter().collect();
            sorted.sort_by_key(|(name, _)| *name);
            for (name, entry) in sorted {
                metas.push(EnvVarMeta {
                    name: name.clone(),
                    category: cat.clone(),
                    description: entry.description.clone(),
                    rec: entry.rec.clone(),
                    def: entry.def.clone(),
                    notes: entry.notes.clone(),
                });
            }
        }
    }

    metas
}

/// List all known Rust environment variables with their current values.
#[tauri::command]
pub fn list_env_vars(state: State<'_, AppState>) -> Vec<EnvVarInfo> {
    let metas = build_env_var_metas_from_db(&state.db);

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
#[tauri::command]
pub fn set_env_var(name: String, value: String) -> AppResult<EnvVarEntry> {
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
    Ok(EnvVarEntry {
        name,
        value,
        is_set: true,
    })
}

/// Remove (unset) an environment variable from the current process.
#[tauri::command]
pub fn remove_env_var(name: String) -> AppResult<EnvVarEntry> {
    if name.is_empty() {
        return Err(AppError::Config(
            "Variable name cannot be empty".to_string(),
        ));
    }

    unsafe {
        env::remove_var(&name);
    }
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

    let entry = crate::config::EnvVarEntryConfig {
        rec,
        def,
        description,
        notes,
    };

    // If category or name changed, delete the old entry first
    if let Some(old_cat) = old_category {
        let old_n = old_name.as_deref().unwrap_or(&name);
        if old_cat != category || old_n != name {
            let _ = crate::db::delete_env_var_entry(&state.db, &old_cat, old_n);
        }
    }

    crate::db::set_env_var_entry(&state.db, &category, &name, &entry)
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
    crate::db::delete_env_var_entry(&state.db, &category, &name)
        .map_err(|e| AppError::Config(format!("failed to delete env var meta: {e}")))?;
    Ok(())
}

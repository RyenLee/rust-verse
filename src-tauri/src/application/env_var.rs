//! Environment variable business logic.
//!
//! Builds metadata lists from the database and handles special env var actions
//! (e.g., RUST_LOG level synchronization).

use crate::domain::entity::EnvVarMeta;
use crate::domain::repository::EnvVarRepository;
use crate::infrastructure::logger;

/// Build EnvVarMeta list from the store.
///
/// Returns entries in a deterministic order: categories follow the
/// canonical order (rustup → cargo → rustc), and variables within each
/// category are sorted alphabetically by name.
pub fn build_env_var_metas_from_db(repo: &dyn EnvVarRepository) -> Vec<EnvVarMeta> {
    let env_vars = repo.get_env_var_metas();
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

/// Handle special environment variable actions when setting.
pub fn handle_special_env_var_set(name: &str, value: &str) {
    match name.to_uppercase().as_str() {
        "RUST_LOG" => {
            if let Some(log_level) = logger::LogLevel::from_str(value) {
                let old_level = logger::get_min_log_level_str();
                logger::set_min_log_level(log_level);
                logger::logger().info(
                    "env",
                    &format!("RUST_LOG changed from {} to {}", old_level, value),
                );
            } else {
                logger::logger().warn(
                    "env",
                    &format!("Invalid RUST_LOG value: '{}', using default", value),
                );
            }
        }
        _ => {}
    }
}

/// Handle special environment variable actions when removing.
pub fn handle_special_env_var_remove(name: &str) {
    match name.to_uppercase().as_str() {
        "RUST_LOG" => {
            let old_level = logger::get_min_log_level_str();
            logger::set_min_log_level(logger::LogLevel::Error);
            logger::logger().info(
                "env",
                &format!(
                    "RUST_LOG removed, log level reset from {} to ERROR",
                    old_level
                ),
            );
        }
        _ => {}
    }
}

//! Cargo plugin management commands — thin forwarding layer.

use tauri::{AppHandle, State};

use crate::domain::config_keys::keys;
use crate::domain::error::AppResult;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::domain::parsing;
use crate::infrastructure::exec;
use crate::infrastructure::logger;
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export for backward compatibility
pub use crate::domain::entity::{CargoPluginInfo, SearchResult};
#[allow(unused_imports)]
pub use crate::domain::parsing::{parse_cargo_plugin_list, parse_search_results};

/// List installed cargo plugins.
#[tauri::command]
pub async fn list_cargo_plugins(
    cargo_path: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<CargoPluginInfo>> {
    logger::logger().info("plugin", "list_cargo_plugins requested");
    crate::infrastructure::system::env::validate_rust_binary(&cargo_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let output = exec::run_command(&cargo_path, &["install", "--list"], 30).await?;
    let db_parsing = state.config_cache.get_parsing(&*state.store);
    let official_names = (&*state.store).get_plugin_names();
    Ok(parsing::parse_cargo_plugin_list(
        &output,
        &db_parsing.cargo_prefix,
        &official_names,
    ))
}

/// Install a cargo plugin with streaming output.
#[tauri::command]
pub async fn install_plugin(
    app: AppHandle,
    state: State<'_, AppState>,
    cargo_path: String,
    crate_name: String,
) -> AppResult<()> {
    logger::logger().log_request(
        "install_plugin",
        &format!("cargo_path={:?}, crate_name={:?}", cargo_path, crate_name),
    );
    crate::infrastructure::system::env::validate_rust_binary(&cargo_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        (
            locale_key,
            events.plugin_install_log,
            events.plugin_install_finished,
        )
    };

    // ── Set running flag & reset cancel flag ──
    {
        let mut running = state.task_state.running.lock().unwrap();
        if *running {
            return Err(crate::domain::error::AppError::Command(
                "Another installation or update task is already in progress.".to_string(),
            ));
        }
        *running = true;
    }
    state
        .task_state
        .cancel_flag
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let cancel_notify = state.task_state.cancel_notify.clone();

    let result = exec::run_command_with_cancel(
        app.clone(),
        &cargo_path,
        &["install", &crate_name],
        &locale_key,
        &log_event,
        &finished_event,
        600,
        cancel_notify,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;

    match result {
        Ok(()) => {
            let display_name = crate_name.clone();
            notifier::notify(
                &app,
                Category::Install,
                Priority::Low,
                NotificationKey::PluginInstalled,
                &[("name", &display_name)],
                Some("/plugins"),
            );
            Ok(())
        }
        Err(e) => {
            notifier::notify(
                &app,
                Category::Operation,
                Priority::Low,
                NotificationKey::PluginInstallFailed,
                &[("name", &crate_name), ("error", &format!("{e}"))],
                Some("/plugins"),
            );
            Err(e)
        }
    }
}

/// Update a cargo plugin to the latest version.
#[tauri::command]
pub async fn update_plugin(
    app: AppHandle,
    state: State<'_, AppState>,
    cargo_path: String,
    crate_name: String,
) -> AppResult<()> {
    logger::logger().log_request(
        "update_plugin",
        &format!("cargo_path={:?}, crate_name={:?}", cargo_path, crate_name),
    );
    crate::infrastructure::system::env::validate_rust_binary(&cargo_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        (
            locale_key,
            events.plugin_install_log,
            events.plugin_install_finished,
        )
    };

    {
        let mut running = state.task_state.running.lock().unwrap();
        if *running {
            return Err(crate::domain::error::AppError::Command(
                "Another installation or update task is already in progress.".to_string(),
            ));
        }
        *running = true;
    }
    state
        .task_state
        .cancel_flag
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let cancel_notify = state.task_state.cancel_notify.clone();

    let result = exec::run_command_with_cancel(
        app.clone(),
        &cargo_path,
        &["install", "--force", &crate_name],
        &locale_key,
        &log_event,
        &finished_event,
        600,
        cancel_notify,
    )
    .await;

    *state.task_state.running.lock().unwrap() = false;

    match result {
        Ok(()) => {
            let display_name = crate_name.clone();
            notifier::notify(
                &app,
                Category::Update,
                Priority::Low,
                NotificationKey::PluginUpdated,
                &[("name", &display_name)],
                Some("/plugins"),
            );
            Ok(())
        }
        Err(e) => {
            notifier::notify(
                &app,
                Category::Operation,
                Priority::Low,
                NotificationKey::PluginUpdateFailed,
                &[("name", &crate_name), ("error", &format!("{e}"))],
                Some("/plugins"),
            );
            Err(e)
        }
    }
}

/// Check for updates for installed plugins.
#[tauri::command]
pub async fn check_plugin_updates(
    cargo_path: String,
    plugins: Vec<(String, String)>,
    state: State<'_, AppState>,
) -> AppResult<Vec<(String, bool)>> {
    logger::logger().log_request(
        "check_plugin_updates",
        &format!("cargo_path={:?}, plugins={:?}", cargo_path, plugins),
    );
    crate::infrastructure::system::env::validate_rust_binary(&cargo_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;

    let timeout = (&*state.store)
        .get_config(keys::TIMEOUT_CARGO_SEARCH)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(crate::infrastructure::config::defaults::cargo_search_seconds);

    let mut results = Vec::new();
    for (crate_name, installed_version) in plugins {
        let output = match exec::run_command_with_timeout(
            &cargo_path,
            &["search", "--registry", "crates-io", &crate_name],
            timeout,
        )
        .await
        {
            Ok(o) => o,
            Err(_) => {
                results.push((crate_name, false));
                continue;
            }
        };

        let search_results = parsing::parse_search_results(&output);
        let has_update = match search_results.into_iter().find(|r| r.name == crate_name) {
            Some(result) => {
                let installed = match semver::Version::parse(&installed_version) {
                    Ok(v) => v,
                    Err(_) => {
                        results.push((crate_name, false));
                        continue;
                    }
                };
                let latest = match semver::Version::parse(&result.version) {
                    Ok(v) => v,
                    Err(_) => {
                        results.push((crate_name, false));
                        continue;
                    }
                };
                latest > installed
            }
            None => false,
        };

        results.push((crate_name, has_update));
    }

    Ok(results)
}

/// Uninstall a cargo plugin.
#[tauri::command]
pub async fn uninstall_plugin(
    app: AppHandle,
    cargo_path: String,
    crate_name: String,
) -> AppResult<()> {
    logger::logger().info(
        "plugin",
        &format!("uninstall_plugin requested: {}", crate_name),
    );
    crate::infrastructure::system::env::validate_rust_binary(&cargo_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    exec::run_command(&cargo_path, &["uninstall", &crate_name], 120).await?;

    // ── Notification: plugin uninstalled ──
    let display_name = crate_name.clone();
    notifier::notify(
        &app,
        Category::Install,
        Priority::Low,
        NotificationKey::PluginUninstalled,
        &[("name", &display_name)],
        Some("/plugins"),
    );

    Ok(())
}

/// Search for cargo plugins using `cargo search`.
#[tauri::command]
pub async fn search_plugins(
    cargo_path: String,
    query: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<SearchResult>> {
    logger::logger().log_request(
        "search_plugins",
        &format!("cargo_path={:?}, query={:?}", cargo_path, query),
    );
    crate::infrastructure::system::env::validate_rust_binary(&cargo_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let timeout = (&*state.store)
        .get_config(keys::TIMEOUT_CARGO_SEARCH)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(crate::infrastructure::config::defaults::cargo_search_seconds);
    let output = exec::run_command_with_timeout(
        &cargo_path,
        &["search", "--registry", "crates-io", &query],
        timeout,
    )
    .await?;
    Ok(parsing::parse_search_results(&output))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config_values() -> (String, Vec<String>) {
        (
            "cargo-".to_string(),
            vec![
                "cargo-clippy".to_string(),
                "cargo-fmt".to_string(),
                "cargo-miri".to_string(),
                "cargo-rustdoc".to_string(),
                "cargo-test-fixture".to_string(),
                "rustfmt".to_string(),
                "clippy".to_string(),
                "miri".to_string(),
            ],
        )
    }

    #[test]
    fn test_parse_cargo_plugin_list() {
        let (prefix, official) = default_config_values();
        let output = "cargo-audit v0.18.3:\n    cargo-audit\ncargo-expand v1.0.0:\n    cargo-expand\nrust-script v0.3.0:\n    rust-script";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "audit");
        assert_eq!(result[0].crate_name, "cargo-audit");
        assert_eq!(result[0].version, "0.18.3");
        assert!(!result[0].is_official);
        assert_eq!(result[1].name, "expand");
        assert_eq!(result[2].name, "rust-script");
        assert_eq!(result[2].crate_name, "rust-script");
    }

    #[test]
    fn test_parse_cargo_plugin_list_non_cargo_prefix() {
        let (prefix, official) = default_config_values();
        let output = "crm v0.2.3:\n    crm.exe";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "crm");
        assert_eq!(result[0].crate_name, "crm");
        assert_eq!(result[0].version, "0.2.3");
    }

    #[test]
    fn test_parse_cargo_plugin_list_empty() {
        let (prefix, official) = default_config_values();
        let result = parse_cargo_plugin_list("", &prefix, &official);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_cargo_plugin_list_no_cargo_prefix() {
        let (prefix, official) = default_config_values();
        let output = "some-tool v1.0.0:\n    some-tool";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "some-tool");
        assert_eq!(result[0].crate_name, "some-tool");
    }

    #[test]
    fn test_parse_cargo_plugin_list_multiple() {
        let (prefix, official) = default_config_values();
        let output = "cargo-audit v0.18.3:\n    cargo-audit\ncargo-clippy v0.1.0:\n    cargo-clippy\ncargo-outdated v0.12.0:\n    cargo-outdated";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "audit");
        assert!(!result[0].is_official);
        assert_eq!(result[1].name, "clippy");
        assert!(result[1].is_official);
        assert_eq!(result[2].name, "outdated");
        assert_eq!(result[2].version, "0.12.0");
    }

    #[test]
    fn test_parse_cargo_plugin_list_no_colon() {
        let (prefix, official) = default_config_values();
        let output = "cargo-audit v0.18.3\n    cargo-audit";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_cargo_plugin_list_no_version() {
        let (prefix, official) = default_config_values();
        let output = "cargo-audit:\n    cargo-audit";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert!(result.is_empty());
    }
}

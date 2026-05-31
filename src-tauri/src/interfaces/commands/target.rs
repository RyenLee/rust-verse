//! Target management commands — thin forwarding layer.

use tauri::{AppHandle, State};

use crate::domain::error::AppResult;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::domain::parsing;
use crate::infrastructure::exec;
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export for backward compatibility
pub use crate::domain::entity::TargetInfo;
#[allow(unused_imports)]
pub use crate::domain::parsing::parse_target_list;

/// List targets for a toolchain.
#[tauri::command]
pub async fn list_targets(
    rustup_path: String,
    toolchain: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<TargetInfo>> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;

    let cache_key = format!("target_list:{}:{}", rustup_path, toolchain);
    if let Some(cached_json) = state.query_cache.get(&cache_key) {
        if let Ok(targets) = serde_json::from_str::<Vec<TargetInfo>>(&cached_json) {
            return Ok(targets);
        }
    }

    let output = match exec::run_command(
        &rustup_path,
        &["target", "list", "--toolchain", &toolchain],
        30,
    )
    .await
    {
        Ok(out) => out,
        Err(crate::domain::error::AppError::Command(msg)) => {
            let lower = msg.to_lowercase();
            if lower.contains("missing manifest") {
                return Err(crate::domain::error::AppError::Command(format!(
                    "Toolchain '{toolchain}' is corrupted or incomplete. \
                    Please reinstall it via 'rustup toolchain uninstall {toolchain}' then 'rustup toolchain install {toolchain}'."
                )));
            }
            return Err(crate::domain::error::AppError::Command(msg));
        }
        Err(e) => return Err(e),
    };
    let db_parsing = state.config_cache.get_parsing(&*state.store);
    let targets = parsing::parse_target_list(
        &output,
        &db_parsing.installed_marker,
        &db_parsing.default_marker,
    );

    if let Ok(json) = serde_json::to_string(&targets) {
        state.query_cache.set(cache_key, json);
    }

    Ok(targets)
}

/// Add a target to a toolchain.
#[tauri::command]
pub async fn add_target(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
    toolchain: String,
    target: String,
) -> AppResult<()> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;

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

    let (locale_key, log_event, finished_event) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        (locale_key, events.install_log, events.install_finished)
    };

    let result = exec::run_command_with_cancel(
        app.clone(),
        &rustup_path,
        &["target", "add", &target, "--toolchain", &toolchain],
        &locale_key,
        &log_event,
        &finished_event,
        120,
        cancel_notify,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;
    state.query_cache.invalidate_all();

    result?;

    let display_target = target.clone();
    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::TargetAdded,
        &[("name", &display_target), ("toolchain", &toolchain)],
        Some("/targets"),
    );

    Ok(())
}

/// Remove a target from a toolchain.
#[tauri::command]
pub async fn remove_target(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
    toolchain: String,
    target: String,
) -> AppResult<()> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;

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

    let (locale_key, log_event, finished_event) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        (locale_key, events.install_log, events.install_finished)
    };

    let result = exec::run_command_with_cancel(
        app.clone(),
        &rustup_path,
        &["target", "remove", &target, "--toolchain", &toolchain],
        &locale_key,
        &log_event,
        &finished_event,
        60,
        cancel_notify,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;
    state.query_cache.invalidate_all();

    result?;

    let display_target = target.clone();
    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::TargetRemoved,
        &[("name", &display_target), ("toolchain", &toolchain)],
        Some("/targets"),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_list() {
        let output =
            "x86_64-pc-windows-msvc (installed)\nx86_64-pc-windows-gnu\naarch64-unknown-linux-gnu";
        let result = parse_target_list(output, "(installed)", "(default)");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "x86_64-pc-windows-msvc");
        assert!(result[0].installed);
        assert!(!result[1].installed);
    }

    #[test]
    fn test_parse_target_list_empty() {
        let result = parse_target_list("", "(installed)", "(default)");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_target_list_with_default() {
        let output = "x86_64-pc-windows-msvc (installed) (default)\naarch64-pc-windows-msvc";
        let result = parse_target_list(output, "(installed)", "(default)");
        assert_eq!(result.len(), 2);
        assert!(result[0].installed);
        assert_eq!(result[0].name, "x86_64-pc-windows-msvc");
    }

    #[test]
    fn test_parse_target_list_many() {
        let output = "x86_64-pc-windows-msvc (installed)\nx86_64-pc-windows-gnu (installed)\naarch64-unknown-linux-gnu\nwasm32-unknown-unknown";
        let result = parse_target_list(output, "(installed)", "(default)");
        assert_eq!(result.len(), 4);
        assert!(result[0].installed);
        assert!(result[1].installed);
        assert!(!result[2].installed);
        assert!(!result[3].installed);
    }
}

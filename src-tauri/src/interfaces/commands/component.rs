//! Component management commands — thin forwarding layer.

use tauri::{AppHandle, State};

use crate::domain::error::AppResult;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::domain::parsing;
use crate::infrastructure::exec;
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export for backward compatibility
pub use crate::domain::entity::ComponentInfo;
#[allow(unused_imports)]
pub use crate::domain::parsing::parse_component_list;

/// List components for a toolchain.
#[tauri::command]
pub async fn list_components(
    rustup_path: String,
    toolchain: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<ComponentInfo>> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let output = exec::run_command(
        &rustup_path,
        &["component", "list", "--toolchain", &toolchain],
        30,
    )
    .await?;
    let db_parsing = state.config_cache.get_parsing(&*state.store);
    Ok(parsing::parse_component_list(
        &output,
        &db_parsing.installed_marker,
    ))
}

/// Add a component to a toolchain.
#[tauri::command]
pub async fn add_component(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
    toolchain: String,
    component: String,
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
    let cancel_flag = state.task_state.cancel_flag.clone();

    let (locale_key, log_event, finished_event) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        (locale_key, events.install_log, events.install_finished)
    };

    let result = exec::run_command_with_cancel(
        app.clone(),
        &rustup_path,
        &["component", "add", &component, "--toolchain", &toolchain],
        &locale_key,
        &log_event,
        &finished_event,
        120,
        cancel_flag,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;

    result?;

    let display_comp = component.clone();
    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::ComponentAdded,
        &[("name", &display_comp), ("toolchain", &toolchain)],
        Some("/components"),
    );

    Ok(())
}

/// Remove a component from a toolchain.
#[tauri::command]
pub async fn remove_component(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
    toolchain: String,
    component: String,
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
    let cancel_flag = state.task_state.cancel_flag.clone();

    let (locale_key, log_event, finished_event) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        (locale_key, events.install_log, events.install_finished)
    };

    let result = exec::run_command_with_cancel(
        app.clone(),
        &rustup_path,
        &["component", "remove", &component, "--toolchain", &toolchain],
        &locale_key,
        &log_event,
        &finished_event,
        60,
        cancel_flag,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;

    result?;

    let display_comp = component.clone();
    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::ComponentRemoved,
        &[("name", &display_comp), ("toolchain", &toolchain)],
        Some("/components"),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_component_list() {
        let output = "rustfmt (installed)\nclippy (installed)\nrls\nrust-analysis";
        let result = parse_component_list(output, "(installed)");
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].name, "rustfmt");
        assert!(result[0].installed);
        assert_eq!(result[2].name, "rls");
        assert!(!result[2].installed);
    }

    #[test]
    fn test_parse_component_list_empty() {
        let result = parse_component_list("", "(installed)");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_component_list_with_toolchain_suffix() {
        let output = "rustfmt-x86_64-pc-windows-msvc (installed)\nclippy-x86_64-pc-windows-msvc";
        let result = parse_component_list(output, "(installed)");
        assert_eq!(result.len(), 2);
        assert!(result[0].installed);
        assert!(!result[1].installed);
    }

    #[test]
    fn test_parse_component_list_whitespace() {
        let output = "  rustfmt (installed)  \n  rls  ";
        let result = parse_component_list(output, "(installed)");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "rustfmt");
        assert_eq!(result[1].name, "rls");
    }
}

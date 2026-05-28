//! Toolchain management commands — thin forwarding layer.
//!
//! See [super::env_var] for the design principle of thin command layers.

use tauri::{AppHandle, State};

use crate::domain::error::AppResult;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::domain::parsing;
use crate::infrastructure::exec;
use crate::infrastructure::logger;
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export for backward compatibility
pub use crate::domain::entity::ToolchainInfo;
#[allow(unused_imports)]
pub use crate::domain::parsing::{parse_channel_from_name, parse_toolchain_list};

/// List all installed toolchains via `rustup toolchain list`.
///
/// For toolchains with date-based names (e.g. `stable-2026-03-26-x86_64-pc-windows-msvc`),
/// the `display_name` field replaces the date with the actual rustc version number
/// (e.g. `stable-1.95.0-x86_64-pc-windows-msvc`).
#[tauri::command]
pub async fn list_toolchains(
    rustup_path: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<ToolchainInfo>> {
    logger::logger().log_request("list_toolchains", &format!("rustup_path={:?}", rustup_path));
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let parsing = state.config_cache.get_parsing(&*state.store);

    let output = exec::run_command(&rustup_path, &["toolchain", "list"], 30).await?;
    let mut toolchains =
        parsing::parse_toolchain_list(&output, &parsing.default_marker, &parsing.active_marker)?;

    // Resolve version numbers for date-based toolchain names
    for tc in &mut toolchains {
        if parsing::toolchain_name_has_date(&tc.name) {
            if let Ok(ver_output) =
                exec::run_command(&rustup_path, &["run", &tc.name, "rustc", "--version"], 15).await
            {
                if let Some(version) = parsing::parse_rustc_version(&ver_output) {
                    tc.display_name = parsing::build_display_name(&tc.name, &version);
                }
            }
        }
    }

    Ok(toolchains)
}

/// Install a toolchain with streaming output.
#[tauri::command]
pub async fn install_toolchain(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
    channel: String,
    date: Option<String>,
) -> AppResult<()> {
    logger::logger().info(
        "toolchain",
        &format!(
            "install_toolchain requested: {} {}",
            channel,
            date.as_deref().unwrap_or("")
        ),
    );
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        (locale_key, events.install_log, events.install_finished)
    };

    let toolchain_name = if let Some(ref d) = date {
        format!("{channel}-{d}")
    } else {
        channel.clone()
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
    let cancel_flag = state.task_state.cancel_flag.clone();

    let result = exec::run_command_with_cancel(
        app.clone(),
        &rustup_path,
        &["toolchain", "install", &toolchain_name],
        &locale_key,
        &log_event,
        &finished_event,
        600,
        cancel_flag,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;

    match result {
        Ok(()) => {
            notifier::notify(
                &app,
                Category::Install,
                Priority::High,
                NotificationKey::ToolchainInstalled,
                &[("channel", &channel)],
                Some("/toolchains"),
            );
            Ok(())
        }
        Err(e) => {
            notifier::notify(
                &app,
                Category::Operation,
                Priority::High,
                NotificationKey::ToolchainInstallFailed,
                &[("channel", &channel), ("error", &format!("{e}"))],
                Some("/toolchains"),
            );
            Err(e)
        }
    }
}

/// Uninstall a toolchain.
#[tauri::command]
pub async fn uninstall_toolchain(
    app: AppHandle,
    rustup_path: String,
    name: String,
) -> AppResult<()> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    exec::run_command(&rustup_path, &["toolchain", "uninstall", &name], 120).await?;

    // ── Notification: toolchain uninstalled ──
    let display_name = name.clone();
    notifier::notify(
        &app,
        Category::Install,
        Priority::Medium,
        NotificationKey::ToolchainUninstalled,
        &[("name", &display_name)],
        Some("/toolchains"),
    );

    Ok(())
}

/// Set the default toolchain.
#[tauri::command]
pub async fn set_default_toolchain(
    app: AppHandle,
    rustup_path: String,
    name: String,
) -> AppResult<()> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    exec::run_command(&rustup_path, &["default", &name], 30).await?;

    // ── Notification: default toolchain changed ──
    let display_name = name.clone();
    notifier::notify(
        &app,
        Category::Operation,
        Priority::Medium,
        NotificationKey::DefaultToolchainChanged,
        &[("name", &display_name)],
        Some("/toolchains"),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toolchain_list_default() {
        let output = "stable-x86_64-pc-windows-msvc (default)\nnightly-x86_64-pc-windows-msvc";
        let result = parse_toolchain_list(output, "(default)", "(active)").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "stable-x86_64-pc-windows-msvc");
        assert!(result[0].is_default);
        assert!(!result[0].is_active);
        assert_eq!(result[0].channel, "stable");
        assert_eq!(result[1].channel, "nightly");
        assert!(!result[1].is_default);
    }

    #[test]
    fn test_parse_toolchain_list_active_default() {
        let output =
            "stable-x86_64-pc-windows-msvc (active, default)\nnightly-x86_64-pc-windows-msvc";
        let result = parse_toolchain_list(output, "(default)", "(active)").unwrap();
        assert_eq!(result[0].name, "stable-x86_64-pc-windows-msvc");
        assert!(result[0].is_default);
        assert!(!result[0].is_active);
    }

    #[test]
    fn test_parse_toolchain_list_active() {
        let output = "stable-x86_64-pc-windows-msvc\nnightly-x86_64-pc-windows-msvc (active)";
        let result = parse_toolchain_list(output, "(default)", "(active)").unwrap();
        assert!(result[1].is_active);
    }

    #[test]
    fn test_parse_toolchain_list_version() {
        let output = "1.75.0-x86_64-pc-windows-msvc";
        let result = parse_toolchain_list(output, "(default)", "(active)").unwrap();
        assert_eq!(result[0].channel, "1.75.0");
    }

    #[test]
    fn test_parse_toolchain_list_empty() {
        let result = parse_toolchain_list("", "(default)", "(active)").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_channel_nightly_date() {
        let channel = parse_channel_from_name("nightly-2024-01-01-x86_64-pc-windows-msvc");
        assert_eq!(channel, "nightly");
    }

    #[test]
    fn test_parse_toolchain_list_with_override() {
        let output = "stable-x86_64-pc-windows-msvc (default)\nnightly-x86_64-pc-windows-msvc (active) (override)";
        let result = parse_toolchain_list(output, "(default)", "(active)").unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].is_default);
        assert!(result[1].is_active);
    }

    #[test]
    fn test_parse_toolchain_list_whitespace() {
        let output = "  stable-x86_64-pc-windows-msvc (default)  \n  beta-x86_64-pc-windows-msvc  ";
        let result = parse_toolchain_list(output, "(default)", "(active)").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "stable-x86_64-pc-windows-msvc");
        assert_eq!(result[1].channel, "beta");
    }

    #[test]
    fn test_parse_channel_custom() {
        let channel = parse_channel_from_name("custom-toolchain-name");
        assert_eq!(channel, "custom");
    }
}

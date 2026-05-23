use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::state::AppState;
use crate::utils::exec;

/// Information about a single installed toolchain.
#[derive(Debug, Clone, Serialize)]
pub struct ToolchainInfo {
    /// Full toolchain name, e.g. "stable-x86_64-pc-windows-msvc"
    pub name: String,
    /// Channel: stable, beta, nightly, or custom
    pub channel: String,
    /// Whether this is the default toolchain
    pub is_default: bool,
    /// Whether this is the active toolchain for the current directory
    pub is_active: bool,
}

/// List all installed toolchains via `rustup toolchain list`.
#[tauri::command]
pub async fn list_toolchains(rustup_path: String, state: State<'_, AppState>) -> AppResult<Vec<ToolchainInfo>> {
    let parsing = crate::db::get_parsing_config(&state.db);
    let default_marker = parsing.default_marker;
    let active_marker = parsing.active_marker;

    let output = exec::run_command(&rustup_path, &["toolchain", "list"]).await?;

    Ok(parse_toolchain_list(&output, &default_marker, &active_marker)?)
}

/// Install a toolchain with streaming output.
///
/// Emits config-specified log events with each line of output,
/// and a finished event when done.
#[tauri::command]
pub async fn install_toolchain(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
    channel: String,
    date: Option<String>,
) -> AppResult<()> {
    let (locale_key, log_event, finished_event) = {
        let events = crate::db::get_events_config(&state.db);
        let locale_key = crate::db::get_simple(&state.db, "locale.force_locale")
            .unwrap_or_else(crate::db::default_force_locale);
        (
            locale_key,
            events.install_log,
            events.install_finished,
        )
    };

    let toolchain_name = if let Some(ref d) = date {
        format!("{channel}-{d}")
    } else {
        channel.clone()
    };

    exec::run_command_with_streaming(
        app,
        &rustup_path,
        &["toolchain", "install", &toolchain_name],
        &locale_key,
        &log_event,
        &finished_event,
    )
    .await
}

/// Uninstall a toolchain.
#[tauri::command]
pub async fn uninstall_toolchain(rustup_path: String, name: String) -> AppResult<()> {
    exec::run_command(&rustup_path, &["toolchain", "uninstall", &name]).await?;
    Ok(())
}

/// Set the default toolchain.
#[tauri::command]
pub async fn set_default_toolchain(rustup_path: String, name: String) -> AppResult<()> {
    exec::run_command(&rustup_path, &["default", &name]).await?;
    Ok(())
}

/// Parse the output of `rustup toolchain list` into structured data.
pub fn parse_toolchain_list(output: &str, default_marker: &str, active_marker: &str) -> AppResult<Vec<ToolchainInfo>> {
    let mut toolchains = Vec::new();

    // Strip parentheses from markers for the combined pattern check
    let default_text = default_marker.trim_matches(|c| c == '(' || c == ')');
    let active_text = active_marker.trim_matches(|c| c == '(' || c == ')');

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let is_default = line.contains(default_marker)
            || line.contains(&format!("(active, {default_text})"))
            || line.contains(&format!("({active_text}, {default_text})"));
        let is_active = line.contains(active_marker) && !is_default;

        // Extract the toolchain name: everything before the first '('
        let name = line.split('(').next().unwrap_or("").trim().to_string();

        if name.is_empty() {
            continue;
        }

        let channel = parse_channel_from_name(&name);

        toolchains.push(ToolchainInfo {
            name,
            channel,
            is_default,
            is_active,
        });
    }

    Ok(toolchains)
}

/// Extract the channel from a toolchain name.
fn parse_channel_from_name(name: &str) -> String {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.is_empty() {
        return name.to_string();
    }

    match parts[0] {
        "stable" | "beta" | "nightly" => return parts[0].to_string(),
        _ => {}
    }

    if parts[0].parse::<f64>().is_ok() {
        return parts[0].to_string();
    }

    parts[0].to_string()
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

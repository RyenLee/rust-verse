use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::state::AppState;
use crate::utils::exec;

/// Update status for a single toolchain.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    /// Toolchain name
    pub toolchain: String,
    /// Whether it's up to date
    pub up_to_date: bool,
    /// New version available (if any)
    pub new_version: Option<String>,
    /// Current version
    pub current_version: Option<String>,
}

/// Check for available updates with a configurable timeout.
#[tauri::command]
pub async fn check_update(rustup_path: String, state: State<'_, AppState>) -> AppResult<Vec<UpdateInfo>> {
    crate::system::env::validate_rust_binary(&rustup_path).map_err(|e| crate::error::AppError::Command(e))?;
    let timeout = crate::db::get_simple(&state.db, "timeouts.rustup_check_seconds")
        .and_then(|s| s.parse().ok())
        .unwrap_or(30); // default 30s timeout
    let output = exec::run_command_with_timeout(&rustup_path, &["check"], timeout).await?;

    eprintln!("[check_update] raw rustup check output:\n{output}");

    let parsing = crate::db::get_parsing_config(&state.db);
    Ok(parse_check_update(
        &output,
        &parsing.status_separator,
        &parsing.up_to_date,
        &parsing.update_available,
        &parsing.version_separator,
    ))
}

/// Update all toolchains with streaming output.
///
/// Emits config-specified log events with each line of output,
/// and a finished event when done.
#[tauri::command]
pub async fn update_all(app: AppHandle, state: State<'_, AppState>, rustup_path: String) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&rustup_path).map_err(|e| crate::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event) = {
        let events = crate::db::get_events_config(&state.db);
        let locale_key = crate::db::get_simple(&state.db, "locale.force_locale")
            .unwrap_or_else(crate::db::default_force_locale);
        (
            locale_key,
            events.update_log,
            events.update_finished,
        )
    };

    exec::run_command_with_streaming(
        app,
        &rustup_path,
        &["update"],
        &locale_key,
        &log_event,
        &finished_event,
        600, // 10 minute timeout for toolchain updates
    )
    .await
}

/// Update rustup itself with streaming output.
///
/// Emits config-specified log events with each line of output,
/// and a finished event when done.
#[tauri::command]
pub async fn update_rustup(app: AppHandle, state: State<'_, AppState>, rustup_path: String) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&rustup_path).map_err(|e| crate::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event) = {
        let events = crate::db::get_events_config(&state.db);
        let locale_key = crate::db::get_simple(&state.db, "locale.force_locale")
            .unwrap_or_else(crate::db::default_force_locale);
        (
            locale_key,
            events.update_log,
            events.update_finished,
        )
    };

    exec::run_command_with_streaming(
        app,
        &rustup_path,
        &["self", "update"],
        &locale_key,
        &log_event,
        &finished_event,
        300, // 5 minute timeout for rustup self-update
    )
    .await
}

/// Check whether a string looks like a valid rustup toolchain name.
///
/// Accepts patterns like:
/// - `rustup`
/// - `stable-x86_64-pc-windows-msvc`
/// - `nightly-x86_64-unknown-linux-gnu`
/// - `1.85.0-x86_64-pc-windows-msvc`
fn is_valid_toolchain_name(name: &str) -> bool {
    if name == "rustup" {
        return true;
    }
    // Toolchain names contain a target triple with at least one dash-separated
    // component after the channel (e.g. "stable-x86_64-pc-windows-msvc")
    let parts: Vec<&str> = name.splitn(2, '-').collect();
    if parts.len() != 2 {
        return false;
    }
    let channel = parts[0];
    let rest = parts[1];
    // Channel must be a known name or a version number
    let valid_channel = matches!(channel, "stable" | "nightly" | "beta")
        || channel.chars().next().map_or(false, |c| c.is_ascii_digit());
    // Target triple must contain at least 3 dash-separated parts
    let has_target_triple = rest.split('-').count() >= 3;
    valid_channel && has_target_triple
}

/// Parse `rustup check` output.
pub fn parse_check_update(
    output: &str,
    status_separator: &str,
    up_to_date_marker: &str,
    update_available_marker: &str,
    version_separator: &str,
) -> Vec<UpdateInfo> {
    let mut updates = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, status_separator).collect();
        if parts.len() != 2 {
            continue;
        }

        let toolchain = parts[0].trim().to_string();
        let status = parts[1].trim();

        // Validate toolchain name looks like a real rustup toolchain
        // Expected formats: "stable-x86_64-pc-windows-msvc", "nightly-...", "1.85.0-...", "rustup"
        if !is_valid_toolchain_name(&toolchain) {
            continue;
        }

        let (up_to_date, new_version, current_version) = if status.starts_with(up_to_date_marker) {
            let ver = status.split(": ").nth(1).map(|v| v.trim().to_string());
            (true, None, ver)
        } else if status.starts_with(update_available_marker) {
            let after_colon = status.split(": ").nth(1).unwrap_or("").trim();
            let version_parts: Vec<&str> = after_colon.split(version_separator).collect();
            let cur = version_parts.first().map(|v| v.trim().to_string());
            let new = version_parts.get(1).map(|v| v.trim().to_string());
            (false, new, cur)
        } else {
            // Unrecognized status line (e.g. informational messages from rustup)
            // Skip instead of treating as "update available"
            continue;
        };

        updates.push(UpdateInfo {
            toolchain,
            up_to_date,
            new_version,
            current_version,
        });
    }

    updates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_check_update_uptodate() {
        let output = "stable-x86_64-pc-windows-msvc - Up to date : 1.75.0 (82e1608df 2023-12-21)";
        let result = parse_check_update(output, " - ", "Up to date", "Update available", " -> ");
        assert_eq!(result.len(), 1);
        assert!(result[0].up_to_date);
        assert_eq!(result[0].toolchain, "stable-x86_64-pc-windows-msvc");
        assert!(result[0].new_version.is_none());
    }

    #[test]
    fn test_parse_check_update_available() {
        let output = "nightly-x86_64-pc-windows-msvc - Update available : 1.77.0-nightly -> 1.78.0-nightly";
        let result = parse_check_update(output, " - ", "Up to date", "Update available", " -> ");
        assert_eq!(result.len(), 1);
        assert!(!result[0].up_to_date);
        assert_eq!(result[0].new_version.as_deref(), Some("1.78.0-nightly"));
    }

    #[test]
    fn test_parse_check_update_mixed() {
        let output = "stable-x86_64-pc-windows-msvc - Up to date : 1.75.0\nnightly-x86_64-pc-windows-msvc - Update available : 1.77.0-nightly -> 1.78.0-nightly";
        let result = parse_check_update(output, " - ", "Up to date", "Update available", " -> ");
        assert_eq!(result.len(), 2);
        assert!(result[0].up_to_date);
        assert!(!result[1].up_to_date);
    }

    #[test]
    fn test_parse_check_update_empty() {
        let result = parse_check_update("", " - ", "Up to date", "Update available", " -> ");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_check_update_unknown_status() {
        // Unrecognized status lines are now skipped instead of treated as "update available"
        let output = "stable-x86_64-pc-windows-msvc - Some unknown status";
        let result = parse_check_update(output, " - ", "Up to date", "Update available", " -> ");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_check_update_invalid_toolchain_name() {
        // Non-toolchain lines (e.g. rustup info/warning messages) are filtered out
        let output = "info: syncing channel updates - some status text";
        let result = parse_check_update(output, " - ", "Up to date", "Update available", " -> ");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_check_update_no_arrow() {
        let output = "nightly-x86_64-pc-windows-msvc - Update available : 1.80.0-nightly";
        let result = parse_check_update(output, " - ", "Up to date", "Update available", " -> ");
        assert_eq!(result.len(), 1);
        assert!(!result[0].up_to_date);
        assert_eq!(result[0].current_version.as_deref(), Some("1.80.0-nightly"));
        assert!(result[0].new_version.is_none());
    }

    #[test]
    fn test_parse_check_update_uptodate_no_version() {
        let output = "stable-x86_64-pc-windows-msvc - Up to date";
        let result = parse_check_update(output, " - ", "Up to date", "Update available", " -> ");
        assert_eq!(result.len(), 1);
        assert!(result[0].up_to_date);
        assert!(result[0].current_version.is_none());
    }
}

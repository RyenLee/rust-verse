use serde::Serialize;
use std::error::Error;
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
pub async fn check_update(
    rustup_path: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<UpdateInfo>> {
    crate::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::error::AppError::Command(e))?;
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

/// Update all toolchains with streaming output and retry support.
///
/// Emits config-specified log events with each line of output,
/// and a finished event when done.
/// On failure, retries up to the configured number of times with exponential backoff.
#[tauri::command]
pub async fn update_all(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event, max_retries, retry_delay_ms) = {
        let events = crate::db::get_events_config(&state.db);
        let locale_key = crate::db::get_simple(&state.db, "locale.force_locale")
            .unwrap_or_else(crate::db::default_force_locale);
        let max_retries: u32 = crate::db::get_simple(&state.db, "retry.update_max_retries")
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);
        let retry_delay_ms: u64 = crate::db::get_simple(&state.db, "retry.update_delay_ms")
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);
        (
            locale_key,
            events.update_log,
            events.update_finished,
            max_retries,
            retry_delay_ms,
        )
    };

    let result = exec::run_command_with_streaming_retry(
        app,
        &rustup_path,
        &["update"],
        &locale_key,
        &log_event,
        &finished_event,
        max_retries,
        retry_delay_ms,
        600, // 10 minute timeout for toolchain updates
    )
    .await;

    result
}

/// Update rustup itself with streaming output and retry support.
///
/// Emits config-specified log events with each line of output,
/// and a finished event when done.
/// On failure, retries up to the configured number of times with exponential backoff.
#[tauri::command]
pub async fn update_rustup(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event, max_retries, retry_delay_ms) = {
        let events = crate::db::get_events_config(&state.db);
        let locale_key = crate::db::get_simple(&state.db, "locale.force_locale")
            .unwrap_or_else(crate::db::default_force_locale);
        let max_retries: u32 = crate::db::get_simple(&state.db, "retry.update_max_retries")
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);
        let retry_delay_ms: u64 = crate::db::get_simple(&state.db, "retry.update_delay_ms")
            .and_then(|s| s.parse().ok())
            .unwrap_or(3000);
        (
            locale_key,
            events.update_log,
            events.update_finished,
            max_retries,
            retry_delay_ms,
        )
    };

    exec::run_command_with_streaming_retry(
        app,
        &rustup_path,
        &["self", "update"],
        &locale_key,
        &log_event,
        &finished_event,
        max_retries,
        retry_delay_ms,
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

/// Network diagnostic result for debugging updater connectivity.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkDiagResult {
    /// Whether the overall test passed
    pub success: bool,
    /// DNS resolution result for github.com
    pub dns: String,
    /// TCP connection test to github.com:443
    pub tcp: String,
    /// HTTP GET result for the update JSON endpoint
    pub http: String,
    /// Full HTTP response status code (if any)
    pub http_status: Option<u16>,
    /// HTTP response body snippet (first 500 chars)
    pub http_body: Option<String>,
    /// Total elapsed time (ms)
    pub elapsed_ms: u64,
}

/// Test network connectivity to the update server for diagnostics.
#[tauri::command]
pub async fn diag_network() -> AppResult<NetworkDiagResult> {
    let start = std::time::Instant::now();
    // Use our own reqwest client that completely ignores system proxy
    let client = reqwest::Client::builder()
        .no_proxy()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| crate::error::AppError::Command(e.to_string()))?;

    // 1. DNS test
    let dns = match tokio::net::lookup_host("github.com:443").await {
        Ok(addrs) => {
            let list: Vec<String> = addrs.map(|a| a.to_string()).collect();
            format!(
                "OK: {} addresses resolved ({})",
                list.len(),
                list.join(", ")
            )
        }
        Err(e) => format!("FAIL: {e}"),
    };

    // 2. TCP test
    let tcp = match tokio::net::TcpStream::connect("github.com:443").await {
        Ok(_) => "OK: TCP connection succeeded".to_string(),
        Err(e) => format!("FAIL: {e}"),
    };

    // 3. HTTP test
    let url = "https://github.com/RyenLee/rust-verse/releases/latest/download/latest.json";
    let (http, http_status, http_body) = match client.get(url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("(failed to read body: {e})"));
            let snippet: String = body.chars().take(500).collect();
            (format!("OK: HTTP {}", status), Some(status), Some(snippet))
        }
        Err(e) => {
            let msg = format!("FAIL: {e}");
            // Try to extract the underlying cause
            let detail = if let Some(src) = e.source() {
                format!(" | source: {src}")
            } else {
                String::new()
            };
            (format!("{msg}{detail}"), None, None)
        }
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let success = http_status.is_some();

    Ok(NetworkDiagResult {
        success,
        dns,
        tcp,
        http,
        http_status,
        http_body,
        elapsed_ms,
    })
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
        let output =
            "nightly-x86_64-pc-windows-msvc - Update available : 1.77.0-nightly -> 1.78.0-nightly";
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

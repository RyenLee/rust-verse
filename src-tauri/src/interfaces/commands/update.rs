//! Toolchain update commands — thin forwarding layer.

use std::error::Error;

use tauri::{AppHandle, State};

use crate::domain::config_keys::keys;
use crate::domain::constants::{log_module, page_route};
use crate::domain::error::AppResult;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::domain::parsing;
use crate::infrastructure::exec;
use crate::infrastructure::logger;
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export for backward compatibility
pub use crate::domain::entity::{NetworkDiagResult, UpdateInfo};
#[allow(unused_imports)]
pub use crate::domain::parsing::{is_valid_toolchain_name, parse_check_update};

fn read_retry_config(store: &dyn crate::domain::repository::DataStore) -> (u32, u64) {
    let batch = store.get_config_batch(&[keys::RETRY_UPDATE_MAX, keys::RETRY_UPDATE_DELAY]);
    let max_retries: u32 = batch
        .get(keys::RETRY_UPDATE_MAX)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let retry_delay_ms: u64 = batch
        .get(keys::RETRY_UPDATE_DELAY)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    (max_retries, retry_delay_ms)
}

/// Check for available updates with a configurable timeout.
///
/// Results are cached for the duration of the global QueryCache TTL (60s)
/// to avoid repeated network requests when the user navigates back and forth.
/// The cache is invalidated automatically after any toolchain install/uninstall
/// or update operation.
#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    rustup_path: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<UpdateInfo>> {
    logger::logger().log_request("check_update", &format!("rustup_path={:?}", rustup_path));
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;

    let cache_key = format!("update_check:{}", rustup_path);
    if let Some(cached_json) = state.query_cache.get(&cache_key) {
        if let Ok(updates) = serde_json::from_str::<Vec<UpdateInfo>>(&cached_json) {
            return Ok(updates);
        }
    }

    let timeout = state.config_cache.get_timeout_rustup_check(&*state.store);
    let output = exec::run_command_with_timeout_allow_codes(&rustup_path, &["check"], timeout, &[100]).await?;
    logger::logger().debug(
        log_module::UPDATE,
        &format!("[check_update] raw rustup check output:\n{output}"),
    );

    let db_parsing = state.config_cache.get_parsing(&*state.store);
    let updates = parsing::parse_check_update(
        &output,
        &db_parsing.status_separator,
        &db_parsing.up_to_date,
        &db_parsing.update_available,
        &db_parsing.version_separator,
    );

    if let Ok(json) = serde_json::to_string(&updates) {
        state.query_cache.set(cache_key, json);
    }

    // ── Notification: updates available ──
    let pending: Vec<_> = updates.iter().filter(|u| !u.up_to_date).collect();
    if !pending.is_empty() {
        let names: Vec<&str> = pending.iter().map(|u| u.toolchain.as_str()).collect();
        let count_str = pending.len().to_string();
        let names_str = names.join(", ");
        notifier::notify(
            &app,
            Category::Update,
            Priority::High,
            NotificationKey::ToolchainUpdatesAvailable,
            &[("count", &count_str), ("names", &names_str)],
            Some(page_route::UPDATES),
        );
    }

    Ok(updates)
}

/// Update all toolchains with streaming output and retry support.
#[tauri::command]
pub async fn update_all(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
) -> AppResult<()> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event, max_retries, retry_delay_ms) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        let (max_retries, retry_delay_ms) = read_retry_config(&*state.store);
        (
            locale_key,
            events.update_log,
            events.update_finished,
            max_retries,
            retry_delay_ms,
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
    let cancel_flag = state.task_state.cancel_flag.clone();
    let cancel_notify = state.task_state.cancel_notify.clone();

    let result = exec::run_command_with_cancel_retry(
        app.clone(),
        &rustup_path,
        &["update"],
        &locale_key,
        &log_event,
        &finished_event,
        max_retries,
        retry_delay_ms,
        600,
        cancel_flag,
        cancel_notify,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;
    state.query_cache.invalidate_all();

    match result {
        Ok(()) => {
            notifier::notify(
                &app,
                Category::Update,
                Priority::High,
                NotificationKey::ToolchainsUpdated,
                &[],
                Some(page_route::UPDATES),
            );
            Ok(())
        }
        Err(e) => {
            notifier::notify(
                &app,
                Category::Operation,
                Priority::High,
                NotificationKey::ToolchainUpdateFailed,
                &[("error", &format!("{e}"))],
                Some(page_route::UPDATES),
            );
            Err(e)
        }
    }
}

/// Update rustup itself with streaming output and retry support.
#[tauri::command]
pub async fn update_rustup(
    app: AppHandle,
    state: State<'_, AppState>,
    rustup_path: String,
) -> AppResult<()> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event, max_retries, retry_delay_ms) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        let (max_retries, retry_delay_ms) = read_retry_config(&*state.store);
        (
            locale_key,
            events.update_log,
            events.update_finished,
            max_retries,
            retry_delay_ms,
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
    let cancel_flag = state.task_state.cancel_flag.clone();
    let cancel_notify = state.task_state.cancel_notify.clone();

    let result = exec::run_command_with_cancel_retry(
        app.clone(),
        &rustup_path,
        &["self", "update"],
        &locale_key,
        &log_event,
        &finished_event,
        max_retries,
        retry_delay_ms,
        300,
        cancel_flag,
        cancel_notify,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;
    state.query_cache.invalidate_all();

    match result {
        Ok(()) => {
            notifier::notify(
                &app,
                Category::Update,
                Priority::High,
                NotificationKey::RustupUpdated,
                &[],
                Some(page_route::UPDATES),
            );
            Ok(())
        }
        Err(e) => {
            notifier::notify(
                &app,
                Category::Operation,
                Priority::High,
                NotificationKey::RustupUpdateFailed,
                &[("error", &format!("{e}"))],
                Some(page_route::UPDATES),
            );
            Err(e)
        }
    }
}

/// Test network connectivity to the update server for diagnostics.
#[tauri::command]
pub async fn diag_network(app: AppHandle) -> AppResult<NetworkDiagResult> {
    let start = std::time::Instant::now();
    let diag_timeout = std::time::Duration::from_secs(10);
    let client = reqwest::Client::builder()
        .no_proxy()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .timeout(diag_timeout)
        .build()
        .map_err(|e| crate::domain::error::AppError::Command(e.to_string()))?;

    // 1. DNS test — GitHub
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

    // 2. TCP test — GitHub (with explicit timeout)
    let tcp = match tokio::time::timeout(diag_timeout, tokio::net::TcpStream::connect("github.com:443")).await {
        Ok(Ok(_)) => "OK: TCP connection succeeded".to_string(),
        Ok(Err(e)) => format!("FAIL: {e}"),
        Err(_) => "FAIL: connection timed out (10s)".to_string(),
    };

    // 3. HTTP test — GitHub update server
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
            let detail = if let Some(src) = e.source() {
                format!(" | source: {src}")
            } else {
                String::new()
            };
            (format!("{msg}{detail}"), None, None)
        }
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let github_reachable = http_status.is_some();

    // 4. Generate conclusion with actionable advice
    let dns_ok = dns.starts_with("OK");
    let tcp_ok = tcp.starts_with("OK");
    let http_ok = http.starts_with("OK");

    let conclusion = if http_ok {
        "All tests passed — the update server is reachable.".to_string()
    } else if dns_ok && !tcp_ok {
        format!(
            "DNS resolved successfully but TCP connection to GitHub was blocked. \
This typically means your network restricts access to GitHub (common in mainland China). \
Suggestions: (1) Use a proxy/VPN to access GitHub; \
(2) Configure proxy in Settings → Proxy; \
(3) If you have a local mirror or corporate network, ensure GitHub IPs are whitelisted."
        )
    } else if !dns_ok {
        format!(
            "DNS resolution failed — your network cannot resolve github.com. \
Please check your DNS settings or try using a public DNS server (e.g., 114.114.114.114 or 8.8.8.8)."
        )
    } else if dns_ok && tcp_ok && !http_ok {
        format!(
            "TCP connected but HTTPS request failed. This may indicate a TLS/SSL issue or \
an HTTP-level proxy blocking the connection. Check your system proxy settings and firewall rules."
        )
    } else {
        "Network diagnostic completed. See individual test results above for details.".to_string()
    };

    if !github_reachable {
        notifier::notify(
            &app,
            Category::Operation,
            Priority::High,
            NotificationKey::NetworkDiagFailed,
            &[],
            Some(page_route::ABOUT),
        );
    }

    Ok(NetworkDiagResult {
        success: github_reachable,
        dns,
        tcp,
        http,
        http_status,
        http_body,
        elapsed_ms,
        conclusion,
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
        let output = "stable-x86_64-pc-windows-msvc - Some unknown status";
        let result = parse_check_update(output, " - ", "Up to date", "Update available", " -> ");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_check_update_invalid_toolchain_name() {
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

use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;

/// Historical release information from rs-histver.
#[derive(Debug, Clone, Serialize)]
pub struct HistRelease {
    pub version: String,
    pub date: String,
    pub channel: String,
}

/// Fetch historical releases from remote and cache locally.
///
/// - `channel`: "stable", "beta", or "nightly"
/// - `full`: use full history source (stable only, fetches RELEASES.md)
/// - `days`: days of history to probe (beta/nightly only)
#[tauri::command]
pub async fn sync_hist_releases(
    state: State<'_, AppState>,
    channel: String,
    full: bool,
    days: u32,
) -> AppResult<u64> {
    // Validate channel name
    if !matches!(channel.as_str(), "stable" | "beta" | "nightly") {
        return Err(crate::error::AppError::Command(format!(
            "Unknown channel '{}'. Must be one of: stable, beta, nightly",
            channel
        )));
    }

    let db_path = histver_db_path(&state);
    let config = rs_histver::Config::with_db_path(&db_path);
    let hv = rs_histver::HistVer::new(config).map_err(|e| {
        crate::error::AppError::Config(format!(
            "Failed to initialize version database ({}): {}",
            db_path.display(),
            e
        ))
    })?;

    let releases = hv.fetch_releases(&channel, full, days).await.map_err(|e| {
        let hint = if e.to_string().contains("dns") || e.to_string().contains("resolve") {
            "DNS resolution failed — check your internet connection or DNS settings."
        } else if e.to_string().contains("timed out") || e.to_string().contains("timeout") {
            "Request timed out — the remote server may be slow or unreachable. Try again later."
        } else if e.to_string().contains("tls") || e.to_string().contains("certificate") {
            "TLS/certificate error — your system clock or root certificates may be outdated."
        } else if e.to_string().contains("connection refused") {
            "Connection refused — the remote server may be down."
        } else {
            "Check your internet connection and try again."
        };
        crate::error::AppError::Network(format!(
            "Failed to fetch {} release data: {}. {}",
            channel, e, hint
        ))
    })?;

    if releases.is_empty() {
        return Err(crate::error::AppError::Network(format!(
            "No {} release data found on the remote server. The data source may be temporarily unavailable.",
            channel
        )));
    }

    let count = hv.store_releases(&releases).map_err(|e| {
        crate::error::AppError::Command(format!(
            "Failed to save release data to local cache ({}): {}",
            db_path.display(),
            e
        ))
    })?;

    Ok(count)
}

/// List cached historical releases, optionally filtered by channel.
#[tauri::command]
pub fn list_hist_releases(
    state: State<'_, AppState>,
    channel: Option<String>,
) -> AppResult<Vec<HistRelease>> {
    let config = rs_histver::Config::with_db_path(histver_db_path(&state));
    let hv = rs_histver::HistVer::new(config)
        .map_err(|e| crate::error::AppError::Command(format!("failed to init histver: {e}")))?;

    let ch = channel.as_deref();
    let releases = hv
        .list_releases(ch)
        .map_err(|e| crate::error::AppError::Command(format!("failed to list releases: {e}")))?;

    Ok(releases
        .into_iter()
        .map(|r| HistRelease {
            version: r.version,
            date: r.date,
            channel: r.channel,
        })
        .collect())
}

/// Search cached historical releases by keyword.
#[tauri::command]
pub fn search_hist_releases(
    state: State<'_, AppState>,
    keyword: String,
    channel: Option<String>,
) -> AppResult<Vec<HistRelease>> {
    let config = rs_histver::Config::with_db_path(histver_db_path(&state));
    let hv = rs_histver::HistVer::new(config)
        .map_err(|e| crate::error::AppError::Command(format!("failed to init histver: {e}")))?;

    let ch = channel.as_deref();
    let results = hv
        .search_releases(&keyword, ch)
        .map_err(|e| crate::error::AppError::Command(format!("failed to search releases: {e}")))?;

    Ok(results
        .into_iter()
        .map(|r| HistRelease {
            version: r.version,
            date: r.date,
            channel: r.channel,
        })
        .collect())
}

/// Count cached historical releases, optionally filtered by channel.
#[tauri::command]
pub fn count_hist_releases(state: State<'_, AppState>, channel: Option<String>) -> AppResult<u64> {
    let config = rs_histver::Config::with_db_path(histver_db_path(&state));
    let hv = rs_histver::HistVer::new(config)
        .map_err(|e| crate::error::AppError::Command(format!("failed to init histver: {e}")))?;

    let ch = channel.as_deref();
    // Use list_releases + len() to avoid Iterator::count ambiguity
    let releases = hv
        .list_releases(ch)
        .map_err(|e| crate::error::AppError::Command(format!("failed to count releases: {e}")))?;
    Ok(releases.len() as u64)
}

/// Compute the histver database path, colocated with the app config database.
fn histver_db_path(_state: &AppState) -> std::path::PathBuf {
    // Place rs-histver.redb alongside the app's config.redb
    // Uses the same logic as get_db_path() in lib.rs
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            return parent.join("data").join("rs-histver.redb");
        }
    }
    std::path::PathBuf::from("data/rs-histver.redb")
}

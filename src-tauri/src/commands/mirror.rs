use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::state::AppState;
use crate::utils::exec;

/// Information about a mirror source from crm.
#[derive(Debug, Clone, Serialize)]
pub struct MirrorInfo {
    /// Mirror name (e.g. "sjtu", "rsproxy-sparse")
    pub name: String,
    /// Index URL from config.toml [registries.<name>].index
    pub index: String,
    /// Mirror type: "sparse" or "git"
    pub mirror_type: String,
    /// Whether this is the currently active mirror (marked with * in crm list)
    pub is_current: bool,
}

/// Latency test result for a single mirror.
#[derive(Debug, Clone, Serialize)]
pub struct MirrorLatency {
    /// Mirror name
    pub name: String,
    /// Whether this is the currently active mirror (marked with *)
    pub is_current: bool,
    /// Network latency in ms, None if failed
    pub network_ms: Option<u64>,
    /// Download latency in ms, None if failed or not tested
    pub download_ms: Option<u64>,
}

/// Result of `crm test` with parsed latency data.
#[derive(Debug, Clone, Serialize)]
pub struct CrmTestResult {
    pub latencies: Vec<MirrorLatency>,
}

/// Check whether crm is installed and functional by running `crm version`.
#[tauri::command]
pub async fn check_crm_installed() -> AppResult<bool> {
    match exec::run_command("crm", &["version"], 10).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Install crm via `cargo install crm` with streaming output.
#[tauri::command]
pub async fn install_crm(
    app: AppHandle,
    state: State<'_, AppState>,
    cargo_path: String,
) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&cargo_path)
        .map_err(|e| crate::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event) = {
        let events = crate::db::get_events_config(&state.db);
        let locale_key = crate::db::get_simple(&state.db, "locale.force_locale")
            .unwrap_or_else(crate::db::default_force_locale);
        (
            locale_key,
            events.plugin_install_log,
            events.plugin_install_finished,
        )
    };

    exec::run_command_with_streaming(
        app,
        &cargo_path,
        &["install", "crm"],
        &locale_key,
        &log_event,
        &finished_event,
        600,
    )
    .await
}

/// List available mirrors from `crm list`.
#[tauri::command]
pub async fn crm_list() -> AppResult<Vec<MirrorInfo>> {
    let output = exec::run_command("crm", &["list"], 30).await?;
    Ok(parse_mirror_list(&output))
}

/// Get the currently active mirror name from `crm current`.
#[tauri::command]
pub async fn crm_current() -> AppResult<String> {
    let output = exec::run_command("crm", &["current"], 15).await?;
    Ok(output.trim().to_string())
}

/// Get the installed crm version.
#[tauri::command]
pub async fn crm_version() -> AppResult<String> {
    let output = exec::run_command("crm", &["version"], 15).await?;
    Ok(output.trim().to_string())
}

/// Switch to a specific mirror using `crm use <name>`.
#[tauri::command]
pub async fn crm_use(name: String) -> AppResult<()> {
    validate_mirror_name(&name)?;
    exec::run_command("crm", &["use", &name], 30).await?;
    Ok(())
}

/// Auto-select the best mirror using `crm best [mode]`.
///
/// `mode` can be: "" (all), "git", "sparse", "git-download", "sparse-download"
#[tauri::command]
pub async fn crm_best(mode: String) -> AppResult<()> {
    validate_best_mode(&mode)?;
    let args = if mode.is_empty() {
        vec!["best"]
    } else {
        vec!["best", &mode]
    };
    exec::run_command("crm", &args, 120).await?;
    Ok(())
}

/// Restore the default official registry using `crm default`.
#[tauri::command]
pub async fn crm_default() -> AppResult<()> {
    exec::run_command("crm", &["default"], 30).await?;
    Ok(())
}

/// Test mirror latency using `crm test [name]`.
///
/// If `name` is None, tests all mirrors.
/// Returns structured latency data.
#[tauri::command]
pub async fn crm_test(name: Option<String>) -> AppResult<CrmTestResult> {
    if let Some(ref n) = name {
        validate_mirror_name(n)?;
    }
    let args = match &name {
        Some(n) => vec!["test", n],
        None => vec!["test"],
    };
    let output = exec::run_command("crm", &args, 120).await?;
    Ok(parse_test_results(&output))
}

/// Validate mirror name to prevent command injection.
fn validate_mirror_name(name: &str) -> AppResult<()> {
    if name.is_empty() {
        return Err(crate::error::AppError::Command(
            "mirror name cannot be empty".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(crate::error::AppError::Command(format!(
            "invalid mirror name '{name}': only alphanumeric, hyphen and underscore allowed"
        )));
    }
    Ok(())
}

/// Validate the best mode parameter.
fn validate_best_mode(mode: &str) -> AppResult<()> {
    match mode {
        "" | "git" | "sparse" | "git-download" | "sparse-download" => Ok(()),
        other => Err(crate::error::AppError::Command(format!(
            "invalid best mode '{other}': allowed values are '', 'git', 'sparse', 'git-download', 'sparse-download'"
        ))),
    }
}

/// Parse the output of `crm list` into a list of MirrorInfo.
///
/// Expected output format:
/// ```text
///   rsproxy         - `https://rsproxy.cn/crates.io-index`
///  * rsproxy-sparse - sparse+`https://rsproxy.cn/index/`
/// ```
///
/// - Lines starting with `*` indicate the current mirror
/// - The `-` separator separates name from index
/// - Index may have `sparse+` prefix or `.git` suffix
fn parse_mirror_list(output: &str) -> Vec<MirrorInfo> {
    let mut mirrors = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Check if this line marks the current mirror with *
        let (is_current, line) = if let Some(rest) = line.strip_prefix('*') {
            (true, rest.trim())
        } else {
            (false, line)
        };

        // Split on " - " to separate name from index part
        let Some((name_part, index_part)) = line.split_once(" - ") else {
            continue;
        };

        let name = name_part.trim().to_string();
        if name.is_empty() {
            continue;
        }

        // Clean up index: remove backtick wrapping and trim
        let index_raw = index_part.trim().trim_matches('`').to_string();

        // Determine mirror type
        let mirror_type = if index_raw.starts_with("sparse+") {
            "sparse".to_string()
        } else if index_raw.ends_with(".git") {
            "git".to_string()
        } else {
            "other".to_string()
        };

        mirrors.push(MirrorInfo {
            name,
            index: index_raw,
            mirror_type,
            is_current,
        });
    }

    mirrors
}

/// Parse the output of `crm test` into structured latency data.
///
/// Expected output format:
/// ```text
/// 网络连接延迟:
///     tuna            -- failed
///     bfsu            -- 96 ms
///   * sjtu-sparse     -- 194 ms
///
/// 软件包下载延迟:
///     ustc            -- failed
///     sjtu            -- 456 ms
/// ```
fn parse_test_results(output: &str) -> CrmTestResult {
    let mut latencies: Vec<MirrorLatency> = Vec::new();
    let mut current_section = ""; // "network" or "download"

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Detect section headers
        if line.contains("网络连接延迟") || line.contains("Network latency") {
            current_section = "network";
            continue;
        }
        if line.contains("软件包下载延迟") || line.contains("Download latency") {
            current_section = "download";
            continue;
        }

        // Parse latency line: "  * mirror-name  --  123 ms" or "  mirror-name  --  failed"
        let (is_current, line) = if let Some(rest) = line.strip_prefix('*') {
            (true, rest.trim())
        } else {
            (false, line)
        };

        let Some((name_part, value_part)) = line.split_once("--") else {
            continue;
        };

        let name = name_part.trim().to_string();
        if name.is_empty() {
            continue;
        }

        let value_part = value_part.trim();

        // Find or create the latency entry
        let entry = latencies.iter_mut().find(|l| l.name == name);
        let entry = if let Some(e) = entry {
            e
        } else {
            latencies.push(MirrorLatency {
                name: name.clone(),
                is_current,
                network_ms: None,
                download_ms: None,
            });
            latencies.last_mut().unwrap()
        };

        // Update is_current if this line has *
        if is_current {
            entry.is_current = true;
        }

        // Parse the value
        if value_part == "failed" {
            // Keep the None value for failed
        } else {
            // Try to extract ms value: "96 ms" -> 96
            let ms: Option<u64> = value_part
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok());

            match current_section {
                "network" => entry.network_ms = ms,
                "download" => entry.download_ms = ms,
                _ => {
                    // If no section detected, treat as network
                    entry.network_ms = ms;
                }
            }
        }
    }

    CrmTestResult { latencies }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mirror_list() {
        let output = "  rsproxy         - `https://rsproxy.cn/crates.io-index`\n * rsproxy-sparse - sparse+`https://rsproxy.cn/index/`\n  ustc            - `https://mirrors.ustc.edu.cn/crates.io-index`\n  tuna            - `https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git`";
        let result = parse_mirror_list(output);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].name, "rsproxy");
        assert_eq!(result[0].mirror_type, "git");
        assert!(!result[0].is_current);
        assert_eq!(result[1].name, "rsproxy-sparse");
        assert_eq!(result[1].mirror_type, "sparse");
        assert!(result[1].is_current);
        assert_eq!(result[2].name, "ustc");
        assert_eq!(result[2].mirror_type, "other");
        assert_eq!(result[3].name, "tuna");
        assert_eq!(result[3].mirror_type, "git");
        assert!(result[3].index.ends_with(".git"));
    }

    #[test]
    fn test_parse_mirror_list_empty() {
        let result = parse_mirror_list("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_test_results() {
        let output = "网络连接延迟:\n    tuna            -- failed\n    bfsu            -- 96 ms\n  * sjtu-sparse     -- 194 ms\n\n软件包下载延迟:\n    ustc            -- failed\n    sjtu            -- 456 ms";
        let result = parse_test_results(output);
        assert_eq!(result.latencies.len(), 5);

        let tuna = result.latencies.iter().find(|l| l.name == "tuna").unwrap();
        assert!(tuna.network_ms.is_none());
        assert!(tuna.download_ms.is_none());

        let bfsu = result.latencies.iter().find(|l| l.name == "bfsu").unwrap();
        assert_eq!(bfsu.network_ms, Some(96));

        let sjtu_sparse = result
            .latencies
            .iter()
            .find(|l| l.name == "sjtu-sparse")
            .unwrap();
        assert_eq!(sjtu_sparse.network_ms, Some(194));
        assert!(sjtu_sparse.is_current);

        let sjtu = result.latencies.iter().find(|l| l.name == "sjtu").unwrap();
        assert_eq!(sjtu.download_ms, Some(456));
    }

    #[test]
    fn test_validate_mirror_name() {
        assert!(validate_mirror_name("sjtu").is_ok());
        assert!(validate_mirror_name("rsproxy-sparse").is_ok());
        assert!(validate_mirror_name("my_mirror").is_ok());
        assert!(validate_mirror_name("").is_err());
        assert!(validate_mirror_name("bad name").is_err());
        assert!(validate_mirror_name("rm -rf /").is_err());
        assert!(validate_mirror_name(";echo").is_err());
    }

    #[test]
    fn test_validate_best_mode() {
        assert!(validate_best_mode("").is_ok());
        assert!(validate_best_mode("git").is_ok());
        assert!(validate_best_mode("sparse").is_ok());
        assert!(validate_best_mode("git-download").is_ok());
        assert!(validate_best_mode("sparse-download").is_ok());
        assert!(validate_best_mode("invalid").is_err());
        assert!(validate_best_mode(";rm -rf").is_err());
    }
}

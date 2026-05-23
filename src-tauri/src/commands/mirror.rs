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
    /// Whether this is a sparse mirror (index starts with "sparse+")
    pub is_sparse: bool,
}

/// Check whether crm is installed on the system.
#[tauri::command]
pub async fn check_crm_installed() -> AppResult<bool> {
    match which::which("crm") {
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
/// Returns the raw output from crm test.
#[tauri::command]
pub async fn crm_test(name: Option<String>) -> AppResult<String> {
    if let Some(ref n) = name {
        validate_mirror_name(n)?;
    }
    let args = match &name {
        Some(n) => vec!["test", n],
        None => vec!["test"],
    };
    let output = exec::run_command("crm", &args, 120).await?;
    Ok(output)
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
/// Expected output format (each line):
///   mirror-name  registry-url
/// or:
///   mirror-name  sparse+registry-url
fn parse_mirror_list(output: &str) -> Vec<MirrorInfo> {
    let mut mirrors = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split on first whitespace
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() != 2 {
            continue;
        }

        let name = parts[0].trim().to_string();
        let index = parts[1].trim().to_string();
        let is_sparse = index.starts_with("sparse+");

        mirrors.push(MirrorInfo {
            name,
            index,
            is_sparse,
        });
    }

    mirrors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mirror_list() {
        let output = "sjtu           https://mirrors.sjtug.sjtu.edu.cn/git/crates.io-index\nrsproxy-sparse sparse+https://rsproxy.cn/index/\nustc           https://mirrors.ustc.edu.cn/crates.io-index";
        let result = parse_mirror_list(output);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "sjtu");
        assert!(!result[0].is_sparse);
        assert_eq!(result[1].name, "rsproxy-sparse");
        assert!(result[1].is_sparse);
        assert_eq!(result[2].name, "ustc");
    }

    #[test]
    fn test_parse_mirror_list_empty() {
        let result = parse_mirror_list("");
        assert!(result.is_empty());
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

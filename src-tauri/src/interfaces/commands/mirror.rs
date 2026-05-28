//! Mirror management commands — thin forwarding layer.

use tauri::{AppHandle, State};

use crate::domain::error::AppResult;
use crate::domain::mirror as mirror_svc;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::domain::parsing;
use crate::infrastructure::exec;
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export for backward compatibility
#[allow(unused_imports)]
pub use crate::domain::entity::{CrmTestResult, MirrorInfo, MirrorLatency};
#[allow(unused_imports)]
pub use crate::domain::mirror::{validate_best_mode, validate_mirror_name};
#[allow(unused_imports)]
pub use crate::domain::parsing::{parse_mirror_list, parse_test_results};

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
    crate::infrastructure::system::env::validate_rust_binary(&cargo_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event) = {
        let events = state.config_cache.get_events(&*state.store);
        let locale_key = state.config_cache.get_locale(&*state.store);
        (
            locale_key,
            events.plugin_install_log,
            events.plugin_install_finished,
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

    let result = exec::run_command_with_cancel(
        app.clone(),
        &cargo_path,
        &["install", "crm"],
        &locale_key,
        &log_event,
        &finished_event,
        600,
        cancel_flag,
    )
    .await;

    // ── Clear running flag ──
    *state.task_state.running.lock().unwrap() = false;

    result?;

    // ── Notification: crm installed ──
    notifier::notify(
        &app,
        Category::Install,
        Priority::Medium,
        NotificationKey::CrmInstalled,
        &[],
        Some("/mirrors"),
    );

    Ok(())
}

/// List available mirrors from `crm list`.
#[tauri::command]
pub async fn crm_list() -> AppResult<Vec<MirrorInfo>> {
    let output = exec::run_command("crm", &["list"], 30).await?;
    Ok(parsing::parse_mirror_list(&output))
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
pub async fn crm_use(app: AppHandle, name: String) -> AppResult<()> {
    mirror_svc::validate_mirror_name(&name)?;
    exec::run_command("crm", &["use", &name], 30).await?;

    notifier::notify(
        &app,
        Category::Operation,
        Priority::Medium,
        NotificationKey::MirrorSwitched,
        &[("name", &name)],
        Some("/mirrors"),
    );

    Ok(())
}

/// Auto-select the best mirror using `crm best [mode]`.
#[tauri::command]
pub async fn crm_best(app: AppHandle, mode: String) -> AppResult<()> {
    mirror_svc::validate_best_mode(&mode)?;
    let args = if mode.is_empty() {
        vec!["best"]
    } else {
        vec!["best", &mode]
    };
    exec::run_command("crm", &args, 120).await?;

    let mode_str = if mode.is_empty() { "default" } else { &mode };
    notifier::notify(
        &app,
        Category::Operation,
        Priority::Medium,
        NotificationKey::MirrorBest,
        &[("mode", mode_str)],
        Some("/mirrors"),
    );

    Ok(())
}

/// Restore the default official registry using `crm default`.
#[tauri::command]
pub async fn crm_default(app: AppHandle) -> AppResult<()> {
    exec::run_command("crm", &["default"], 30).await?;

    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::MirrorReset,
        &[],
        Some("/mirrors"),
    );

    Ok(())
}

/// Test mirror latency using `crm test [name]`.
#[tauri::command]
pub async fn crm_test(name: Option<String>) -> AppResult<CrmTestResult> {
    if let Some(ref n) = name {
        mirror_svc::validate_mirror_name(n)?;
    }
    let args = match &name {
        Some(n) => vec!["test", n],
        None => vec!["test"],
    };
    let output = exec::run_command("crm", &args, 120).await?;
    Ok(parsing::parse_test_results(&output))
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
        assert_eq!(result[0].mirror_type, "other");
        assert!(!result[0].is_current);
        assert_eq!(result[1].name, "rsproxy-sparse");
        assert_eq!(result[1].mirror_type, "sparse");
        assert!(result[1].is_current);
        assert_eq!(result[2].name, "ustc");
        assert_eq!(result[2].mirror_type, "other");
        assert_eq!(result[3].name, "tuna");
        assert_eq!(result[3].mirror_type, "git");
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
        let bfsu = result.latencies.iter().find(|l| l.name == "bfsu").unwrap();
        assert_eq!(bfsu.network_ms, Some(96));
    }

    #[test]
    fn test_validate_mirror_name() {
        assert!(validate_mirror_name("sjtu").is_ok());
        assert!(validate_mirror_name("rsproxy-sparse").is_ok());
        assert!(validate_mirror_name("my_mirror").is_ok());
        assert!(validate_mirror_name("").is_err());
        assert!(validate_mirror_name("bad name").is_err());
        assert!(validate_mirror_name("rm -rf /").is_err());
    }

    #[test]
    fn test_validate_best_mode() {
        assert!(validate_best_mode("").is_ok());
        assert!(validate_best_mode("git").is_ok());
        assert!(validate_best_mode("sparse").is_ok());
        assert!(validate_best_mode("git-download").is_ok());
        assert!(validate_best_mode("sparse-download").is_ok());
        assert!(validate_best_mode("invalid").is_err());
    }
}

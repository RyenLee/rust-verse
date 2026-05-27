//! Rustup lifecycle management business logic.
//!
//! Handles rustup installation, uninstallation, PATH refresh,
//! and binary detection. Extracted from lib.rs's inline logic.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::infrastructure::db;
use crate::infrastructure::exec::run_command;
use crate::infrastructure::logger;
use crate::infrastructure::system::binary_exists;

/// Internal logic for refreshing process PATH and Rust-related env vars.
pub fn refresh_process_path_inner() -> crate::domain::error::AppResult<String> {
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut added: Vec<String> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use crate::infrastructure::system::{read_system_env_var, read_user_env_var};

        let mut new_path_parts: Vec<String> = Vec::new();
        if let Some(system_path) = read_system_env_var("Path") {
            new_path_parts.push(system_path);
        }
        if let Some(user_path) = read_user_env_var("Path") {
            new_path_parts.push(user_path);
        }

        if !new_path_parts.is_empty() {
            let new_path = new_path_parts.join(";");
            let old_path = std::env::var("PATH").unwrap_or_default();
            if new_path != old_path {
                unsafe {
                    std::env::set_var("PATH", &new_path);
                }
                added.push(format!("PATH updated ({} chars)", new_path.len()));
            }
        }

        for var_name in &["CARGO_HOME", "RUSTUP_HOME"] {
            let current = std::env::var(var_name).ok();
            let from_system = read_system_env_var(var_name);
            let from_user = read_user_env_var(var_name);
            let registry_val = from_user.or(from_system);
            if current != registry_val {
                if let Some(val) = &registry_val {
                    unsafe {
                        std::env::set_var(var_name, val);
                    }
                    added.push(format!("{var_name}={val}"));
                }
            }
        }
    }

    if added.is_empty() {
        Ok("No changes detected.".to_string())
    } else {
        Ok(format!("Updated: {}", added.join(", ")))
    }
}

/// Check whether a binary is functionally available by running `<name> --version`.
pub async fn is_binary_functional(name: &str) -> bool {
    let mut cmd = tokio::process::Command::new(name);
    cmd.arg("--version");
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }
    tokio::time::timeout(std::time::Duration::from_secs(5), cmd.output())
        .await
        .map(|result| result.map(|o| o.status.success()).unwrap_or(false))
        .unwrap_or(false)
}

/// Uninstall rustup via `rustup self uninstall`.
pub async fn uninstall_rustup(
    state: &crate::state::AppState,
) -> crate::domain::error::AppResult<String> {
    let (rustup, _) = db::get_binaries_config(&*state.store);
    if !binary_exists(&rustup) {
        return Err(crate::domain::error::AppError::Command(
            "rustup is not installed".to_string(),
        ));
    }

    let result = run_command(&rustup, &["self", "uninstall", "-y"], 120).await;
    match result {
        Ok(output) => {
            *state.rustup_path.lock().unwrap() = None;
            *state.cargo_path.lock().unwrap() = None;
            Ok(output)
        }
        Err(e) => {
            let err_msg = format!("{e}");
            let is_locked = err_msg.contains("os error 32")
                || err_msg.contains("being used")
                || err_msg.contains("another program");
            let is_access_denied = err_msg.contains("os error 5") || err_msg.contains("拒绝访问");

            if is_locked || is_access_denied {
                #[cfg(target_os = "windows")]
                {
                    let lock_processes = [
                        "cargo",
                        "rustc",
                        "rust-analyzer",
                        "rustfmt",
                        "clippy-driver",
                    ];
                    for proc_name in lock_processes {
                        let _ = tokio::process::Command::new("taskkill")
                            .args(["/F", "/IM", &format!("{proc_name}.exe")])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .creation_flags(0x08000000)
                            .status()
                            .await;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }

                let retry = run_command(&rustup, &["self", "uninstall", "-y"], 120).await;
                match retry {
                    Ok(output) => {
                        *state.rustup_path.lock().unwrap() = None;
                        *state.cargo_path.lock().unwrap() = None;
                        Ok(output)
                    }
                    Err(retry_err) => {
                        #[cfg(target_os = "windows")]
                        if is_access_denied {
                            let elevated = try_elevated_uninstall(&rustup).await;
                            match elevated {
                                Ok(()) => {
                                    *state.rustup_path.lock().unwrap() = None;
                                    *state.cargo_path.lock().unwrap() = None;
                                    return Ok("Elevated uninstall completed.".to_string());
                                }
                                Err(_) => return Err(retry_err),
                            }
                        }
                        Err(retry_err)
                    }
                }
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(target_os = "windows")]
async fn try_elevated_uninstall(rustup: &str) -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join("rustverse-elevated-uninstall");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("failed to create temp dir: {e}"))?;

    let script_path = temp_dir.join("uninstall.ps1");
    let script_content = format!(
        "$rustupPath = '{escaped}'\nStart-Process -FilePath $rustupPath -ArgumentList 'self','uninstall','-y' -Verb RunAs -Wait",
        escaped = rustup.replace("'", "''")
    );
    std::fs::write(&script_path, &script_content)
        .map_err(|e| format!("failed to write script: {e}"))?;

    let output = tokio::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path.to_string_lossy(),
        ])
        .creation_flags(0x08000000)
        .output()
        .await
        .map_err(|e| format!("failed to launch elevated process: {e}"))?;

    let _ = std::fs::remove_file(&script_path);
    let _ = std::fs::remove_dir(&temp_dir);

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("elevated uninstall failed: {stderr}"))
    }
}

/// Variant of `install_rustup` that supports cancellation via `cancel_flag`.
///
/// Used by the background-task-aware `lib.rs::install_rustup` command so the
/// frontend can cancel a running installation.
pub async fn install_rustup_with_cancel(
    app: tauri::AppHandle,
    cancel_flag: Arc<AtomicBool>,
) -> crate::domain::error::AppResult<()> {
    let log = logger::logger();
    log.info("install", "Install rustup requested (cancel-aware)");

    let _ = refresh_process_path_inner();

    let rustup_ok = is_binary_functional("rustup").await;
    let cargo_ok = is_binary_functional("cargo").await;

    if rustup_ok && cargo_ok {
        log.info(
            "install",
            "rustup and cargo are already installed, aborting",
        );
        return Err(crate::domain::error::AppError::Command(
            "rustup and cargo are already installed".to_string(),
        ));
    }

    log.info("install", "Starting rustup installation...");

    let installer_path = crate::infrastructure::installer::ensure_installer(&app).await?;
    crate::infrastructure::installer::execute_installer_with_cancel(
        app,
        &installer_path,
        cancel_flag,
    )
    .await?;

    let _ = refresh_process_path_inner();
    log.info("install", "Rustup installation completed successfully");
    Ok(())
}

/// Determine the WebView2 user data directory path.
pub fn get_webview_data_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let dir = parent.join("webview");
            std::fs::create_dir_all(&dir).ok();
            return dir;
        }
    }
    let dir = PathBuf::from("webview");
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// Determine the database file path.
pub fn get_db_path() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let data_dir = parent.join("data");
            std::fs::create_dir_all(&data_dir).ok();
            return data_dir.join("config.redb");
        }
    }
    let data_dir = PathBuf::from("data");
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("config.redb")
}

/// Migrate database from old flat location to data/ directory.
pub fn migrate_db_to_data_dir() {
    let Ok(exe_path) = std::env::current_exe() else {
        return;
    };
    let Some(exe_dir) = exe_path.parent() else {
        return;
    };

    let old_path = exe_dir.join("config.redb");
    if !old_path.exists() {
        return;
    }

    let data_dir = exe_dir.join("data");
    let new_path = data_dir.join("config.redb");
    if new_path.exists() {
        return;
    }

    std::fs::create_dir_all(&data_dir).ok();
    match std::fs::rename(&old_path, &new_path) {
        Ok(()) => logger::logger().info(
            "migration",
            &format!("Migrated database: {:?} -> {:?}", old_path, new_path),
        ),
        Err(e) => logger::logger().warn(
            "migration",
            &format!("failed to move database to data/ dir: {e}; will use old location"),
        ),
    }
}

/// Try to migrate from legacy config.toml to redb.
pub fn try_migrate_from_toml(db: &redb::Database) {
    let Ok(exe_path) = std::env::current_exe() else {
        return;
    };
    let Some(exe_dir) = exe_path.parent() else {
        return;
    };

    let toml_path = exe_dir.join("config.toml");
    if !toml_path.exists() {
        return;
    }

    match db::migrate_from_toml(db, &toml_path) {
        Ok(true) => {
            let migrated = exe_dir.join("config.toml.migrated");
            let _ = std::fs::rename(&toml_path, &migrated);
            logger::logger().info(
                "migration",
                "Migrated config.toml -> config.redb, renamed to config.toml.migrated",
            );
        }
        Ok(false) => logger::logger().debug(
            "migration",
            "config.toml exists but matches defaults, skipping migration",
        ),
        Err(e) => logger::logger().warn("migration", &format!("config.toml migration failed: {e}")),
    }
}

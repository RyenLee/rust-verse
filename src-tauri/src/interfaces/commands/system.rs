use crate::domain::error::AppResult;
use crate::infrastructure::system::env::binary_exists;
use crate::infrastructure::{installer, logger};
use crate::state::AppState;

/// Internal logic for refreshing process PATH and Rust-related env vars.
pub fn refresh_process_path_inner() -> AppResult<String> {
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut added: Vec<String> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use crate::infrastructure::system::env::{read_system_env_var, read_user_env_var};

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

/// Refresh the current process PATH from the Windows Registry.
#[tauri::command]
pub fn refresh_process_path() -> AppResult<String> {
    refresh_process_path_inner()
}

/// Check whether a binary is functionally available by running `<name> --version`.
/// Returns `true` only if the command executes successfully.
async fn is_binary_functional(name: &str) -> bool {
    let mut cmd = tokio::process::Command::new(name);
    cmd.arg("--version");
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    tokio::time::timeout(std::time::Duration::from_secs(5), cmd.output())
        .await
        .map(|result| result.map(|o| o.status.success()).unwrap_or(false))
        .unwrap_or(false)
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
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
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

/// Uninstall rustup via `rustup self uninstall`.
#[tauri::command]
pub async fn uninstall_rustup(state: tauri::State<'_, AppState>) -> AppResult<String> {
    use crate::infrastructure::exec::run_command;

    let rustup = state
        .store
        .get_config("binaries.rustup")
        .unwrap_or_else(|| "rustup".to_string());
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
async fn install_rustup_windows(app: tauri::AppHandle) -> AppResult<()> {
    let installer_path = installer::ensure_installer(&app).await?;
    installer::execute_installer(app, &installer_path).await?;
    let _ = refresh_process_path_inner();
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn install_rustup_unix(app: tauri::AppHandle) -> AppResult<()> {
    let installer_path = installer::ensure_installer(&app).await?;
    installer::execute_installer(app, &installer_path).await?;
    let _ = refresh_process_path_inner();
    Ok(())
}

/// Install rustup using the official installer with streaming output.
#[tauri::command]
pub async fn install_rustup(app: tauri::AppHandle) -> AppResult<()> {
    let log = logger::logger();
    log.info("install", "Install rustup requested");

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

    #[cfg(target_os = "windows")]
    {
        let result = install_rustup_windows(app).await;
        match &result {
            Ok(()) => log.info("install", "Rustup installation completed successfully"),
            Err(e) => log.error("install", &format!("Rustup installation failed: {e}")),
        }
        result
    }

    #[cfg(not(target_os = "windows"))]
    {
        let result = install_rustup_unix(app).await;
        match &result {
            Ok(()) => log.info("install", "Rustup installation completed successfully"),
            Err(e) => log.error("install", &format!("Rustup installation failed: {e}")),
        }
        result
    }
}

/// Get the log directory path for the frontend.
#[tauri::command]
pub fn get_log_dir() -> String {
    logger::logger().log_dir().to_string_lossy().to_string()
}

/// Write a log message from the frontend to the backend log file.
#[tauri::command]
pub fn frontend_log(level: String, module: String, message: String) {
    logger::logger().log_from_str(&level, &module, &message);
}

/// Request cancellation of the currently running background task.
#[tauri::command]
pub fn cancel_background_task(state: tauri::State<'_, AppState>) -> AppResult<()> {
    state
        .task_state
        .cancel_flag
        .store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Check whether a long-running background task is currently executing.
#[tauri::command]
pub fn is_background_task_running(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    let running = *state.task_state.running.lock().unwrap();
    Ok(running)
}

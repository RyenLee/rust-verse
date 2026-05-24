mod commands;
mod config;
mod db;
mod error;
mod installer;
mod logger;
mod state;
mod system;
mod utils;

use std::path::PathBuf;

use commands::component::{add_component, list_components, remove_component};
use commands::env_check::{check_env, get_versions};
use commands::env_var::{
    delete_env_var_meta, get_env_var, list_env_vars, remove_env_var, set_env_var,
    update_env_var_meta,
};
use commands::locale::{LocaleScanState, get_locale, list_available_locales, set_locale};
use commands::mirror::{check_crm_installed, crm_best, crm_current, crm_default, crm_list, crm_test, crm_use, crm_version, install_crm};
use commands::override_cmd::{get_override, list_overrides, remove_override, set_override};
use commands::persist::{
    is_env_var_persisted, list_persisted_env_vars, persist_env_var, remove_persisted_env_var,
};
use commands::plugin::{install_plugin, list_cargo_plugins, search_plugins, uninstall_plugin};
use commands::target::{add_target, list_targets, remove_target};
use commands::toolchain::{
    install_toolchain, list_toolchains, set_default_toolchain, uninstall_toolchain,
};
use commands::update::{check_update, update_all, update_rustup};
use config::get_config;
use state::AppState;
use system::env::binary_exists;
use tauri::{WebviewUrl, WebviewWindowBuilder};

/// Internal logic for refreshing process PATH and Rust-related env vars.
fn refresh_process_path_inner() -> crate::error::AppResult<String> {
    #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
    let mut added: Vec<String> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use system::env::{read_system_env_var, read_user_env_var};

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
fn refresh_process_path() -> crate::error::AppResult<String> {
    refresh_process_path_inner()
}

/// Uninstall rustup via `rustup self uninstall`.
#[tauri::command]
async fn uninstall_rustup(state: tauri::State<'_, AppState>) -> crate::error::AppResult<String> {
    use crate::utils::exec::run_command;

    let (rustup, _) = db::get_binaries_config(&state.db);
    if !binary_exists(&rustup) {
        return Err(crate::error::AppError::Command(
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
                            .creation_flags(0x08000000) // CREATE_NO_WINDOW
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
    // Write the PowerShell command to a temporary script file to avoid
    // command injection via string interpolation. The rustup path is passed
    // as a script variable rather than embedded in the command string.
    let temp_dir = std::env::temp_dir().join("rustverse-elevated-uninstall");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("failed to create temp dir: {e}"))?;

    let script_path = temp_dir.join("uninstall.ps1");
    let script_content = format!(
        "$rustupPath = '{escaped}'\nStart-Process -FilePath $rustupPath -ArgumentList 'self','uninstall','-y' -Verb RunAs -Wait",
        escaped = rustup.replace("'", "''")
    );
    std::fs::write(&script_path, &script_content)
        .map_err(|e| format!("failed to write script: {e}"))?;

    // IMPORTANT: Do NOT use -NonInteractive here. PowerShell in non-interactive
    // mode conflicts with -Verb RunAs — the UAC dialog may fail to appear,
    // causing PowerShell to hang indefinitely waiting for a prompt the user
    // never sees. -NoProfile alone is safe (skips profile load, ~fast startup).
    // Using -ExecutionPolicy Bypass to avoid script execution policy blocking.
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

    // Clean up the temporary script
    let _ = std::fs::remove_file(&script_path);
    let _ = std::fs::remove_dir(&temp_dir);

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("elevated uninstall failed: {stderr}"))
    }
}

/// Install rustup using the official installer with streaming output.
#[tauri::command]
async fn install_rustup(app: tauri::AppHandle) -> crate::error::AppResult<()> {
    let log = logger::logger();
    log.info("install", "Install rustup requested");

    // Refresh PATH first to detect any partially-completed installations
    let _ = refresh_process_path_inner();

    // Block installation only when both rustup and cargo are *functionally* available.
    // Using `binary_exists` is insufficient after uninstall because residual files
    // may remain on disk (e.g. ~/.cargo/bin/rustup.exe) while the toolchain is
    // actually broken. We verify by running `--version` instead.
    let rustup_ok = is_binary_functional("rustup").await;
    let cargo_ok = is_binary_functional("cargo").await;

    if rustup_ok && cargo_ok {
        log.info("install", "rustup and cargo are already installed, aborting");
        return Err(crate::error::AppError::Command(
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
async fn install_rustup_windows(app: tauri::AppHandle) -> crate::error::AppResult<()> {
    // Ensure installer is available (cached or downloaded) and verified
    let installer_path = installer::ensure_installer(&app).await?;

    // Execute the installer
    installer::execute_installer(app, &installer_path).await?;

    // Refresh process PATH so that newly installed rustup/cargo can be found
    let _ = refresh_process_path_inner();

    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn install_rustup_unix(app: tauri::AppHandle) -> crate::error::AppResult<()> {
    // Ensure installer is available (cached or downloaded) and verified
    let installer_path = installer::ensure_installer(&app).await?;

    // Execute the installer
    installer::execute_installer(app, &installer_path).await?;

    // Refresh process PATH so that newly installed rustup/cargo can be found
    let _ = refresh_process_path_inner();

    Ok(())
}

/// Get the WebView2 user data directory path.
fn get_webview_data_dir() -> PathBuf {
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
fn get_db_path() -> PathBuf {
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

fn migrate_db_to_data_dir() {
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
        Ok(()) => eprintln!("Migrated database: {:?} -> {:?}", old_path, new_path),
        Err(e) => {
            eprintln!("Warning: failed to move database to data/ dir: {e}; will use old location")
        }
    }
}

fn try_migrate_from_toml(db: &redb::Database) {
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
            eprintln!("Migrated config.toml -> config.redb, renamed to config.toml.migrated");
        }
        Ok(false) => eprintln!("config.toml exists but matches defaults, skipping migration"),
        Err(e) => eprintln!("Warning: config.toml migration failed: {e}"),
    }
}

/// Get the log directory path for the frontend.
#[tauri::command]
fn get_log_dir() -> String {
    logger::logger().log_dir().to_string_lossy().to_string()
}

/// Write a log message from the frontend to the backend log file.
#[tauri::command]
fn frontend_log(level: String, module: String, message: String) {
    logger::logger().log(&level, &module, &message);
}

macro_rules! dual_log {
    ($log:expr, $level:expr, $module:expr, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        let msg = format!($fmt $(, $arg)*);
        eprintln!("[{}] {}", $module, msg);
        $log.log($level, $module, &msg);
    }};
    ($log:expr, $level:expr, $module:expr, $fmt:expr) => {{
        eprintln!("[{}] {}", $module, $fmt);
        $log.log($level, $module, $fmt);
    }};
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    eprintln!("=== RustVerse v{} startup ===", env!("CARGO_PKG_VERSION"));

    let log = logger::logger();
    eprintln!("[startup] Logger initialized at {:?}", log.log_dir());

    eprintln!("[startup] Running DB migration check...");
    migrate_db_to_data_dir();

    let db_path = get_db_path();
    dual_log!(log, "INFO", "startup", "Database path: {:?}", db_path);
    let db = db::open_or_create(&db_path).unwrap_or_else(|e| {
        dual_log!(
            log,
            "ERROR",
            "startup",
            "Failed to open database: {e}, falling back to in-memory"
        );
        redb::Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())
            .expect("in-memory database should always succeed")
    });
    dual_log!(log, "INFO", "startup", "Database opened successfully");

    try_migrate_from_toml(&db);

    let app_state = AppState::new(db);
    let locale_scan_state = LocaleScanState::new();
    let webview_data_dir = get_webview_data_dir();

    dual_log!(
        log,
        "INFO",
        "startup",
        "Webview data dir: {:?}",
        webview_data_dir
    );
    dual_log!(log, "INFO", "startup", "Log directory: {:?}", log.log_dir());

    let log_for_setup = log;
    eprintln!("[startup] Building Tauri application...");

    tauri::Builder::default()
        .setup(move |app| {
            eprintln!("[setup] Tauri setup started");
            log_for_setup.info("setup", "Tauri setup started");

            let main_window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("RustVerse")
                    .inner_size(1024.0, 720.0)
                    .min_inner_size(768.0, 480.0)
                    .resizable(true)
                    .data_directory(webview_data_dir)
                    .build();

            match main_window {
                Ok(_) => {
                    eprintln!("[setup] Main window created successfully");
                    log_for_setup.info("setup", "Main window created successfully")
                }
                Err(e) => {
                    eprintln!("[setup] ERROR: Failed to create main window: {e}");
                    log_for_setup.error("setup", &format!("Failed to create main window: {e}"))
                }
            }

            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
            }
            #[cfg(debug_assertions)]
            {
                // DevTools auto-open disabled — use pnpm tauri dev for debugging
                // let window = tauri::Manager::get_webview_window(app, "main").unwrap();
                // window.open_devtools();
            }
            eprintln!("[setup] Tauri setup completed");
            log_for_setup.info("setup", "Tauri setup completed");
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_prevent_default::init())
        .invoke_handler(tauri::generate_handler![
            check_env,
            refresh_process_path,
            get_log_dir,
            frontend_log,
            uninstall_rustup,
            install_rustup,
            get_versions,
            get_config,
            list_toolchains,
            install_toolchain,
            uninstall_toolchain,
            set_default_toolchain,
            get_override,
            set_override,
            remove_override,
            list_overrides,
            list_components,
            add_component,
            remove_component,
            list_targets,
            add_target,
            remove_target,
            check_update,
            update_all,
            update_rustup,
            list_cargo_plugins,
            search_plugins,
            install_plugin,
            uninstall_plugin,
            list_env_vars,
            get_env_var,
            set_env_var,
            remove_env_var,
            update_env_var_meta,
            delete_env_var_meta,
            persist_env_var,
            remove_persisted_env_var,
            is_env_var_persisted,
            list_persisted_env_vars,
            get_locale,
            set_locale,
            list_available_locales,
            check_crm_installed,
            install_crm,
            crm_list,
            crm_current,
            crm_version,
            crm_use,
            crm_best,
            crm_default,
            crm_test,
        ])
        .manage(app_state)
        .manage(locale_scan_state)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

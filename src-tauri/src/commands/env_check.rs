use crate::db;
use crate::state::AppState;
use crate::system::env::find_binary;
use crate::utils::exec::run_command;
use tauri::Emitter;

/// Environment check result for the frontend.
#[derive(serde::Serialize, Clone)]
pub struct EnvCheck {
    pub rustup_installed: bool,
    pub cargo_installed: bool,
    pub rustup_error: Option<String>,
    pub cargo_error: Option<String>,
    pub cargo_home: Option<String>,
    pub rustup_home: Option<String>,
}

/// Emit a log event so the frontend can display real-time progress.
fn emit_log<R: tauri::Runtime>(app: &tauri::AppHandle<R>, msg: &str) {
    let _ = app.emit("env-check-log", msg);
}

/// Check rustup by running `rustup --version` with a 10s timeout.
async fn check_rustup(app: &tauri::AppHandle, binary_name: &str) -> (bool, Option<String>) {
    emit_log(app, &format!("Searching for {}...", binary_name));

    let rustup_full_path = match find_binary(binary_name) {
        Ok(path) => {
            // Validate that the found binary matches the expected name
            // to prevent executing a hijacked binary with a different name
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem != binary_name {
                    emit_log(app, &format!(
                        "Found binary at {} but stem '{}' does not match expected '{}', skipping",
                        path.display(), stem, binary_name
                    ));
                    return (false, Some(format!("binary name mismatch: expected '{binary_name}', found '{stem}'")));
                }
            }
            emit_log(app, &format!("Found {} at {}", binary_name, path.display()));
            path
        }
        Err(e) => {
            emit_log(app, &format!("{} not found: {}", binary_name, e));
            return (false, None);
        }
    };

    emit_log(app, &format!("Running {} --version...", binary_name));

    let mut cmd = tokio::process::Command::new(&rustup_full_path);
    cmd.arg("--version");
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let version_result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        cmd.output(),
    )
    .await;

    match version_result {
        Ok(Ok(output)) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            emit_log(
                app,
                &format!("{} version: {}", binary_name, version.lines().next().unwrap_or(&version)),
            );

            // Ensure dir is in PATH for subsequent commands
            if let Some(parent) = rustup_full_path.parent() {
                let dir = parent.to_string_lossy().to_string();
                let current_path = std::env::var("PATH").unwrap_or_default();
                let separator = if cfg!(windows) { ";" } else { ":" };
                let in_path = current_path
                    .split(separator)
                    .any(|p| p.eq_ignore_ascii_case(&dir));
                if !in_path {
                    let new_path = format!("{}{}{}", dir, separator, current_path);
                    unsafe {
                        std::env::set_var("PATH", &new_path);
                    }
                    emit_log(app, &format!("Added {} to PATH", dir));
                }
            }
            (true, None)
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let error_msg = if stderr.is_empty() {
                format!("{} exited with code {:?}", binary_name, output.status.code())
            } else {
                stderr.chars().take(200).collect()
            };
            emit_log(app, &format!("{} failed: {}", binary_name, error_msg));
            (false, Some(error_msg))
        }
        Ok(Err(e)) => {
            let msg = format!("failed to execute {}: {}", binary_name, e);
            emit_log(app, &msg);
            (false, Some(msg))
        }
        Err(_) => {
            let msg = format!("{} --version timed out (10s)", binary_name);
            emit_log(app, &msg);
            (false, Some(msg))
        }
    }
}

/// Check if the Rust toolchain environment is available.
///
/// Verifies both rustup AND cargo by running their respective --version commands.
/// Emits `env-check-log` events for real-time progress display.
#[tauri::command]
pub async fn check_env(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<EnvCheck, String> {
    let (rustup, cargo) = db::get_binaries_config(&state.db);

    emit_log(&app, "Environment check started...");

    let (cargo_home, rustup_home) = crate::system::env::resolve_rust_homes();

    // Step 1: Check rustup
    emit_log(&app, "--- Checking rustup ---");
    let (rustup_installed, rustup_error) = check_rustup(&app, &rustup).await;

    // Step 2: Check cargo
    emit_log(&app, "--- Checking cargo ---");
    let (cargo_installed, cargo_error) = check_rustup(&app, &cargo).await;

    let both_ok = rustup_installed && cargo_installed;
    if both_ok {
        emit_log(&app, "Environment check passed: both rustup and cargo are available.");
    } else {
        emit_log(
            &app,
            &format!(
                "Environment check failed: rustup={}, cargo={}",
                rustup_installed, cargo_installed
            ),
        );
    }

    Ok(EnvCheck {
        rustup_installed,
        cargo_installed,
        rustup_error,
        cargo_error,
        cargo_home,
        rustup_home,
    })
}

/// Get rustup and cargo version strings.
#[derive(serde::Serialize)]
pub struct VersionInfo {
    pub rustup_version: Option<String>,
    pub cargo_version: Option<String>,
}

#[tauri::command]
pub async fn get_versions(state: tauri::State<'_, AppState>) -> Result<VersionInfo, String> {
    let (rustup, cargo) = db::get_binaries_config(&state.db);

    let rustup_version = run_command(&rustup, &["--version"], 30)
        .await
        .ok()
        .map(|s| s.lines().next().unwrap_or(&s).to_string());
    let cargo_version = run_command(&cargo, &["--version"], 30)
        .await
        .ok()
        .map(|s| s.lines().next().unwrap_or(&s).to_string());
    Ok(VersionInfo {
        rustup_version,
        cargo_version,
    })
}
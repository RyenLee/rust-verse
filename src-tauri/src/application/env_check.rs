//! Environment check business logic.
//!
//! Core env check routines separated from Tauri command concerns.

use crate::domain::constants::env_check_event;
use crate::infrastructure::logger;
use crate::infrastructure::system::find_binary;
use tauri::Emitter;

/// Emit a log event to both the frontend and the log file.
pub fn emit_log<R: tauri::Runtime>(app: &tauri::AppHandle<R>, msg: &str) {
    let _ = app.emit(env_check_event::LOG_EVENT, msg);
    logger::logger().info(env_check_event::LOG_MODULE, msg);
}

/// Check rustup/cargo by running `<binary> --version` with a 10s timeout.
pub async fn check_rustup(app: &tauri::AppHandle, binary_name: &str) -> (bool, Option<String>) {
    emit_log(app, &format!("Searching for {}...", binary_name));

    let full_path = match find_binary(binary_name) {
        Ok(path) => {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem != binary_name {
                    emit_log(
                        app,
                        &format!(
                            "Found binary at {} but stem '{}' does not match expected '{}', skipping",
                            path.display(),
                            stem,
                            binary_name
                        ),
                    );
                    return (
                        false,
                        Some(format!(
                            "binary name mismatch: expected '{binary_name}', found '{stem}'"
                        )),
                    );
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

    let mut cmd = tokio::process::Command::new(&full_path);
    cmd.arg("--version");
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }
    let version_result =
        tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output()).await;

    match version_result {
        Ok(Ok(output)) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            emit_log(
                app,
                &format!(
                    "{} version: {}",
                    binary_name,
                    version.lines().next().unwrap_or(&version)
                ),
            );
            if let Some(parent) = full_path.parent() {
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
                format!(
                    "{} exited with code {:?}",
                    binary_name,
                    output.status.code()
                )
            } else {
                stderr.chars().take(200).collect()
            };
            emit_log(app, &format!("{} failed: {}", binary_name, error_msg));
            (false, Some(error_msg))
        }
        Ok(Err(e)) => {
            let raw = e.to_string();
            let msg = if raw.contains("os error 448") || raw.contains("448") {
                let hint = "\
This error is caused by Windows security settings blocking access to the Rust toolchain path.\n\
Possible causes:\n\
  1. Windows Controlled Folder Access is enabled (Windows Security > Virus & threat protection > Manage settings > Controlled folder access)\n\
  2. The D: drive or toolchain path is on a network/virtual drive not trusted by Windows\n\
  3. Windows Defender Application Guard is blocking the path\n\
Suggested fixes:\n\
  - Add the Rust toolchain directory (Installer directory) to Windows Defender exclusions\n\
  - Add this application to Controlled Folder Access allowed apps\n\
  - Move the Rust toolchain to the default user directory (C:\\Users\\<name>\\.cargo)";
                format!(
                    "failed to execute {}: {}\n\nHint: {}\n\nOriginal: {}",
                    binary_name,
                    "Windows security blocked execution (os error 448 - untrusted mount point)",
                    hint,
                    raw
                )
            } else {
                format!("failed to execute {}: {}", binary_name, raw)
            };
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
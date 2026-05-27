use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::domain::error::{AppError, AppResult};

/// Rust-related environment variables that affect download sources.
/// These are read from the Windows Registry (or process env on other platforms)
/// and injected into rustup/cargo child processes so they use the configured
/// mirror instead of the default `static.rust-lang.org`.
const RUSTUP_MIRROR_VARS: &[&str] = &["RUSTUP_DIST_SERVER", "RUSTUP_UPDATE_ROOT"];

/// Inject `RUSTUP_DIST_SERVER` and `RUSTUP_UPDATE_ROOT` into a child process
/// if they are set in the Windows Registry (user or system level) but not
/// already present in the current process environment.
///
/// This ensures that mirror settings configured via the app's Environment
/// Variables page are respected by `rustup toolchain install` and similar
/// commands, even though the current process may not have been restarted
/// after the user set those variables.
fn inject_rustup_mirror_env(cmd: &mut Command) {
    for &var_name in RUSTUP_MIRROR_VARS {
        // If already set in the current process env, the child will inherit it.
        if std::env::var(var_name).is_ok() {
            continue;
        }
        // Otherwise, try to read from the Windows Registry.
        #[cfg(target_os = "windows")]
        {
            let value = crate::infrastructure::system::env::read_user_env_var(var_name)
                .or_else(|| crate::infrastructure::system::env::read_system_env_var(var_name));
            if let Some(val) = value {
                cmd.env(var_name, &val);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            // On non-Windows platforms, process env is the only source.
            let _ = var_name;
        }
    }
}

/// Execute a command with `LC_ALL=C` and capture its combined output.
///
/// Sets `LC_ALL=C` to ensure consistent output format across locales,
/// which is critical for reliable parsing of `rustup`/`cargo` output.
///
/// If the command does not complete within `timeout_secs`, `AppError::Timeout` is returned.
pub async fn run_command(bin: &str, args: &[&str], timeout_secs: u64) -> AppResult<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .env("LC_ALL", "C")
        .env("CARGO_HTTP_MULTIPLEXING", "false")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    // Apply proxy settings from the resolver (cached DB read).
    let proxy_config = crate::infrastructure::proxy::get_proxy_config();
    crate::infrastructure::proxy::apply_proxy_env(&mut cmd, &proxy_config);

    // Inject RUSTUP_DIST_SERVER / RUSTUP_UPDATE_ROOT from registry if set.
    inject_rustup_mirror_env(&mut cmd);

    let result =
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let code = output.status.code().unwrap_or(-1);
                Err(AppError::Command(format!(
                    "'{bin}' exited with code {code}: {stderr}"
                )))
            }
        }
        Ok(Err(e)) => {
            let raw = e.to_string();
            // os error 448 on Windows: "Cannot traverse this path because it contains an untrusted mount point."
            // This is typically caused by Windows Controlled Folder Access or Windows Defender Application Guard.
            if raw.contains("os error 448") || raw.contains("448") {
                Err(AppError::Command(format!(
                    "failed to execute '{bin}': Windows security blocked execution (os error 448 - untrusted mount point). \
                    This is usually caused by Windows Controlled Folder Access. Try adding the Rust toolchain \
                    directory (D:\\Rust) or this application to Windows Defender exclusions, or add the app to \
                    Controlled Folder Access allowed apps. Original error: {raw}"
                )))
            } else {
                Err(AppError::Command(format!(
                    "failed to execute '{bin}': {raw}"
                )))
            }
        }
        Err(_) => Err(AppError::Timeout(timeout_secs)),
    }
}

/// Execute a command with a timeout (in seconds).
///
/// Delegates to `run_command` with the specified timeout.
/// Kept for backward compatibility with callers that explicitly set timeout.
pub async fn run_command_with_timeout(
    bin: &str,
    args: &[&str],
    timeout_secs: u64,
) -> AppResult<String> {
    run_command(bin, args, timeout_secs).await
}

/// Execute a command with a timeout, allowing specific exit codes as success.
///
/// Some commands like `rustup check` use non-zero exit codes to indicate
/// specific states (e.g., exit code 100 means "updates available").
/// This function treats the specified exit codes as successful.
pub async fn run_command_with_timeout_allow_codes(
    bin: &str,
    args: &[&str],
    timeout_secs: u64,
    allowed_codes: &[i32],
) -> AppResult<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .env("LC_ALL", "C")
        .env("CARGO_HTTP_MULTIPLEXING", "false")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let proxy_config = crate::infrastructure::proxy::get_proxy_config();
    crate::infrastructure::proxy::apply_proxy_env(&mut cmd, &proxy_config);
    inject_rustup_mirror_env(&mut cmd);

    let result =
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let code = output.status.code().unwrap_or(-1);
            if output.status.success() || allowed_codes.contains(&code) {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                Err(AppError::Command(format!(
                    "'{bin}' exited with code {code}: {stderr}"
                )))
            }
        }
        Ok(Err(e)) => {
            let raw = e.to_string();
            if raw.contains("os error 448") || raw.contains("448") {
                Err(AppError::Command(format!(
                    "failed to execute '{bin}': Windows security blocked execution (os error 448 - untrusted mount point). \
                    This is usually caused by Windows Controlled Folder Access. Try adding the Rust toolchain \
                    directory (D:\\Rust) or this application to Windows Defender exclusions, or add the app to \
                    Controlled Folder Access allowed apps. Original error: {raw}"
                )))
            } else {
                Err(AppError::Command(format!(
                    "failed to execute '{bin}': {raw}"
                )))
            }
        }
        Err(_) => Err(AppError::Timeout(timeout_secs)),
    }
}

/// Read lines from a child process output stream and emit them as Tauri events.
/// Also writes each line to the log file for persistent logging.
fn spawn_line_reader(
    app: AppHandle,
    stream: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    event_name: String,
) {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit(&event_name, &line);
            crate::infrastructure::logger::logger()
                .info("stream", &format!("[{}] {}", event_name, line));
        }
    });
}

// ── Cancel-aware streaming variant ────────────────────────────────────────

/// Like `run_command_with_streaming`, but polls `cancel_flag` every 500ms
/// and kills the child process if cancellation is requested.
///
/// This variant is used by long-running background tasks (e.g., `install_rustup`)
/// that need to support user-initiated cancellation from the frontend.
pub async fn run_command_with_cancel(
    app: AppHandle,
    command: &str,
    args: &[&str],
    locale_key: &str,
    log_event: &str,
    finished_event: &str,
    timeout_secs: u64,
    cancel_flag: Arc<AtomicBool>,
) -> AppResult<()> {
    let mut child_cmd = Command::new(command);
    child_cmd
        .args(args)
        .env("LC_ALL", locale_key)
        .env("CARGO_HTTP_MULTIPLEXING", "false")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        child_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    // Apply proxy settings.
    let proxy_config = crate::infrastructure::proxy::get_proxy_config();
    crate::infrastructure::proxy::apply_proxy_env(&mut child_cmd, &proxy_config);

    // Inject RUSTUP_DIST_SERVER / RUSTUP_UPDATE_ROOT from registry if set.
    inject_rustup_mirror_env(&mut child_cmd);

    let mut child = child_cmd
        .spawn()
        .map_err(|e| AppError::Command(format!("failed to spawn '{command}': {e}")))?;

    if let Some(stderr) = child.stderr.take() {
        spawn_line_reader(app.clone(), stderr, log_event.to_string());
    }
    if let Some(stdout) = child.stdout.take() {
        spawn_line_reader(app.clone(), stdout, log_event.to_string());
    }

    let cancel_flag_clone = cancel_flag.clone();
    let cancel_future = async move {
        loop {
            if cancel_flag_clone.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    };

    let wait_fut = child.wait();
    let timeout_fut = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs));

    tokio::select! {
        // Cancel was requested — kill the process
        _ = cancel_future => {
            let _ = child.kill().await;
            let _ = app.emit(finished_event, ());
            Err(AppError::Command("Task cancelled by user".to_string()))
        }

        // Process exited normally
        status_result = wait_fut => {
            match status_result {
                Ok(status) => {
                    let _ = app.emit(finished_event, ());
                    if status.success() {
                        Ok(())
                    } else {
                        let code = status.code().unwrap_or(-1);
                        Err(AppError::Command(format!(
                            "command failed with exit code {code}"
                        )))
                    }
                }
                Err(e) => {
                    let _ = app.emit(finished_event, ());
                    Err(AppError::Command(format!(
                        "failed to wait for process: {e}"
                    )))
                }
            }
        }

        // Timeout elapsed
        _ = timeout_fut => {
            let _ = child.kill().await;
            let _ = app.emit(finished_event, ());
            Err(AppError::Timeout(timeout_secs))
        }
    }
}

/// Execute a command in a specific working directory.
///
/// Used by `rustup override set/unset` which need to run in the target directory.
///
/// If the command does not complete within `timeout_secs`, `AppError::Timeout` is returned.
pub async fn run_command_with_cwd(
    bin: &str,
    args: &[&str],
    cwd: &str,
    timeout_secs: u64,
) -> AppResult<String> {
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .env("LC_ALL", "C")
        .env("CARGO_HTTP_MULTIPLEXING", "false")
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    // Apply proxy settings.
    let proxy_config = crate::infrastructure::proxy::get_proxy_config();
    crate::infrastructure::proxy::apply_proxy_env(&mut cmd, &proxy_config);

    // Inject RUSTUP_DIST_SERVER / RUSTUP_UPDATE_ROOT from registry if set.
    inject_rustup_mirror_env(&mut cmd);

    let result =
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let code = output.status.code().unwrap_or(-1);
                Err(AppError::Command(format!(
                    "'{bin}' exited with code {code}: {stderr}"
                )))
            }
        }
        Ok(Err(e)) => Err(AppError::Command(format!(
            "failed to execute '{bin}' in '{cwd}': {e}"
        ))),
        Err(_) => Err(AppError::Timeout(timeout_secs)),
    }
}

/// Like `run_command_with_cancel`, but with automatic retry on failure.
pub async fn run_command_with_cancel_retry(
    app: AppHandle,
    command: &str,
    args: &[&str],
    locale_key: &str,
    log_event: &str,
    finished_event: &str,
    max_retries: u32,
    retry_delay_ms: u64,
    timeout_secs: u64,
    cancel_flag: Arc<AtomicBool>,
) -> AppResult<()> {
    let mut attempt: u32 = 0;

    loop {
        // Check cancellation before each attempt
        if cancel_flag.load(Ordering::SeqCst) {
            return Err(AppError::Command("Task cancelled by user".to_string()));
        }

        attempt += 1;

        if attempt > 1 {
            let retry_msg = format!(
                "Retry attempt {attempt}/{total}...",
                attempt = attempt,
                total = max_retries + 1
            );
            let _ = app.emit(log_event, &retry_msg);
        }

        let result = run_command_with_cancel(
            app.clone(),
            command,
            args,
            locale_key,
            log_event,
            finished_event,
            timeout_secs,
            cancel_flag.clone(),
        )
        .await;

        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                // If cancelled, propagate immediately
                if cancel_flag.load(Ordering::SeqCst) {
                    return Err(AppError::Command("Task cancelled by user".to_string()));
                }

                let remaining = (max_retries + 1).saturating_sub(attempt);
                if remaining == 0 {
                    return Err(e);
                }

                let delay = retry_delay_ms * 2u64.pow(attempt.saturating_sub(2));
                let capped_delay = delay.min(60_000);

                let error_msg = format!(
                    "Update failed on attempt {attempt}/{total}. Retrying in {delay}s...",
                    attempt = attempt,
                    total = max_retries + 1,
                    delay = capped_delay / 1000
                );
                let _ = app.emit(log_event, &error_msg);

                tokio::time::sleep(std::time::Duration::from_millis(capped_delay)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command_success() {
        let result = run_command("cmd", &["/C", "echo", "hello"], 30).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[tokio::test]
    async fn test_run_command_not_found() {
        let result = run_command("nonexistent_binary_xyz_12345", &[], 30).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Command(msg) => assert!(msg.contains("failed to execute")),
            other => panic!("expected Command error, got: {other}"),
        }
    }

    #[tokio::test]
    async fn test_run_command_with_timeout_success() {
        let result = run_command_with_timeout("cmd", &["/C", "echo", "fast"], 5).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fast");
    }

    #[tokio::test]
    async fn test_run_command_exit_nonzero() {
        let result = run_command("cmd", &["/C", "exit", "1"], 30).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Command(msg) => assert!(msg.contains("exited with code")),
            other => panic!("expected Command error, got: {other}"),
        }
    }
}

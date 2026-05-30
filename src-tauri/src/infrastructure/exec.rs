use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::domain::constants::{error_pattern, locale, log_module, rustup_mirror_var};
use crate::domain::error::{AppError, AppResult};

/// Rust-related environment variables that affect download sources.
/// These are read from the Windows Registry (or process env on other platforms)
/// and injected into rustup/cargo child processes so they use the configured
/// mirror instead of the default `static.rust-lang.org`.

/// Inject `RUSTUP_DIST_SERVER` and `RUSTUP_UPDATE_ROOT` into a child process
/// if they are set in the Windows Registry (user or system level) but not
/// already present in the current process environment.
///
/// This ensures that mirror settings configured via the app's Environment
/// Variables page are respected by `rustup toolchain install` and similar
/// commands, even though the current process may not have been restarted
/// after the user set those variables.
fn inject_rustup_mirror_env(cmd: &mut Command) {
    for &var_name in rustup_mirror_var::ALL {
        if std::env::var(var_name).is_ok() {
            continue;
        }
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
            let _ = var_name;
        }
    }
}

/// Resolve a binary name to an absolute path.
///
/// Delegates to `find_binary` which searches:
/// 1. Current process PATH (`which::which`)
/// 2. System-level PATH from Windows Registry
/// 3. `CARGO_HOME/bin`
/// 4. `~/.cargo/bin`
///
/// Falls back to the original name if all lookups fail, so `Command::new`
/// can produce a clear error message.
fn resolve_binary(bin: &str) -> std::path::PathBuf {
    if std::path::Path::new(bin).is_absolute() || bin.contains(std::path::MAIN_SEPARATOR) {
        return std::path::PathBuf::from(bin);
    }

    match crate::infrastructure::system::env::find_binary(bin) {
        Ok(path) => path,
        Err(_) => std::path::PathBuf::from(bin),
    }
}

/// Build a `tokio::process::Command` from a resolved absolute path.
///
/// Use this when the binary path has already been resolved by `resolve_binary`.
/// Does NOT call `resolve_binary` again.
fn init_command_from_path(path: &std::path::Path, args: &[&str], locale: &str) -> Command {
    let mut cmd = Command::new(path);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env(locale::LC_ALL, locale);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let proxy_config = crate::infrastructure::proxy::get_proxy_config();
    crate::infrastructure::proxy::apply_proxy_env(&mut cmd, &proxy_config);
    inject_rustup_mirror_env(&mut cmd);

    cmd
}

/// Execute a command with `LC_ALL=C` and capture its combined output.
///
/// Sets `LC_ALL=C` to ensure consistent output format across locales,
/// which is critical for reliable parsing of `rustup`/`cargo` output.
///
/// If the command does not complete within `timeout_secs`, `AppError::Timeout` is returned.
pub async fn run_command(bin: &str, args: &[&str], timeout_secs: u64) -> AppResult<String> {
    let resolved = resolve_binary(bin);
    let mut cmd = init_command_from_path(&resolved, args, locale::LC_C);

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
            if raw.contains(error_pattern::PROGRAM_NOT_FOUND)
                || raw.contains(error_pattern::FILE_NOT_FOUND)
                || raw.contains(error_pattern::NO_SUCH_FILE)
            {
                Err(AppError::BinaryNotFound(format!(
                    "'{bin}' not found in PATH or ~/.cargo/bin. Please install it first."
                )))
            } else if raw.contains(error_pattern::OS_ERROR_448) || raw.contains(error_pattern::OS_ERROR_448_SHORT) {
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
        Err(elapsed) => Err(AppError::Timeout(format!(
            "command '{bin}' timed out after {timeout_secs}s: {elapsed}"
        ))),
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
    let resolved = resolve_binary(bin);
    let mut cmd = init_command_from_path(&resolved, args, locale::LC_C);

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
            if raw.contains(error_pattern::PROGRAM_NOT_FOUND)
                || raw.contains(error_pattern::FILE_NOT_FOUND)
                || raw.contains(error_pattern::NO_SUCH_FILE)
            {
                Err(AppError::BinaryNotFound(format!(
                    "'{bin}' not found in PATH or ~/.cargo/bin. Please install it first."
                )))
            } else if raw.contains(error_pattern::OS_ERROR_448) || raw.contains(error_pattern::OS_ERROR_448_SHORT) {
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
        Err(elapsed) => Err(AppError::Timeout(format!(
            "command '{bin}' timed out after {timeout_secs}s: {elapsed}"
        ))),
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
                .info(log_module::STREAM, &format!("[{}] {}", event_name, line));
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
    let resolved = resolve_binary(command);
    let mut child_cmd = init_command_from_path(&resolved, args, locale_key);

    let mut child = child_cmd.spawn().map_err(|e| {
        let raw = e.to_string();
        if raw.contains(error_pattern::PROGRAM_NOT_FOUND)
            || raw.contains(error_pattern::FILE_NOT_FOUND)
            || raw.contains(error_pattern::NO_SUCH_FILE)
        {
            AppError::BinaryNotFound(format!(
                "'{command}' not found in PATH or ~/.cargo/bin. Please install it first."
            ))
        } else {
            AppError::Command(format!("failed to spawn '{command}': {e}"))
        }
    })?;

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
            Err(AppError::Timeout(format!(
                "command '{command}' timed out after {timeout_secs}s"
            )))
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
    let resolved = resolve_binary(bin);
    let mut cmd = init_command_from_path(&resolved, args, locale::LC_C);
    cmd.current_dir(cwd);

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
            if raw.contains("program not found")
                || raw.contains("cannot find the file specified")
                || raw.contains("No such file or directory")
            {
                Err(AppError::BinaryNotFound(format!(
                    "'{bin}' not found in PATH or ~/.cargo/bin. Please install it first."
                )))
            } else {
                Err(AppError::Command(format!(
                    "failed to execute '{bin}' in '{cwd}': {e}"
                )))
            }
        }
        Err(elapsed) => Err(AppError::Timeout(format!(
            "command '{bin}' timed out after {timeout_secs}s: {elapsed}"
        ))),
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
            AppError::BinaryNotFound(msg) => assert!(msg.contains("not found")),
            other => panic!("expected BinaryNotFound error, got: {other}"),
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

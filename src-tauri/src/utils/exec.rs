use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::error::{AppError, AppResult};

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
        Ok(Err(e)) => Err(AppError::Command(format!("failed to execute '{bin}': {e}"))),
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

/// Run a command with streaming output forwarded as Tauri events.
///
/// Spawns the process, pipes stdout and stderr through `BufReader`,
/// and emits each line as a `log_event`. When the process exits,
/// emits `finished_event` and returns the result.
///
/// If the process does not complete within `timeout_secs`, it is killed
/// and `AppError::Timeout` is returned.
pub async fn run_command_with_streaming(
    app: AppHandle,
    command: &str,
    args: &[&str],
    locale_key: &str,
    log_event: &str,
    finished_event: &str,
    timeout_secs: u64,
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
    let mut child = child_cmd
        .spawn()
        .map_err(|e| AppError::Command(format!("failed to spawn '{command}': {e}")))?;

    if let Some(stderr) = child.stderr.take() {
        spawn_line_reader(app.clone(), stderr, log_event.to_string());
    }
    if let Some(stdout) = child.stdout.take() {
        spawn_line_reader(app.clone(), stdout, log_event.to_string());
    }

    let result =
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), child.wait()).await;

    match result {
        Ok(Ok(status)) => {
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
        Ok(Err(e)) => {
            let _ = app.emit(finished_event, ());
            Err(AppError::Command(format!(
                "failed to wait for process: {e}"
            )))
        }
        Err(_) => {
            // Timeout elapsed — kill the child process
            let _ = child.kill().await;
            let _ = app.emit(finished_event, ());
            Err(AppError::Timeout(timeout_secs))
        }
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
            // Write streaming output to log file
            crate::logger::logger().info("stream", &format!("[{}] {}", event_name, line));
        }
    });
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

/// Run a streaming command with automatic retry on failure.
///
/// On each failure, emits a `retry-attempt` event with the attempt number and error,
/// then waits with exponential backoff before retrying.
///
/// - `max_retries`: maximum number of retry attempts (0 = no retry, attempt once only)
/// - `retry_delay_ms`: base delay in milliseconds; doubles on each retry (exponential backoff)
pub async fn run_command_with_streaming_retry(
    app: AppHandle,
    command: &str,
    args: &[&str],
    locale_key: &str,
    log_event: &str,
    finished_event: &str,
    max_retries: u32,
    retry_delay_ms: u64,
    timeout_secs: u64,
) -> AppResult<()> {
    let mut attempt: u32 = 0;

    loop {
        attempt += 1;

        if attempt > 1 {
            // Emit retry event so the frontend can show retry status
            let retry_msg = format!(
                "Retry attempt {attempt}/{total}...",
                attempt = attempt,
                total = max_retries + 1
            );
            let _ = app.emit(log_event, &retry_msg);
        }

        let result = run_command_with_streaming(
            app.clone(),
            command,
            args,
            locale_key,
            log_event,
            finished_event,
            timeout_secs,
        )
        .await;

        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                let remaining = (max_retries + 1).saturating_sub(attempt);
                if remaining == 0 {
                    return Err(e);
                }

                // Exponential backoff: base_delay * 2^(attempt-2) for retry attempts
                let delay = retry_delay_ms * 2u64.pow(attempt.saturating_sub(2));
                let capped_delay = delay.min(60_000); // cap at 60 seconds

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

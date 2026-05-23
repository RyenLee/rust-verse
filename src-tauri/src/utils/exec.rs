use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::error::{AppError, AppResult};

/// Execute a command with `LC_ALL=C` and capture its combined output.
///
/// Sets `LC_ALL=C` to ensure consistent output format across locales,
/// which is critical for reliable parsing of `rustup`/`cargo` output.
pub async fn run_command(bin: &str, args: &[&str]) -> AppResult<String> {
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
    let output = cmd
        .output()
        .await
        .map_err(|e| AppError::Command(format!("failed to execute '{bin}': {e}")))?;

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

/// Execute a command with a timeout (in seconds).
///
/// Wraps `run_command` with `tokio::time::timeout`. Returns
/// `AppError::Timeout` if the command does not complete within the limit.
pub async fn run_command_with_timeout(
    bin: &str,
    args: &[&str],
    timeout_secs: u64,
) -> AppResult<String> {
    tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        run_command(bin, args),
    )
    .await
    .map_err(|_| AppError::Timeout(timeout_secs))?
}

/// Run a command with streaming output forwarded as Tauri events.
///
/// Spawns the process, pipes stdout and stderr through `BufReader`,
/// and emits each line as a `log_event`. When the process exits,
/// emits `finished_event` and returns the result.
pub async fn run_command_with_streaming(
    app: AppHandle,
    command: &str,
    args: &[&str],
    locale_key: &str,
    log_event: &str,
    finished_event: &str,
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

    let status = child
        .wait()
        .await
        .map_err(|e| AppError::Command(format!("failed to wait for process: {e}")))?;

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

/// Read lines from a child process output stream and emit them as Tauri events.
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
        }
    });
}

/// Execute a command in a specific working directory.
///
/// Used by `rustup override set/unset` which need to run in the target directory.
pub async fn run_command_with_cwd(bin: &str, args: &[&str], cwd: &str) -> AppResult<String> {
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
    let output = cmd
        .output()
        .await
        .map_err(|e| AppError::Command(format!("failed to execute '{bin}' in '{cwd}': {e}")))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command_success() {
        let result = run_command("cmd", &["/C", "echo", "hello"]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[tokio::test]
    async fn test_run_command_not_found() {
        let result = run_command("nonexistent_binary_xyz_12345", &[]).await;
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
        let result = run_command("cmd", &["/C", "exit", "1"]).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Command(msg) => assert!(msg.contains("exited with code")),
            other => panic!("expected Command error, got: {other}"),
        }
    }
}

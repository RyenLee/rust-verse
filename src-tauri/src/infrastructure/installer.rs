use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tauri::{AppHandle, Emitter};

use crate::domain::constants::{channel, event_name, file_name, installer, url};
use crate::domain::error::{AppError, AppResult};
use crate::infrastructure::logger;

fn install_log(app: &AppHandle, msg: impl AsRef<str>) {
    let msg = msg.as_ref();
    let _ = app.emit(event_name::INSTALL_LOG, msg);
    logger::logger().info("install", msg);
}

const MAX_RETRIES: u32 = 3;

struct InstallerMeta {
    url: String,
    file_name: &'static str,
}

#[cfg(target_os = "windows")]
fn get_installer_meta() -> InstallerMeta {
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    InstallerMeta {
        url: format!("{}{}", url::RUSTUP_WIN, arch),
        file_name: file_name::RUSTUP_INIT_EXE,
    }
}

#[cfg(target_os = "macos")]
fn get_installer_meta() -> InstallerMeta {
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else {
        "x86_64-apple-darwin"
    };
    InstallerMeta {
        url: format!(
            "{}{}/{}",
            url::RUSTUP_DIST,
            arch,
            file_name::RUSTUP_INIT
        ),
        file_name: file_name::RUSTUP_INIT,
    }
}

#[cfg(target_os = "linux")]
fn get_installer_meta() -> InstallerMeta {
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-gnu"
    } else {
        "x86_64-unknown-linux-gnu"
    };
    InstallerMeta {
        url: format!(
            "{}{}/{}",
            url::RUSTUP_DIST,
            arch,
            file_name::RUSTUP_INIT
        ),
        file_name: file_name::RUSTUP_INIT,
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn get_installer_meta() -> InstallerMeta {
    InstallerMeta {
        url: url::RUSTUP_SH.to_string(),
        file_name: file_name::RUSTUP_INIT_SH,
    }
}

fn get_data_dir() -> AppResult<PathBuf> {
    let data_dir = crate::infrastructure::app_paths::app_paths()
        .installer_cache_dir()
        .clone();
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| AppError::Config(format!("failed to create data dir: {e}")))?;
    }
    Ok(data_dir)
}

fn get_cached_installer_path() -> AppResult<PathBuf> {
    let meta = get_installer_meta();
    Ok(get_data_dir()?.join(meta.file_name))
}

async fn download_installer(app: &AppHandle, url_str: &str, dest: &Path) -> AppResult<()> {
    install_log(app, format!("Downloading installer from {}...", url_str));

    let client = crate::infrastructure::http_client::http_client();

    let response = client
        .get(url_str)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("download failed: {e}")))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "download failed with HTTP status: {}",
            response.status()
        )));
    }

    let total_size = response.content_length();
    if let Some(size) = total_size {
        install_log(
            app,
            format!("Download size: {:.1} MB", size as f64 / 1_048_576.0),
        );
    }

    let mut file = std::fs::File::create(dest)
        .map_err(|e| AppError::Network(format!("failed to create file: {e}")))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    let mut last_progress_emit = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Network(format!("download stream error: {e}")))?;
        std::io::Write::write_all(&mut file, &chunk)
            .map_err(|e| AppError::Network(format!("file write error: {e}")))?;
        downloaded += chunk.len() as u64;

        if last_progress_emit.elapsed() >= std::time::Duration::from_millis(500) {
            if let Some(total) = total_size {
                let pct = (downloaded as f64 / total as f64 * 100.0) as u8;
                install_log(
                    app,
                    format!(
                        "Downloading... {}% ({:.1} MB)",
                        pct,
                        downloaded as f64 / 1_048_576.0
                    ),
                );
            } else {
                install_log(
                    app,
                    format!("Downloading... {:.1} MB", downloaded as f64 / 1_048_576.0),
                );
            }
            last_progress_emit = std::time::Instant::now();
        }
    }

    std::io::Write::flush(&mut file)
        .map_err(|e| AppError::Network(format!("file flush error: {e}")))?;

    install_log(
        app,
        format!(
            "Download complete: {:.1} MB",
            downloaded as f64 / 1_048_576.0
        ),
    );

    Ok(())
}

pub fn cleanup_stale_cache() -> AppResult<()> {
    let data_dir = get_data_dir()?;
    if !data_dir.exists() {
        return Ok(());
    }

    let current_file = get_installer_meta().file_name;
    let entries = std::fs::read_dir(&data_dir)
        .map_err(|e| AppError::Config(format!("failed to read data dir: {e}")))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == current_file {
                    continue;
                }
                if name.starts_with("rustup-init") {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
    Ok(())
}

pub async fn ensure_installer(app: &AppHandle) -> AppResult<PathBuf> {
    let meta = get_installer_meta();
    let cached_path = get_cached_installer_path()?;

    let _ = cleanup_stale_cache();

    if cached_path.exists() && cached_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        install_log(app, "Found cached installer.");
        return Ok(cached_path);
    }

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            install_log(
                app,
                format!(
                    "Retrying download (attempt {}/{})...",
                    attempt + 1,
                    MAX_RETRIES
                ),
            );
        }

        match download_installer(app, &meta.url, &cached_path).await {
            Ok(()) => return Ok(cached_path),
            Err(e) => {
                let _ = std::fs::remove_file(&cached_path);

                if attempt < MAX_RETRIES - 1 {
                    install_log(app, format!("Download failed: {}. Retrying...", e));
                    continue;
                } else {
                    let data_dir = get_data_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| "<app_dir>/data".to_string());

                    install_log(app, "All download attempts failed.");
                    install_log(
                        app,
                        format!(
                            "You can manually download the installer from:\n  {}",
                            meta.url
                        ),
                    );
                    install_log(
                        app,
                        format!(
                            "Place the file as: {}{}{}",
                            data_dir,
                            std::path::MAIN_SEPARATOR,
                            meta.file_name
                        ),
                    );
                    install_log(app, "Then click \"Retry\" to continue installation.");

                    return Err(AppError::Network(format!(
                        "download failed after {} retries: {}\n\nManual install: download from {}\nand save as {}{}{}",
                        MAX_RETRIES,
                        e,
                        meta.url,
                        data_dir,
                        std::path::MAIN_SEPARATOR,
                        meta.file_name
                    )));
                }
            }
        }
    }

    Err(AppError::Network(
        "failed to obtain installer after all retries".to_string(),
    ))
}

pub async fn execute_installer(app: AppHandle, installer_path: &Path) -> AppResult<()> {
    install_log(&app, "Running installer...");

    #[cfg(target_os = "windows")]
    {
        let mut child = tokio::process::Command::new(installer_path)
            .args([installer::FLAG_YES, installer::FLAG_DEFAULT_TOOLCHAIN, channel::STABLE])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| AppError::Command(format!("failed to spawn installer: {e}")))?;

        let status = child
            .wait()
            .await
            .map_err(|e| AppError::Command(format!("installer wait failed: {e}")))?;

        if status.success() {
            Ok(())
        } else {
            Err(AppError::Command(format!(
                "installer exited with code {}",
                status.code().unwrap_or(-1)
            )))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(installer_path)
                .map_err(|e| AppError::Command(format!("failed to read installer metadata: {e}")))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(installer_path, perms).map_err(|e| {
                AppError::Command(format!("failed to set installer permissions: {e}"))
            })?;
        }

        let is_script = installer_path.extension().is_some_and(|ext| ext == "sh");
        let mut child = if is_script {
            tokio::process::Command::new("sh")
                .args([
                    &installer_path.to_string_lossy(),
                    installer::FLAG_YES,
                    installer::FLAG_DEFAULT_TOOLCHAIN,
                    channel::STABLE,
                ])
                .spawn()
                .map_err(|e| AppError::Command(format!("failed to spawn installer: {e}")))?
        } else {
            tokio::process::Command::new(installer_path)
                .args([installer::FLAG_YES, installer::FLAG_DEFAULT_TOOLCHAIN, channel::STABLE])
                .spawn()
                .map_err(|e| AppError::Command(format!("failed to spawn installer: {e}")))?
        };

        let status = child
            .wait()
            .await
            .map_err(|e| AppError::Command(format!("installer wait failed: {e}")))?;

        if status.success() {
            Ok(())
        } else {
            Err(AppError::Command(format!(
                "installer exited with code {}",
                status.code().unwrap_or(-1)
            )))
        }
    }
}

#[allow(dead_code)]
pub async fn execute_installer_with_cancel(
    app: AppHandle,
    installer_path: &Path,
    cancel_flag: Arc<AtomicBool>,
) -> AppResult<()> {
    install_log(&app, "Running installer...");

    #[cfg(target_os = "windows")]
    {
        crate::infrastructure::exec::run_command_with_cancel(
            app,
            &installer_path.to_string_lossy(),
            &[installer::FLAG_YES, installer::FLAG_DEFAULT_TOOLCHAIN, channel::STABLE],
            "C",
            event_name::INSTALL_LOG,
            event_name::INSTALL_FINISHED,
            600,
            cancel_flag,
        )
        .await
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(installer_path)
                .map_err(|e| AppError::Command(format!("failed to read installer metadata: {e}")))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(installer_path, perms).map_err(|e| {
                AppError::Command(format!("failed to set installer permissions: {e}"))
            })?;
        }

        let is_script = installer_path.extension().is_some_and(|ext| ext == "sh");

        if is_script {
            crate::infrastructure::exec::run_command_with_cancel(
                app,
                "sh",
                &[
                    &installer_path.to_string_lossy(),
                    installer::FLAG_YES,
                    installer::FLAG_DEFAULT_TOOLCHAIN,
                    channel::STABLE,
                ],
                "C",
                event_name::INSTALL_LOG,
                event_name::INSTALL_FINISHED,
                600,
                cancel_flag,
            )
            .await
        } else {
            crate::infrastructure::exec::run_command_with_cancel(
                app,
                &installer_path.to_string_lossy(),
                &[installer::FLAG_YES, installer::FLAG_DEFAULT_TOOLCHAIN, channel::STABLE],
                "C",
                event_name::INSTALL_LOG,
                event_name::INSTALL_FINISHED,
                600,
                cancel_flag,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_data_dir() {
        let result = get_data_dir();
        assert!(result.is_ok());
        let dir = result.unwrap();
        assert!(dir.to_string_lossy().ends_with("data"));
    }

    #[test]
    fn test_get_cached_installer_path() {
        let result = get_cached_installer_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        #[cfg(target_os = "windows")]
        assert!(path.to_string_lossy().contains("rustup-init.exe"));
        #[cfg(not(target_os = "windows"))]
        assert!(path.to_string_lossy().contains("rustup-init"));
    }

    #[test]
    fn test_max_retries_value() {
        assert_eq!(MAX_RETRIES, 3);
    }
}
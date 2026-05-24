//! Installer module: download, cache, verify, and execute rustup installer.
//!
//! Flow:
//! 1. Check if cached installer exists in `[exe_dir]/data/`
//! 2. If cached, verify SHA256 integrity
//! 3. If no cache or integrity fails, download with progress and save to cache
//! 4. Verify downloaded file integrity
//! 5. If verification fails, delete and retry (up to MAX_RETRIES)
//! 6. Execute the installer

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::error::{AppError, AppResult};

/// Maximum download+verify retry attempts.
const MAX_RETRIES: u32 = 3;

/// Download timeout in seconds.
const DOWNLOAD_TIMEOUT_SECS: u64 = 300;

// ── Platform-specific installer metadata ──────────────────────────────────

/// Installer metadata for the current platform.
struct InstallerMeta {
    /// URL to download the installer from.
    url: String,
    /// URL to download the SHA256 hash file from.
    hash_url: String,
    /// File name for the cached installer.
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
        url: format!("https://win.rustup.rs/{}", arch),
        hash_url: format!("https://win.rustup.rs/{}.sha256", arch),
        file_name: "rustup-init.exe",
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
            "https://static.rust-lang.org/rustup/dist/{}/rustup-init",
            arch
        ),
        hash_url: format!(
            "https://static.rust-lang.org/rustup/dist/{}/rustup-init.sha256",
            arch
        ),
        file_name: "rustup-init",
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
            "https://static.rust-lang.org/rustup/dist/{}/rustup-init",
            arch
        ),
        hash_url: format!(
            "https://static.rust-lang.org/rustup/dist/{}/rustup-init.sha256",
            arch
        ),
        file_name: "rustup-init",
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn get_installer_meta() -> InstallerMeta {
    InstallerMeta {
        url: "https://sh.rustup.rs".to_string(),
        hash_url: String::new(),
        file_name: "rustup-init.sh",
    }
}

// ── Cache directory ───────────────────────────────────────────────────────

/// Get the data directory for caching installers: `[exe_dir]/data/`.
fn get_data_dir() -> AppResult<PathBuf> {
    let exe_path = std::env::current_exe()
        .map_err(|e| AppError::Config(format!("failed to get exe path: {e}")))?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| AppError::Config("exe has no parent directory".to_string()))?;
    let data_dir = exe_dir.join("data");
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| AppError::Config(format!("failed to create data dir: {e}")))?;
    }
    Ok(data_dir)
}

/// Get the cached installer path.
fn get_cached_installer_path() -> AppResult<PathBuf> {
    let meta = get_installer_meta();
    Ok(get_data_dir()?.join(meta.file_name))
}

// ── SHA256 verification ───────────────────────────────────────────────────

/// Compute SHA256 hash of a file, returning hex string.
fn compute_file_sha256(path: &Path) -> AppResult<String> {
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path)
        .map_err(|e| AppError::Integrity(format!("failed to open file for hashing: {e}")))?;
    std::io::copy(&mut file, &mut hasher)
        .map_err(|e| AppError::Integrity(format!("failed to read file for hashing: {e}")))?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Fetch the expected SHA256 hash from the official hash URL.
async fn fetch_expected_hash(hash_url: &str) -> AppResult<String> {
    if hash_url.is_empty() {
        // No hash URL available (e.g. sh.rustup.rs script); skip verification
        return Ok(String::new());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::Network(format!("failed to create HTTP client: {e}")))?;

    let response = client
        .get(hash_url)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("failed to fetch hash file: {e}")))?;

    let text = response
        .text()
        .await
        .map_err(|e| AppError::Network(format!("failed to read hash response: {e}")))?;

    // Hash file format: "<hash>  <filename>" or just "<hash>"
    let hash = text.split_whitespace().next().unwrap_or(&text).to_string();
    Ok(hash)
}

/// Verify file integrity by comparing SHA256 hash.
/// Returns Ok(()) if hash matches or no expected hash available.
/// Returns Err if hash mismatch.
fn verify_integrity(file_path: &Path, expected_hash: &str) -> AppResult<()> {
    if expected_hash.is_empty() {
        // No expected hash available, skip verification
        return Ok(());
    }

    let actual_hash = compute_file_sha256(file_path)?;

    if actual_hash.eq_ignore_ascii_case(expected_hash) {
        Ok(())
    } else {
        Err(AppError::Integrity(format!(
            "SHA256 mismatch: expected {}, got {}",
            expected_hash, actual_hash
        )))
    }
}

// ── Download with progress ────────────────────────────────────────────────

/// Download installer to cache directory with streaming progress.
/// Emits `rustup-install-log` events with download progress.
async fn download_installer(app: &AppHandle, url: &str, dest: &Path) -> AppResult<()> {
    let _ = app.emit(
        "rustup-install-log",
        format!("Downloading installer from {}...", url),
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Network(format!("failed to create HTTP client: {e}")))?;

    let response = client
        .get(url)
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
        let _ = app.emit(
            "rustup-install-log",
            format!("Download size: {:.1} MB", size as f64 / 1_048_576.0),
        );
    }

    // Stream the response body to file
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

        // Emit progress at most once per 500ms to avoid flooding
        if last_progress_emit.elapsed() >= std::time::Duration::from_millis(500) {
            if let Some(total) = total_size {
                let pct = (downloaded as f64 / total as f64 * 100.0) as u8;
                let _ = app.emit(
                    "rustup-install-log",
                    format!(
                        "Downloading... {}% ({:.1} MB)",
                        pct,
                        downloaded as f64 / 1_048_576.0
                    ),
                );
            } else {
                let _ = app.emit(
                    "rustup-install-log",
                    format!("Downloading... {:.1} MB", downloaded as f64 / 1_048_576.0),
                );
            }
            last_progress_emit = std::time::Instant::now();
        }
    }

    // Flush file
    std::io::Write::flush(&mut file)
        .map_err(|e| AppError::Network(format!("file flush error: {e}")))?;

    let _ = app.emit(
        "rustup-install-log",
        format!(
            "Download complete: {:.1} MB",
            downloaded as f64 / 1_048_576.0
        ),
    );

    Ok(())
}

// ── Cleanup ───────────────────────────────────────────────────────────────

/// Clean up old/stale installer cache files (except the current platform's installer).
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
                // Keep current platform installer and hash files
                if name == current_file || name.ends_with(".sha256") {
                    continue;
                }
                // Remove old installers
                if name.starts_with("rustup-init") {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
    Ok(())
}

// ── Main entry: ensure installer with cache + verify + retry ──────────────

/// Ensure the installer is available (cached or downloaded) and verified.
///
/// This implements the full flow:
/// 1. Check cache → verify integrity → return path if valid
/// 2. Download → verify → return path if valid
/// 3. On integrity failure: delete file → retry download (up to MAX_RETRIES)
///
/// Returns the path to the verified installer.
pub async fn ensure_installer(app: &AppHandle) -> AppResult<PathBuf> {
    let meta = get_installer_meta();
    let cached_path = get_cached_installer_path()?;

    // Clean up stale cache files from other platforms
    let _ = cleanup_stale_cache();

    for attempt in 0..=MAX_RETRIES {
        // Step 1: Check if cached installer exists
        if cached_path.exists() && cached_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            let _ = app.emit(
                "rustup-install-log",
                "Found cached installer, verifying integrity...",
            );

            // Step 2: Fetch expected hash
            let expected_hash = match fetch_expected_hash(&meta.hash_url).await {
                Ok(h) => h,
                Err(e) => {
                    let _ = app.emit(
                        "rustup-install-log",
                        format!("Warning: could not fetch hash for verification: {}", e),
                    );
                    // If we can't fetch the hash, trust the cached file on first attempt
                    // but re-download on subsequent attempts
                    if attempt == 0 {
                        let _ = app.emit(
                            "rustup-install-log",
                            "Using cached installer (verification unavailable).",
                        );
                        return Ok(cached_path);
                    }
                    String::new()
                }
            };

            // Step 3: Verify integrity
            match verify_integrity(&cached_path, &expected_hash) {
                Ok(()) => {
                    let _ = app.emit("rustup-install-log", "Integrity verification passed.");
                    return Ok(cached_path);
                }
                Err(e) => {
                    let _ = app.emit(
                        "rustup-install-log",
                        format!("Integrity check failed: {}. Deleting cached file.", e),
                    );
                    let _ = std::fs::remove_file(&cached_path);

                    if attempt >= MAX_RETRIES {
                        return Err(AppError::Integrity(format!(
                            "installer integrity check failed after {} retries: {}",
                            MAX_RETRIES, e
                        )));
                    }

                    let _ = app.emit(
                        "rustup-install-log",
                        format!(
                            "Retrying download (attempt {}/{})...",
                            attempt + 1,
                            MAX_RETRIES
                        ),
                    );
                    continue;
                }
            }
        }

        // Step 4: Download installer
        if attempt > 0 {
            let _ = app.emit(
                "rustup-install-log",
                format!("Retrying download (attempt {}/{})...", attempt, MAX_RETRIES),
            );
        }

        match download_installer(app, &meta.url, &cached_path).await {
            Ok(()) => {}
            Err(e) => {
                // Clean up partial download
                let _ = std::fs::remove_file(&cached_path);

                if attempt >= MAX_RETRIES {
                    return Err(AppError::Network(format!(
                        "download failed after {} retries: {}",
                        MAX_RETRIES, e
                    )));
                }
                let _ = app.emit(
                    "rustup-install-log",
                    format!("Download failed: {}. Retrying...", e),
                );
                continue;
            }
        }

        // Step 5: Verify downloaded file
        let expected_hash = match fetch_expected_hash(&meta.hash_url).await {
            Ok(h) => h,
            Err(e) => {
                let _ = app.emit(
                    "rustup-install-log",
                    format!("Warning: could not fetch hash: {}", e),
                );
                // If hash unavailable, trust the freshly downloaded file
                return Ok(cached_path);
            }
        };

        match verify_integrity(&cached_path, &expected_hash) {
            Ok(()) => {
                let _ = app.emit("rustup-install-log", "Integrity verification passed.");
                return Ok(cached_path);
            }
            Err(e) => {
                let _ = std::fs::remove_file(&cached_path);

                if attempt >= MAX_RETRIES {
                    return Err(AppError::Integrity(format!(
                        "integrity check failed after {} retries: {}",
                        MAX_RETRIES, e
                    )));
                }

                let _ = app.emit(
                    "rustup-install-log",
                    format!(
                        "Integrity check failed: {}. Re-downloading (attempt {}/{})...",
                        e,
                        attempt + 1,
                        MAX_RETRIES
                    ),
                );
                continue;
            }
        }
    }

    // Should not reach here, but just in case
    Err(AppError::Integrity(
        "failed to obtain a valid installer after all retries".to_string(),
    ))
}

/// Execute the rustup installer with streaming output.
pub async fn execute_installer(app: AppHandle, installer_path: &Path) -> AppResult<()> {
    let _ = app.emit("rustup-install-log", "Running installer...");

    #[cfg(target_os = "windows")]
    {
        crate::utils::exec::run_command_with_streaming(
            app,
            &installer_path.to_string_lossy(),
            &["-y", "--default-toolchain", "stable"],
            "C",
            "rustup-install-log",
            "rustup-install-finished",
            600,
        )
        .await
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Make the installer executable on Unix
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

        // For shell scripts (fallback), use `sh` to execute; for binaries, run directly
        let is_script = installer_path.extension().is_some_and(|ext| ext == "sh");

        if is_script {
            crate::utils::exec::run_command_with_streaming(
                app,
                "sh",
                &[
                    &installer_path.to_string_lossy(),
                    "-y",
                    "--default-toolchain",
                    "stable",
                ],
                "C",
                "rustup-install-log",
                "rustup-install-finished",
                600,
            )
            .await
        } else {
            crate::utils::exec::run_command_with_streaming(
                app,
                &installer_path.to_string_lossy(),
                &["-y", "--default-toolchain", "stable"],
                "C",
                "rustup-install-log",
                "rustup-install-finished",
                600,
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
    fn test_compute_sha256_of_nonexistent_file() {
        let result = compute_file_sha256(Path::new("/nonexistent/file.xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_integrity_empty_hash() {
        // Empty expected hash should pass (skip verification)
        let result = verify_integrity(Path::new("/nonexistent/file.xyz"), "");
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_retries_value() {
        assert_eq!(MAX_RETRIES, 3);
    }
}

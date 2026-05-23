use std::path::PathBuf;

use crate::error::{AppError, AppResult};

/// Resolve CARGO_HOME and RUSTUP_HOME from environment variables.
///
/// Priority (highest to lowest):
/// 1. Process environment variables (CARGO_HOME, RUSTUP_HOME)
/// 2. Windows registry (user, then system)
/// 3. Default paths: `~/.cargo` and `~/.rustup`
pub fn resolve_rust_homes() -> (Option<String>, Option<String>) {
    let home_dir = dirs::home_dir();
    let default_cargo = home_dir.as_ref().map(|h| h.join(".cargo"));
    let default_rustup = home_dir.as_ref().map(|h| h.join(".rustup"));

    let cargo_from_env = std::env::var("CARGO_HOME").ok();
    let rustup_from_env = std::env::var("RUSTUP_HOME").ok();

    #[cfg(target_os = "windows")]
    {
        let cargo_from_registry = cargo_from_env.is_none().then(|| {
            read_user_env_var("CARGO_HOME")
                .or_else(|| read_system_env_var("CARGO_HOME"))
        }).flatten();

        let rustup_from_registry = rustup_from_env.is_none().then(|| {
            read_user_env_var("RUSTUP_HOME")
                .or_else(|| read_system_env_var("RUSTUP_HOME"))
        }).flatten();

        let cargo = cargo_from_env
            .or(cargo_from_registry)
            .or_else(|| default_cargo.map(|p| p.to_string_lossy().to_string()));
        let rustup = rustup_from_env
            .or(rustup_from_registry)
            .or_else(|| default_rustup.map(|p| p.to_string_lossy().to_string()));

        return (cargo, rustup);
    }

    #[cfg(not(target_os = "windows"))]
    {
        (
            cargo_from_env.or_else(|| default_cargo.map(|p| p.to_string_lossy().to_string())),
            rustup_from_env.or_else(|| default_rustup.map(|p| p.to_string_lossy().to_string())),
        )
    }
}

/// Find a binary on the system `PATH` by name.
///
/// Searches in this order:
/// 1. Current process `PATH` (via `which` crate — may be stale if system PATH changed)
/// 2. System-level PATH from Windows Registry (only on Windows)
/// 3. `~/.cargo/bin` (default Rust installation location)
/// 4. `CARGO_HOME/bin` if `CARGO_HOME` is set in the process environment or system env
///
/// Returns the first matching path, or `AppError::BinaryNotFound`.
pub fn find_binary(name: &str) -> AppResult<PathBuf> {
    // 1. Check current process PATH
    if let Ok(path) = which::which(name) {
        return Ok(path);
    }

    // 2. Check system-level PATH from Windows Registry
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = find_binary_in_system_path(name) {
            return Ok(path);
        }
    }

    // 3. Check CARGO_HOME/bin if CARGO_HOME is set
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        let bin = PathBuf::from(&cargo_home)
            .join("bin")
            .join(format_bin_name(name));
        if bin.exists() {
            return Ok(bin);
        }
    }

    // 4. Check system CARGO_HOME from registry (Windows)
    #[cfg(target_os = "windows")]
    {
        if let Some(cargo_home) = read_system_env_var("CARGO_HOME") {
            let bin = PathBuf::from(&cargo_home)
                .join("bin")
                .join(format_bin_name(name));
            if bin.exists() {
                return Ok(bin);
            }
        }
    }

    // 5. Check ~/.cargo/bin (most common rustup install location)
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo/bin").join(format_bin_name(name));
        if cargo_bin.exists() {
            return Ok(cargo_bin);
        }
    }

    Err(AppError::BinaryNotFound(name.to_string()))
}

/// Check if a binary exists on the system.
pub fn binary_exists(name: &str) -> bool {
    find_binary(name).is_ok()
}

/// Add `.exe` suffix on Windows.
fn format_bin_name(name: &str) -> String {
    if cfg!(target_os = "windows") && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Read an environment variable from the Windows system registry (HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment).
///
/// This reflects the latest system-level environment variable, even if the
/// current process inherited an older value at startup.
#[cfg(target_os = "windows")]
pub fn read_system_env_var(name: &str) -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let env = hklm
        .open_subkey_with_flags(
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            KEY_READ,
        )
        .ok()?;

    env.get_value(name).ok()
}

/// Read an environment variable from the Windows user registry (HKCU\Environment).
///
/// This reflects the latest user-level environment variable.
#[cfg(target_os = "windows")]
pub fn read_user_env_var(name: &str) -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ).ok()?;
    env.get_value(name).ok()
}

/// Search for a binary in the system-level PATH from the Windows Registry.
///
/// Combines both system (HKLM) and user (HKCU) PATH values, then searches
/// each directory for the binary.
#[cfg(target_os = "windows")]
fn find_binary_in_system_path(name: &str) -> Option<PathBuf> {
    let bin_name = format_bin_name(name);

    // Collect PATH directories from both system and user registry
    let mut path_dirs: Vec<PathBuf> = Vec::new();

    if let Some(system_path) = read_system_env_var("Path") {
        for dir in system_path.split(';') {
            let p = PathBuf::from(dir.trim());
            if !p.as_os_str().is_empty() {
                path_dirs.push(p);
            }
        }
    }

    if let Some(user_path) = read_user_env_var("Path") {
        for dir in user_path.split(';') {
            let p = PathBuf::from(dir.trim());
            if !p.as_os_str().is_empty() && !path_dirs.contains(&p) {
                path_dirs.push(p);
            }
        }
    }

    // Also check CARGO_HOME/bin from registry
    if let Some(cargo_home) = read_system_env_var("CARGO_HOME") {
        let bin_dir = PathBuf::from(&cargo_home).join("bin");
        if !path_dirs.contains(&bin_dir) {
            path_dirs.push(bin_dir);
        }
    }
    if let Some(cargo_home) = read_user_env_var("CARGO_HOME") {
        let bin_dir = PathBuf::from(&cargo_home).join("bin");
        if !path_dirs.contains(&bin_dir) {
            path_dirs.push(bin_dir);
        }
    }

    // Search each directory
    for dir in path_dirs {
        let candidate = dir.join(&bin_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bin_name() {
        let name = format_bin_name("rustup");
        if cfg!(target_os = "windows") {
            assert_eq!(name, "rustup.exe");
        } else {
            assert_eq!(name, "rustup");
        }
    }

    #[test]
    fn test_find_binary_not_found() {
        let result = find_binary("nonexistent_binary_xyz_12345");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BinaryNotFound(name) => assert_eq!(name, "nonexistent_binary_xyz_12345"),
            other => panic!("expected BinaryNotFound, got: {other}"),
        }
    }
}

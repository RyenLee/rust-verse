use crate::error::{AppError, AppResult};

#[cfg(not(windows))]
const RUSTVERSE_MARKER: &str = "# RustVerse managed";

/// Persist an environment variable to the system.
///
/// On Windows: writes to `HKEY_CURRENT_USER\Environment` via registry and broadcasts `WM_SETTINGCHANGE`.
/// On Unix: appends `export VARNAME=value` with a marker comment to the user's shell config.
#[tauri::command]
pub async fn persist_env_var(name: String, value: String) -> AppResult<()> {
    if name.is_empty() {
        return Err(AppError::Config(
            "Variable name cannot be empty".to_string(),
        ));
    }
    if name.contains('=') || name.contains('\0') {
        return Err(AppError::Config(
            "Variable name contains invalid characters".to_string(),
        ));
    }

    #[cfg(windows)]
    {
        persist_env_var_windows(&name, &value)
    }

    #[cfg(not(windows))]
    {
        persist_env_var_unix(&name, &value)
    }
}

/// Remove a persisted environment variable from the system.
#[tauri::command]
pub async fn remove_persisted_env_var(name: String) -> AppResult<()> {
    if name.is_empty() {
        return Err(AppError::Config(
            "Variable name cannot be empty".to_string(),
        ));
    }

    #[cfg(windows)]
    {
        remove_persisted_env_var_windows(&name)
    }

    #[cfg(not(windows))]
    {
        remove_persisted_env_var_unix(&name)
    }
}

/// Check if an environment variable is persisted at system level.
#[tauri::command]
pub async fn is_env_var_persisted(name: String) -> AppResult<bool> {
    #[cfg(windows)]
    {
        is_env_var_persisted_windows(&name)
    }

    #[cfg(not(windows))]
    {
        is_env_var_persisted_unix(&name)
    }
}

/// List all persisted Rust environment variables.
#[tauri::command]
pub async fn list_persisted_env_vars() -> AppResult<Vec<String>> {
    #[cfg(windows)]
    {
        list_persisted_env_vars_windows()
    }

    #[cfg(not(windows))]
    {
        list_persisted_env_vars_unix()
    }
}

// ============================================================
// Windows implementation using winreg + WM_SETTINGCHANGE
// ============================================================

/// Broadcast WM_SETTINGCHANGE to notify the system that environment variables changed.
#[cfg(windows)]
fn broadcast_env_change() {
    unsafe extern "system" {
        fn SendMessageTimeoutW(
            h_wnd: isize,
            msg: u32,
            w_param: usize,
            l_param: *const u16,
            fu_flags: u32,
            u_timeout: u32,
            result: *mut usize,
        ) -> isize;
    }

    const HWND_BROADCAST: isize = 0xffff;
    const WM_SETTINGCHANGE: u32 = 0x001a;
    const SMTO_ABORTIFHUNG: u32 = 0x0002;

    let wide_str: Vec<u16> = "Environment\0".encode_utf16().collect();
    let mut _result: usize = 0;
    unsafe {
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            wide_str.as_ptr(),
            SMTO_ABORTIFHUNG,
            5000,
            &mut _result,
        );
    }
}

#[cfg(windows)]
fn persist_env_var_windows(name: &str, value: &str) -> AppResult<()> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_SET_VALUE)
        .map_err(|e| AppError::Config(format!("failed to open registry: {e}")))?;

    env.set_value(name, &value)
        .map_err(|e| AppError::Config(format!("failed to set registry value: {e}")))?;

    broadcast_env_change();

    Ok(())
}

#[cfg(windows)]
fn remove_persisted_env_var_windows(name: &str) -> AppResult<()> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_SET_VALUE)
        .map_err(|e| AppError::Config(format!("failed to open registry: {e}")))?;

    // Check if the value exists first
    if env.get_raw_value(name).is_err() {
        return Ok(()); // Already not present, nothing to do
    }

    env.delete_value(name)
        .map_err(|e| AppError::Config(format!("failed to delete registry value: {e}")))?;

    broadcast_env_change();

    Ok(())
}

#[cfg(windows)]
fn is_env_var_persisted_windows(name: &str) -> AppResult<bool> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ)
        .map_err(|e| AppError::Config(format!("failed to open registry: {e}")))?;

    Ok(env.get_raw_value(name).is_ok())
}

/// All known Rust-related environment variable names.
/// Used by `list_persisted_env_vars_windows` to check which ones
/// are actually present in the user's registry.
#[cfg(windows)]
fn known_rust_env_var_names() -> Vec<String> {
    // These are all the env vars defined in our database (default_env_vars)
    vec![
        "CARGO_HOME".to_string(),
        "RUSTUP_HOME".to_string(),
        "CARGO_BUILD_TARGET_DIR".to_string(),
        "CARGO_TARGET_DIR".to_string(),
        "CARGO_INCREMENTAL".to_string(),
        "CARGO_NET_GIT_FETCH_WITH_CLI".to_string(),
        "CARGO_NET_RETRY".to_string(),
        "CARGO_TERM_COLOR".to_string(),
        "CARGO_HTTP_TIMEOUT".to_string(),
        "HTTP_PROXY".to_string(),
        "HTTPS_PROXY".to_string(),
        "NO_PROXY".to_string(),
        "RUSTUP_DIST_SERVER".to_string(),
        "RUSTUP_UPDATE_ROOT".to_string(),
        "RUSTFLAGS".to_string(),
        "CARGO_MAKEFLAGS".to_string(),
        "MIRIFLAGS".to_string(),
        "RUST_BACKTRACE".to_string(),
        "RUST_LOG".to_string(),
        "RUSTC_BOOTSTRAP".to_string(),
        "CARGO_PROFILE_RELEASE_LTO".to_string(),
        "CARGO_PROFILE_RELEASE_CODEGEN_UNITS".to_string(),
        "CARGO_PROFILE_DEV_OPT_LEVEL".to_string(),
        "RUSTC_FORCE_INCREMENTAL".to_string(),
    ]
}

#[cfg(windows)]
fn list_persisted_env_vars_windows() -> AppResult<Vec<String>> {
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ)
        .map_err(|e| AppError::Config(format!("failed to open registry: {e}")))?;

    // Check which known Rust env vars are actually present in the registry
    let known = known_rust_env_var_names();
    let mut result = Vec::new();
    for name in &known {
        if env.get_raw_value(name).is_ok() {
            result.push(name.clone());
        }
    }

    // Also check for any env vars with Rust-related prefixes that we might not have listed
    let rust_prefixes = ["RUST", "CARGO", "MIRIFLAGS"];
    for (name, _) in env.enum_values().filter_map(|r| r.ok()) {
        if !result.contains(&name) && rust_prefixes.iter().any(|prefix| name.starts_with(prefix)) {
            result.push(name);
        }
    }

    Ok(result)
}

// ============================================================
// Unix implementation using shell config files
// ============================================================

#[cfg(not(windows))]
fn persist_env_var_unix(name: &str, value: &str) -> AppResult<()> {
    let shell_config = find_shell_config()?;

    // Read existing content
    let content = std::fs::read_to_string(&shell_config).unwrap_or_default();

    // Remove any existing managed lines for this variable
    let filtered = remove_managed_lines(&content, name);

    // Append the new export line with marker
    let export_line = format!("export {name}={value}  {RUSTVERSE_MARKER}");
    let new_content = if filtered.is_empty() {
        export_line
    } else if filtered.ends_with('\n') {
        format!("{filtered}{export_line}\n")
    } else {
        format!("{filtered}\n{export_line}\n")
    };

    std::fs::write(&shell_config, new_content)
        .map_err(|e| AppError::Config(format!("failed to write shell config: {e}")))?;

    Ok(())
}

#[cfg(not(windows))]
fn remove_persisted_env_var_unix(name: &str) -> AppResult<()> {
    let shell_config = find_shell_config()?;

    let content = std::fs::read_to_string(&shell_config).unwrap_or_default();

    let filtered = remove_managed_lines(&content, name);

    std::fs::write(&shell_config, filtered)
        .map_err(|e| AppError::Config(format!("failed to write shell config: {e}")))?;

    Ok(())
}

#[cfg(not(windows))]
fn is_env_var_persisted_unix(name: &str) -> AppResult<bool> {
    let shell_config = match find_shell_config() {
        Ok(p) => p,
        Err(_) => return Ok(false),
    };

    let content = std::fs::read_to_string(&shell_config).unwrap_or_default();

    Ok(content
        .lines()
        .any(|line| line.contains(RUSTVERSE_MARKER) && line.contains(&format!("export {name}="))))
}

#[cfg(not(windows))]
fn list_persisted_env_vars_unix() -> AppResult<Vec<String>> {
    let shell_config = match find_shell_config() {
        Ok(p) => p,
        Err(_) => return Ok(Vec::new()),
    };

    let content = std::fs::read_to_string(&shell_config).unwrap_or_default();

    let mut result = Vec::new();
    for line in content.lines() {
        if line.contains(RUSTVERSE_MARKER) {
            if let Some(name) = line.strip_prefix("export ") {
                if let Some(name) = name.split('=').next() {
                    result.push(name.trim().to_string());
                }
            }
        }
    }

    Ok(result)
}

#[cfg(not(windows))]
fn find_shell_config() -> AppResult<std::path::PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Config("cannot determine home directory".to_string()))?;

    let shell = std::env::var("SHELL").unwrap_or_default();

    // Determine which shell config file to use
    let config_file = if shell.contains("zsh") {
        home.join(".zshrc")
    } else if shell.contains("bash") {
        // Prefer .bashrc; fall back to .bash_profile
        let bashrc = home.join(".bashrc");
        if bashrc.exists() {
            bashrc
        } else {
            home.join(".bash_profile")
        }
    } else {
        // Fallback: try .profile
        home.join(".profile")
    };

    if !config_file.exists() {
        // Create the file if it doesn't exist
        std::fs::write(&config_file, "")
            .map_err(|e| AppError::Config(format!("failed to create shell config: {e}")))?;
    }

    Ok(config_file)
}

#[cfg(not(windows))]
fn remove_managed_lines(content: &str, var_name: &str) -> String {
    content
        .lines()
        .filter(|line| {
            // Keep lines that are NOT managed exports for this variable
            !(line.contains(RUSTVERSE_MARKER) && line.contains(&format!("export {var_name}=")))
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn test_registry_delete_roundtrip() {
        use winreg::RegKey;
        use winreg::enums::*;

        let test_name = "RustVerseDeleteTest";
        let test_value = "test_value_12345";

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags("Environment", KEY_READ | KEY_SET_VALUE)
            .unwrap();

        // Write
        env.set_value(test_name, &test_value).unwrap();

        // Verify write
        let read_value: String = env.get_value(test_name).unwrap();
        assert_eq!(read_value, test_value);

        // Delete
        env.delete_value(test_name).unwrap();

        // Verify delete
        let result: Result<String, _> = env.get_value(test_name);
        assert!(result.is_err(), "Value should have been deleted");

        // Cleanup
        let _ = env.delete_value(test_name);
    }

    #[cfg(windows)]
    #[test]
    fn test_remove_persisted_env_var_windows() {
        let test_name = "RustVerseRemoveTest";
        let test_value = "test_remove_value";

        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags(
                "Environment",
                winreg::enums::KEY_READ | winreg::enums::KEY_SET_VALUE,
            )
            .unwrap();

        // Write first
        env.set_value(test_name, &test_value).unwrap();

        // Verify it exists
        let val: String = env.get_value(test_name).unwrap();
        assert_eq!(val, test_value);

        // Now call our function
        let result = remove_persisted_env_var_windows(test_name);
        assert!(
            result.is_ok(),
            "remove_persisted_env_var_windows should succeed: {:?}",
            result
        );

        // Verify it's gone
        let env_read = hkcu
            .open_subkey_with_flags("Environment", winreg::enums::KEY_READ)
            .unwrap();
        let check: Result<String, _> = env_read.get_value(test_name);
        assert!(
            check.is_err(),
            "Value should have been deleted by remove_persisted_env_var_windows"
        );

        // Cleanup
        let _ = env.delete_value(test_name);
    }
}

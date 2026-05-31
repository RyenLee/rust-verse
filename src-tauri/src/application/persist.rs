//! Environment variable persistence business logic.
//!
//! Platform-specific implementations for persisting env vars to the
//! system (Windows Registry or Unix shell config files).

use crate::domain::error::{AppError, AppResult};

#[cfg(not(windows))]
const RUSTVERSE_MARKER: &str = "# RustVerse managed";

// ============================================================
// Public API — dispatched to platform-specific implementation
// ============================================================

#[allow(unused_variables)]
pub fn persist_env_var(name: &str, value: &str) -> AppResult<()> {
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
        persist_env_var_windows(name, value)
    }

    #[cfg(not(windows))]
    {
        persist_env_var_unix(name, value)
    }
}

#[allow(unused_variables)]
pub fn remove_persisted_env_var(name: &str) -> AppResult<()> {
    if name.is_empty() {
        return Err(AppError::Config(
            "Variable name cannot be empty".to_string(),
        ));
    }

    #[cfg(windows)]
    {
        remove_persisted_env_var_windows(name)
    }

    #[cfg(not(windows))]
    {
        remove_persisted_env_var_unix(name)
    }
}

#[allow(unused_variables)]
pub fn is_env_var_persisted(name: &str) -> AppResult<bool> {
    #[cfg(windows)]
    {
        is_env_var_persisted_windows(name)
    }

    #[cfg(not(windows))]
    {
        is_env_var_persisted_unix(name)
    }
}

pub fn list_persisted_env_vars() -> AppResult<Vec<String>> {
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

    if name == "CARGO_HOME" {
        let _ = add_cargo_home_bin_to_path_windows(&env, value);
    }

    broadcast_env_change();
    Ok(())
}

#[cfg(windows)]
fn add_cargo_home_bin_to_path_windows(
    env: &winreg::RegKey,
    cargo_home_value: &str,
) -> AppResult<()> {
    let resolved_bin = format!(r"{}\bin", cargo_home_value);
    let current_path: Result<String, _> = env.get_value("Path");
    let new_path = match current_path {
        Ok(path) => {
            let entries: Vec<&str> = path.split(';').collect();
            let legacy_entry = format!("%{}%\\bin", "CARGO_HOME");
            if entries
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&resolved_bin))
            {
                return Ok(());
            }
            if entries
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&legacy_entry))
            {
                return Ok(());
            }
            if path.ends_with(';') {
                format!("{}{}", path, resolved_bin)
            } else {
                format!("{};{}", path, resolved_bin)
            }
        }
        Err(_) => resolved_bin.clone(),
    };
    env.set_value("Path", &new_path)
        .map_err(|e| AppError::Config(format!("failed to update PATH in registry: {e}")))?;
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

    if env.get_raw_value(name).is_err() {
        return Ok(());
    }
    if name == "CARGO_HOME" {
        let cargo_home_value: Result<String, _> = env.get_value("CARGO_HOME");
        let _ = remove_cargo_home_bin_from_path_windows(&env, cargo_home_value.ok());
    }
    env.delete_value(name)
        .map_err(|e| AppError::Config(format!("failed to delete registry value: {e}")))?;
    broadcast_env_change();
    Ok(())
}

#[cfg(windows)]
fn remove_cargo_home_bin_from_path_windows(
    env: &winreg::RegKey,
    cargo_home_value: Option<String>,
) -> AppResult<()> {
    let legacy_entry = r"%CARGO_HOME%\bin";
    let current_path: Result<String, _> = env.get_value("Path");
    if let Ok(path) = current_path {
        let entries: Vec<&str> = path.split(';').collect();
        let filtered: Vec<&str> = entries
            .iter()
            .filter(|e| {
                if e.eq_ignore_ascii_case(legacy_entry) {
                    return false;
                }
                if let Some(ref val) = cargo_home_value {
                    let resolved = format!(r"{}\bin", val);
                    if e.eq_ignore_ascii_case(&resolved) {
                        return false;
                    }
                }
                true
            })
            .copied()
            .collect();
        if filtered.len() != entries.len() {
            let new_path = filtered.join(";");
            env.set_value("Path", &new_path)
                .map_err(|e| AppError::Config(format!("failed to update PATH in registry: {e}")))?;
        }
    }
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

#[cfg(windows)]
fn known_rust_env_var_names() -> Vec<String> {
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

    let known = known_rust_env_var_names();
    let mut result = Vec::new();
    for name in &known {
        if env.get_raw_value(name).is_ok() {
            result.push(name.clone());
        }
    }
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
    let content = std::fs::read_to_string(&shell_config).unwrap_or_default();
    let filtered = remove_managed_lines(&content, name);
    let export_line = format!("export {name}={value}  {RUSTVERSE_MARKER}");
    let mut new_content = if filtered.is_empty() {
        export_line
    } else if filtered.ends_with('\n') {
        format!("{filtered}{export_line}\n")
    } else {
        format!("{filtered}\n{export_line}\n")
    };
    if name == "CARGO_HOME" {
        let path_line = format!(r#"export PATH="$CARGO_HOME/bin:$PATH"  {RUSTVERSE_MARKER}_PATH"#);
        let without_path = remove_managed_path_lines(&new_content);
        new_content = if without_path.ends_with('\n') {
            format!("{without_path}{path_line}\n")
        } else {
            format!("{without_path}\n{path_line}\n")
        };
    }
    std::fs::write(&shell_config, new_content)
        .map_err(|e| AppError::Config(format!("failed to write shell config: {e}")))?;
    Ok(())
}

#[cfg(not(windows))]
fn remove_persisted_env_var_unix(name: &str) -> AppResult<()> {
    let shell_config = find_shell_config()?;
    let content = std::fs::read_to_string(&shell_config).unwrap_or_default();
    let mut filtered = remove_managed_lines(&content, name);
    if name == "CARGO_HOME" {
        filtered = remove_managed_path_lines(&filtered);
    }
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
    let config_file = if shell.contains("zsh") {
        home.join(".zshrc")
    } else if shell.contains("bash") {
        let bashrc = home.join(".bashrc");
        if bashrc.exists() {
            bashrc
        } else {
            home.join(".bash_profile")
        }
    } else {
        home.join(".profile")
    };
    if !config_file.exists() {
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
            !(line.contains(RUSTVERSE_MARKER) && line.contains(&format!("export {var_name}=")))
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

#[cfg(not(windows))]
fn remove_managed_path_lines(content: &str) -> String {
    content
        .lines()
        .filter(|line| {
            !(line.contains(&format!("{RUSTVERSE_MARKER}_PATH")) && line.contains("CARGO_HOME/bin"))
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

        env.set_value(test_name, &test_value).unwrap();
        let read_value: String = env.get_value(test_name).unwrap();
        assert_eq!(read_value, test_value);
        env.delete_value(test_name).unwrap();
        let result: Result<String, _> = env.get_value(test_name);
        assert!(result.is_err());

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

        env.set_value(test_name, &test_value).unwrap();
        let val: String = env.get_value(test_name).unwrap();
        assert_eq!(val, test_value);

        let result = remove_persisted_env_var_windows(test_name);
        assert!(result.is_ok());

        let env_read = hkcu
            .open_subkey_with_flags("Environment", winreg::enums::KEY_READ)
            .unwrap();
        let check: Result<String, _> = env_read.get_value(test_name);
        assert!(check.is_err());

        let _ = env.delete_value(test_name);
    }
}

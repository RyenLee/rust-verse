use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;
use crate::utils::exec;

/// Override information for a directory.
#[derive(Debug, Clone, Serialize)]
pub struct OverrideInfo {
    /// Directory path with the override
    pub path: String,
    /// Toolchain name set as override
    pub toolchain: String,
}

/// Get the override for a specific directory.
///
/// Executes `rustup override list` and filters for the given path.
#[tauri::command]
pub async fn get_override(rustup_path: String, dir_path: String, state: State<'_, AppState>) -> AppResult<Option<OverrideInfo>> {
    crate::system::env::validate_rust_binary(&rustup_path).map_err(|e| crate::error::AppError::Command(e))?;
    let output = exec::run_command(&rustup_path, &["override", "list"], 30).await?;
    let parsing = crate::db::get_parsing_config(&state.db);
    let no_overrides = parsing.no_overrides;
    let override_info = parse_override_list(&output, &no_overrides)
        .into_iter()
        .find(|o| o.path == dir_path);
    Ok(override_info)
}

/// Set a toolchain override for a directory.
#[tauri::command]
pub async fn set_override(
    rustup_path: String,
    dir_path: String,
    toolchain: String,
) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&rustup_path).map_err(|e| crate::error::AppError::Command(e))?;
    validate_dir_path(&dir_path)?;
    // rustup override set requires being in the target directory
    // Use --path flag if available, otherwise cd
    exec::run_command_with_cwd(&rustup_path, &["override", "set", &toolchain], &dir_path, 60).await?;
    Ok(())
}

/// Remove a toolchain override for a directory.
#[tauri::command]
pub async fn remove_override(rustup_path: String, dir_path: String) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&rustup_path).map_err(|e| crate::error::AppError::Command(e))?;
    validate_dir_path(&dir_path)?;
    exec::run_command_with_cwd(&rustup_path, &["override", "unset"], &dir_path, 60).await?;
    Ok(())
}

/// List all overrides.
#[tauri::command]
pub async fn list_overrides(rustup_path: String, state: State<'_, AppState>) -> AppResult<Vec<OverrideInfo>> {
    crate::system::env::validate_rust_binary(&rustup_path).map_err(|e| crate::error::AppError::Command(e))?;
    let output = exec::run_command(&rustup_path, &["override", "list"], 30).await?;
    let parsing = crate::db::get_parsing_config(&state.db);
    let no_overrides = parsing.no_overrides;
    Ok(parse_override_list(&output, &no_overrides))
}

/// Parse `rustup override list` output.
///
/// Example output:
/// ```text
/// /home/user/project    nightly-x86_64-unknown-linux-gnu
/// /home/user/other      stable-x86_64-unknown-linux-gnu
/// ```
pub fn parse_override_list(output: &str, no_overrides_marker: &str) -> Vec<OverrideInfo> {
    let mut overrides = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.contains(no_overrides_marker) {
            continue;
        }

        // Split on whitespace: first part is path, rest is toolchain
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() == 2 {
            overrides.push(OverrideInfo {
                path: parts[0].trim().to_string(),
                toolchain: parts[1].trim().to_string(),
            });
        }
    }

    overrides
}

/// Validate that a directory path exists and is actually a directory.
fn validate_dir_path(dir_path: &str) -> AppResult<()> {
    let path = std::path::Path::new(dir_path);
    if !path.exists() {
        return Err(crate::error::AppError::Command(
            format!("directory does not exist: {dir_path}")
        ));
    }
    if !path.is_dir() {
        return Err(crate::error::AppError::Command(
            format!("path is not a directory: {dir_path}")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_override_list() {
        let output = "/home/user/project    nightly-x86_64-unknown-linux-gnu\n/home/user/other      stable-x86_64-unknown-linux-gnu";
        let result = parse_override_list(output, "no overrides");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "/home/user/project");
        assert_eq!(result[0].toolchain, "nightly-x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_parse_override_list_empty() {
        let output = "no overrides";
        let result = parse_override_list(output, "no overrides");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_override_list_blank() {
        let result = parse_override_list("", "no overrides");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_override_list_multiple_spaces() {
        let output = "/home/user/project      nightly-x86_64-unknown-linux-gnu";
        let result = parse_override_list(output, "no overrides");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "/home/user/project");
        assert_eq!(result[0].toolchain, "nightly-x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_parse_override_list_windows_path() {
        let output = "C:\\Users\\dev\\project    stable-x86_64-pc-windows-msvc";
        let result = parse_override_list(output, "no overrides");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "C:\\Users\\dev\\project");
    }

    #[test]
    fn test_parse_override_list_no_toolchain() {
        let output = "/some/path";
        let result = parse_override_list(output, "no overrides");
        assert!(result.is_empty());
    }
}

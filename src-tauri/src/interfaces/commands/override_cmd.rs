//! Toolchain override commands — thin forwarding layer.

use tauri::{AppHandle, State};

use crate::domain::error::AppResult;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::domain::parsing;
use crate::infrastructure::exec;
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export for backward compatibility
pub use crate::domain::entity::OverrideInfo;
#[allow(unused_imports)]
pub use crate::domain::parsing::parse_override_list;

/// Get the override for a specific directory.
#[tauri::command]
pub async fn get_override(
    rustup_path: String,
    dir_path: String,
    state: State<'_, AppState>,
) -> AppResult<Option<OverrideInfo>> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let output = exec::run_command(&rustup_path, &["override", "list"], 30).await?;
    let db_parsing = crate::infrastructure::db::get_parsing_config(&*state.store);
    let override_info = parsing::parse_override_list(&output, &db_parsing.no_overrides)
        .into_iter()
        .find(|o| o.path == dir_path);
    Ok(override_info)
}

/// Set a toolchain override for a directory.
#[tauri::command]
pub async fn set_override(
    app: AppHandle,
    rustup_path: String,
    dir_path: String,
    toolchain: String,
) -> AppResult<()> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    validate_dir_path(&dir_path)?;
    exec::run_command_with_cwd(
        &rustup_path,
        &["override", "set", &toolchain],
        &dir_path,
        60,
    )
    .await?;

    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::OverrideSet,
        &[("toolchain", &toolchain), ("path", &dir_path)],
        Some("/overrides"),
    );

    Ok(())
}

/// Remove a toolchain override for a directory.
#[tauri::command]
pub async fn remove_override(
    app: AppHandle,
    rustup_path: String,
    dir_path: String,
) -> AppResult<()> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    validate_dir_path(&dir_path)?;
    exec::run_command_with_cwd(&rustup_path, &["override", "unset"], &dir_path, 60).await?;

    notifier::notify(
        &app,
        Category::Operation,
        Priority::Low,
        NotificationKey::OverrideRemoved,
        &[("path", &dir_path)],
        Some("/overrides"),
    );

    Ok(())
}

/// List all overrides.
#[tauri::command]
pub async fn list_overrides(
    rustup_path: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<OverrideInfo>> {
    crate::infrastructure::system::env::validate_rust_binary(&rustup_path)
        .map_err(|e| crate::domain::error::AppError::Command(e))?;
    let output = exec::run_command(&rustup_path, &["override", "list"], 30).await?;
    let db_parsing = crate::infrastructure::db::get_parsing_config(&*state.store);
    Ok(parsing::parse_override_list(
        &output,
        &db_parsing.no_overrides,
    ))
}

/// Validate that a directory path exists and is actually a directory.
fn validate_dir_path(dir_path: &str) -> AppResult<()> {
    let path = std::path::Path::new(dir_path);
    if !path.exists() {
        return Err(crate::domain::error::AppError::Command(format!(
            "directory does not exist: {dir_path}"
        )));
    }
    if !path.is_dir() {
        return Err(crate::domain::error::AppError::Command(format!(
            "path is not a directory: {dir_path}"
        )));
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

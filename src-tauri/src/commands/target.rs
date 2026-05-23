use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;
use crate::utils::exec;

/// Target information.
#[derive(Debug, Clone, Serialize)]
pub struct TargetInfo {
    /// Target triple, e.g. "x86_64-pc-windows-msvc"
    pub name: String,
    /// Whether the target is installed
    pub installed: bool,
}

/// List targets for a toolchain.
#[tauri::command]
pub async fn list_targets(rustup_path: String, toolchain: String, state: State<'_, AppState>) -> AppResult<Vec<TargetInfo>> {
    let output = exec::run_command(&rustup_path, &["target", "list", "--toolchain", &toolchain]).await?;
    let parsing = crate::db::get_parsing_config(&state.db);
    Ok(parse_target_list(&output, &parsing.installed_marker, &parsing.default_marker))
}

/// Add a target to a toolchain.
#[tauri::command]
pub async fn add_target(
    rustup_path: String,
    toolchain: String,
    target: String,
) -> AppResult<()> {
    exec::run_command(&rustup_path, &["target", "add", &target, "--toolchain", &toolchain]).await?;
    Ok(())
}

/// Remove a target from a toolchain.
#[tauri::command]
pub async fn remove_target(
    rustup_path: String,
    toolchain: String,
    target: String,
) -> AppResult<()> {
    exec::run_command(&rustup_path, &["target", "remove", &target, "--toolchain", &toolchain]).await?;
    Ok(())
}

/// Parse `rustup target list` output.
///
/// Example:
/// ```text
/// x86_64-pc-windows-msvc (installed)
/// x86_64-pc-windows-gnu
/// aarch64-unknown-linux-gnu
/// ```
pub fn parse_target_list(output: &str, installed_marker: &str, default_marker: &str) -> Vec<TargetInfo> {
    let mut targets = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let installed = line.contains(installed_marker);
        let name = line.replace(installed_marker, "").replace(default_marker, "").trim().to_string();

        if !name.is_empty() {
            targets.push(TargetInfo { name, installed });
        }
    }

    targets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_list() {
        let output = "x86_64-pc-windows-msvc (installed)\nx86_64-pc-windows-gnu\naarch64-unknown-linux-gnu";
        let result = parse_target_list(output, "(installed)", "(default)");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "x86_64-pc-windows-msvc");
        assert!(result[0].installed);
        assert!(!result[1].installed);
    }

    #[test]
    fn test_parse_target_list_empty() {
        let result = parse_target_list("", "(installed)", "(default)");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_target_list_with_default() {
        let output = "x86_64-pc-windows-msvc (installed) (default)\naarch64-pc-windows-msvc";
        let result = parse_target_list(output, "(installed)", "(default)");
        assert_eq!(result.len(), 2);
        assert!(result[0].installed);
        assert_eq!(result[0].name, "x86_64-pc-windows-msvc");
    }

    #[test]
    fn test_parse_target_list_many() {
        let output = "x86_64-pc-windows-msvc (installed)\nx86_64-pc-windows-gnu (installed)\naarch64-unknown-linux-gnu\nwasm32-unknown-unknown";
        let result = parse_target_list(output, "(installed)", "(default)");
        assert_eq!(result.len(), 4);
        assert!(result[0].installed);
        assert!(result[1].installed);
        assert!(!result[2].installed);
        assert!(!result[3].installed);
    }
}

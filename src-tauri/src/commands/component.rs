use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;
use crate::utils::exec;

/// Component or target installation status.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentInfo {
    /// Component name, e.g. "rustfmt", "clippy"
    pub name: String,
    /// Whether the component is installed
    pub installed: bool,
}

/// List components for a toolchain.
#[tauri::command]
pub async fn list_components(rustup_path: String, toolchain: String, state: State<'_, AppState>) -> AppResult<Vec<ComponentInfo>> {
    let output = exec::run_command(&rustup_path, &["component", "list", "--toolchain", &toolchain]).await?;
    let parsing = crate::db::get_parsing_config(&state.db);
    Ok(parse_component_list(&output, &parsing.installed_marker))
}

/// Add a component to a toolchain.
#[tauri::command]
pub async fn add_component(
    rustup_path: String,
    toolchain: String,
    component: String,
) -> AppResult<()> {
    exec::run_command(&rustup_path, &["component", "add", &component, "--toolchain", &toolchain]).await?;
    Ok(())
}

/// Remove a component from a toolchain.
#[tauri::command]
pub async fn remove_component(
    rustup_path: String,
    toolchain: String,
    component: String,
) -> AppResult<()> {
    exec::run_command(&rustup_path, &["component", "remove", &component, "--toolchain", &toolchain]).await?;
    Ok(())
}

/// Parse `rustup component list` output.
///
/// Example:
/// ```text
/// rustfmt (installed)
/// clippy (installed)
/// rls
/// rust-analysis
/// ```
pub fn parse_component_list(output: &str, installed_marker: &str) -> Vec<ComponentInfo> {
    let mut components = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let installed = line.contains(installed_marker);
        let name = line.replace(installed_marker, "").trim().to_string();

        if !name.is_empty() {
            components.push(ComponentInfo { name, installed });
        }
    }

    components
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_component_list() {
        let output = "rustfmt (installed)\nclippy (installed)\nrls\nrust-analysis";
        let result = parse_component_list(output, "(installed)");
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].name, "rustfmt");
        assert!(result[0].installed);
        assert_eq!(result[2].name, "rls");
        assert!(!result[2].installed);
    }

    #[test]
    fn test_parse_component_list_empty() {
        let result = parse_component_list("", "(installed)");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_component_list_with_toolchain_suffix() {
        let output = "rustfmt-x86_64-pc-windows-msvc (installed)\nclippy-x86_64-pc-windows-msvc";
        let result = parse_component_list(output, "(installed)");
        assert_eq!(result.len(), 2);
        assert!(result[0].installed);
        assert!(!result[1].installed);
    }

    #[test]
    fn test_parse_component_list_whitespace() {
        let output = "  rustfmt (installed)  \n  rls  ";
        let result = parse_component_list(output, "(installed)");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "rustfmt");
        assert_eq!(result[1].name, "rls");
    }
}

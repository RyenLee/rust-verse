use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::state::AppState;
use crate::utils::exec;

/// Information about an installed cargo plugin.
#[derive(Debug, Clone, Serialize)]
pub struct CargoPluginInfo {
    /// Plugin name without `cargo-` prefix, e.g. "audit", "expand"
    pub name: String,
    /// Full crate name, e.g. "cargo-audit"
    pub crate_name: String,
    /// Installed version
    pub version: String,
    /// Whether this is an officially maintained Rust tool
    pub is_official: bool,
}

fn is_official_plugin(crate_name: &str, official_names: &[String]) -> bool {
    official_names.iter().any(|n| n == crate_name)
}

/// List installed cargo plugins.
#[tauri::command]
pub async fn list_cargo_plugins(
    cargo_path: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<CargoPluginInfo>> {
    crate::system::env::validate_rust_binary(&cargo_path).map_err(|e| crate::error::AppError::Command(e))?;
    let output = exec::run_command(&cargo_path, &["install", "--list"], 30).await?;
    let parsing = crate::db::get_parsing_config(&state.db);
    let official_names = crate::db::get_plugin_names(&state.db);
    Ok(parse_cargo_plugin_list(
        &output,
        &parsing.cargo_prefix,
        &official_names,
    ))
}

/// Install a cargo plugin with streaming output.
///
/// Emits config-specified log events with each line of output,
/// and a finished event when done.
#[tauri::command]
pub async fn install_plugin(
    app: AppHandle,
    state: State<'_, AppState>,
    cargo_path: String,
    crate_name: String,
) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&cargo_path).map_err(|e| crate::error::AppError::Command(e))?;
    let (locale_key, log_event, finished_event) = {
        let events = crate::db::get_events_config(&state.db);
        let locale_key = crate::db::get_simple(&state.db, "locale.force_locale")
            .unwrap_or_else(crate::db::default_force_locale);
        (
            locale_key,
            events.plugin_install_log,
            events.plugin_install_finished,
        )
    };

    exec::run_command_with_streaming(
        app,
        &cargo_path,
        &["install", &crate_name],
        &locale_key,
        &log_event,
        &finished_event,
        600, // 10 minute timeout for plugin installation
    )
    .await
}

/// Uninstall a cargo plugin.
#[tauri::command]
pub async fn uninstall_plugin(cargo_path: String, crate_name: String) -> AppResult<()> {
    crate::system::env::validate_rust_binary(&cargo_path).map_err(|e| crate::error::AppError::Command(e))?;
    exec::run_command(&cargo_path, &["uninstall", &crate_name], 120).await?;
    Ok(())
}

/// Search cargo plugins on crates.io.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// Search for cargo plugins using `cargo search`.
#[tauri::command]
pub async fn search_plugins(
    cargo_path: String,
    query: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<SearchResult>> {
    crate::system::env::validate_rust_binary(&cargo_path).map_err(|e| crate::error::AppError::Command(e))?;
    let timeout = crate::db::get_simple(&state.db, "timeouts.cargo_search_seconds")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(crate::db::default_cargo_search_seconds);
    let output = exec::run_command_with_timeout(&cargo_path, &["search", &query], timeout).await?;
    Ok(parse_search_results(&output))
}

/// Parse `cargo search` output.
pub fn parse_search_results(output: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format: "crate-name = "version" # description"
        let Some((name_ver, desc_part)) = line.split_once('#') else {
            continue;
        };

        let name_ver = name_ver.trim();
        let description = desc_part.trim().to_string();

        let Some((name, ver)) = name_ver.split_once(" = ") else {
            continue;
        };

        let name = name.trim().to_string();
        let version = ver.trim().trim_matches('"').to_string();

        results.push(SearchResult {
            name,
            description,
            version,
        });
    }

    results
}

/// Parse `cargo install --list` output.
pub fn parse_cargo_plugin_list(
    output: &str,
    cargo_prefix: &str,
    official_names: &[String],
) -> Vec<CargoPluginInfo> {
    let mut plugins = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        if !line.starts_with(cargo_prefix) || !line.ends_with(':') {
            continue;
        }

        let line = &line[..line.len() - 1];

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }

        let crate_name = parts[0].to_string();
        let version = parts[1].trim_start_matches('v').to_string();
        let name = crate_name
            .strip_prefix(cargo_prefix)
            .unwrap_or(&crate_name)
            .to_string();

        plugins.push(CargoPluginInfo {
            name,
            crate_name: crate_name.clone(),
            version,
            is_official: is_official_plugin(&crate_name, official_names),
        });
    }

    plugins
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config_values() -> (String, Vec<String>) {
        (
            "cargo-".to_string(),
            vec![
                "cargo-clippy".to_string(),
                "cargo-fmt".to_string(),
                "cargo-miri".to_string(),
                "cargo-rustdoc".to_string(),
                "cargo-test-fixture".to_string(),
                "rustfmt".to_string(),
                "clippy".to_string(),
                "miri".to_string(),
            ],
        )
    }

    #[test]
    fn test_parse_cargo_plugin_list() {
        let (prefix, official) = default_config_values();
        let output = "cargo-audit v0.18.3:\n    cargo-audit\ncargo-expand v1.0.0:\n    cargo-expand\nrust-script v0.3.0:\n    rust-script";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "audit");
        assert_eq!(result[0].crate_name, "cargo-audit");
        assert_eq!(result[0].version, "0.18.3");
        assert!(!result[0].is_official);
        assert_eq!(result[1].name, "expand");
    }

    #[test]
    fn test_parse_cargo_plugin_list_empty() {
        let (prefix, official) = default_config_values();
        let result = parse_cargo_plugin_list("", &prefix, &official);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_cargo_plugin_list_no_cargo_prefix() {
        let (prefix, official) = default_config_values();
        let output = "some-tool v1.0.0:\n    some-tool";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_cargo_plugin_list_multiple() {
        let (prefix, official) = default_config_values();
        let output = "cargo-audit v0.18.3:\n    cargo-audit\ncargo-clippy v0.1.0:\n    cargo-clippy\ncargo-outdated v0.12.0:\n    cargo-outdated";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "audit");
        assert!(!result[0].is_official);
        assert_eq!(result[1].name, "clippy");
        assert!(result[1].is_official);
        assert_eq!(result[2].name, "outdated");
        assert_eq!(result[2].version, "0.12.0");
    }

    #[test]
    fn test_parse_cargo_plugin_list_no_colon() {
        let (prefix, official) = default_config_values();
        let output = "cargo-audit v0.18.3\n    cargo-audit";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_cargo_plugin_list_no_version() {
        let (prefix, official) = default_config_values();
        let output = "cargo-audit:\n    cargo-audit";
        let result = parse_cargo_plugin_list(output, &prefix, &official);
        assert!(result.is_empty());
    }
}

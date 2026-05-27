//! Domain parsing services — pure functions for parsing CLI tool output.
//!
//! No framework dependencies, no I/O, no side effects.

use crate::domain::entity::{
    CargoPluginInfo, CrmTestResult, ComponentInfo, MirrorInfo, MirrorLatency, OverrideInfo,
    SearchResult, TargetInfo, ToolchainInfo, UpdateInfo,
};
use crate::domain::error::AppResult;

// ── Component ──

pub fn parse_component_list(output: &str, installed_marker: &str) -> Vec<ComponentInfo> {
    let mut components = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let installed = line.contains(installed_marker);
        let name = line.replace(installed_marker, "").trim().to_string();
        if !name.is_empty() {
            components.push(ComponentInfo { name, installed });
        }
    }
    components
}

// ── Target ──

pub fn parse_target_list(output: &str, installed_marker: &str, default_marker: &str) -> Vec<TargetInfo> {
    let mut targets = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let installed = line.contains(installed_marker);
        let name = line.replace(installed_marker, "").replace(default_marker, "").trim().to_string();
        if !name.is_empty() {
            targets.push(TargetInfo { name, installed });
        }
    }
    targets
}

// ── Override ──

pub fn parse_override_list(output: &str, no_overrides_marker: &str) -> Vec<OverrideInfo> {
    let mut overrides = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.contains(no_overrides_marker) { continue; }
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

// ── Plugin ──

fn is_official_plugin(crate_name: &str, official_names: &[String]) -> bool {
    official_names.iter().any(|n| n == crate_name)
}

pub fn parse_cargo_plugin_list(output: &str, cargo_prefix: &str, official_names: &[String]) -> Vec<CargoPluginInfo> {
    let mut plugins = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if !line.ends_with(':') { continue; }
        let line = &line[..line.len() - 1];
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 { continue; }
        let crate_name = parts[0].to_string();
        let version = parts[1].trim_start_matches('v').to_string();
        let name = crate_name.strip_prefix(cargo_prefix).unwrap_or(&crate_name).to_string();
        plugins.push(CargoPluginInfo {
            name,
            crate_name: crate_name.clone(),
            version,
            is_official: is_official_plugin(&crate_name, official_names),
        });
    }
    plugins
}

pub fn parse_search_results(output: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let Some((name_ver, desc_part)) = line.split_once('#') else { continue; };
        let description = desc_part.trim().to_string();
        let Some((name, ver)) = name_ver.trim().split_once(" = ") else { continue; };
        results.push(SearchResult {
            name: name.trim().to_string(),
            description,
            version: ver.trim().trim_matches('"').to_string(),
        });
    }
    results
}

// ── Mirror ──

pub fn parse_mirror_list(output: &str) -> Vec<MirrorInfo> {
    let mut mirrors = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let (is_current, line) = if let Some(rest) = line.strip_prefix('*') {
            (true, rest.trim())
        } else {
            (false, line)
        };
        let Some((name_part, index_part)) = line.split_once(" - ") else { continue; };
        let name = name_part.trim().to_string();
        if name.is_empty() { continue; }
        let index_raw = index_part.trim().trim_matches('`').to_string();
        let mirror_type = if index_raw.starts_with("sparse+") {
            "sparse".to_string()
        } else if index_raw.ends_with(".git") {
            "git".to_string()
        } else {
            "other".to_string()
        };
        mirrors.push(MirrorInfo {
            name,
            index: index_raw,
            mirror_type,
            is_current,
        });
    }
    mirrors
}

pub fn parse_test_results(output: &str) -> CrmTestResult {
    let mut latencies: Vec<MirrorLatency> = Vec::new();
    let mut current_section = "";
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if line.contains("网络连接延迟") || line.contains("Network latency") {
            current_section = "network";
            continue;
        }
        if line.contains("软件包下载延迟") || line.contains("Download latency") {
            current_section = "download";
            continue;
        }
        let (is_current, line) = if let Some(rest) = line.strip_prefix('*') {
            (true, rest.trim())
        } else {
            (false, line)
        };
        let Some((name_part, value_part)) = line.split_once("--") else { continue; };
        let name = name_part.trim().to_string();
        if name.is_empty() { continue; }
        let value_part = value_part.trim();
        let entry = latencies.iter_mut().find(|l| l.name == name);
        let entry = if let Some(e) = entry { e } else {
            latencies.push(MirrorLatency { name: name.clone(), is_current, network_ms: None, download_ms: None });
            latencies.last_mut().unwrap()
        };
        if is_current { entry.is_current = true; }
        if value_part != "failed" {
            let ms: Option<u64> = value_part.split_whitespace().next().and_then(|s| s.parse().ok());
            match current_section {
                "network" => entry.network_ms = ms,
                "download" => entry.download_ms = ms,
                _ => entry.network_ms = ms,
            }
        }
    }
    CrmTestResult { latencies }
}

// ── Toolchain ──

/// Check if a toolchain name contains a date pattern like `YYYY-MM-DD`.
///
/// Examples:
/// - `stable-2026-03-26-x86_64-pc-windows-msvc` → true
/// - `nightly-2025-01-15-x86_64-pc-windows-msvc` → true
/// - `stable-x86_64-pc-windows-msvc` → false
/// - `1.75.0-x86_64-pc-windows-msvc` → false
pub fn toolchain_name_has_date(name: &str) -> bool {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() < 4 {
        return false;
    }
    // Check if parts[1] looks like YYYY and parts[2] looks like MM and parts[3] looks like DD
    let year = parts[1].parse::<u32>().unwrap_or(0);
    let month = parts[2].parse::<u32>().unwrap_or(0);
    let day = parts[3].parse::<u32>().unwrap_or(0);
    (2000..=2100).contains(&year) && (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// Build a display name by replacing the date portion with the given version string.
///
/// Examples:
/// - (`stable-2026-03-26-x86_64-pc-windows-msvc`, `1.95.0`) → `stable-1.95.0-x86_64-pc-windows-msvc`
/// - (`nightly-2025-01-15-aarch64-apple-darwin`, `1.89.0`) → `nightly-1.89.0-aarch64-apple-darwin`
pub fn build_display_name(name: &str, version: &str) -> String {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() >= 4 && toolchain_name_has_date(name) {
        // Replace parts[1..4] (YYYY-MM-DD) with the version
        let mut display_parts: Vec<&str> = vec![parts[0]];
        display_parts.push(version);
        display_parts.extend_from_slice(&parts[4..]);
        display_parts.join("-")
    } else {
        name.to_string()
    }
}

/// Parse the rustc version from `rustc --version` output.
///
/// Input: `rustc 1.95.0 (2f993c6a6 2026-03-26)`
/// Output: `1.95.0`
pub fn parse_rustc_version(output: &str) -> Option<String> {
    let line = output.lines().next()?.trim();
    // Format: "rustc X.Y.Z (hash date)" or "rustc X.Y.Z-beta.N (hash date)"
    let after_rustc = line.strip_prefix("rustc ")?;
    let version = after_rustc.split_whitespace().next()?;
    // Validate it looks like a version number
    if version.split('.').any(|p| p.parse::<u32>().is_err() && !p.starts_with(|c: char| c.is_ascii_digit())) {
        // Allow pre-release tags like "1.89.0-beta.2" — first segment must be digit
        if version.split('.').next()?.parse::<u32>().is_ok() {
            return Some(version.to_string());
        }
        return None;
    }
    Some(version.to_string())
}

pub fn parse_channel_from_name(name: &str) -> String {
    let parts: Vec<&str> = name.split('-').collect();
    if parts.is_empty() { return name.to_string(); }
    match parts[0] {
        "stable" | "beta" | "nightly" => return parts[0].to_string(),
        _ => {}
    }
    if parts[0].parse::<f64>().is_ok() { return parts[0].to_string(); }
    parts[0].to_string()
}

pub fn parse_toolchain_list(output: &str, default_marker: &str, active_marker: &str) -> AppResult<Vec<ToolchainInfo>> {
    let mut toolchains = Vec::new();
    let default_text = default_marker.trim_matches(|c| c == '(' || c == ')');
    let active_text = active_marker.trim_matches(|c| c == '(' || c == ')');
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let is_default = line.contains(default_marker)
            || line.contains(&format!("(active, {default_text})"))
            || line.contains(&format!("({active_text}, {default_text})"));
        let is_active = line.contains(active_marker) && !is_default;
        let name = line.split('(').next().unwrap_or("").trim().to_string();
        if name.is_empty() { continue; }
        let channel = parse_channel_from_name(&name);
        toolchains.push(ToolchainInfo { name: name.clone(), display_name: name, channel, is_default, is_active });
    }
    Ok(toolchains)
}

// ── Update ──

pub fn is_valid_toolchain_name(name: &str) -> bool {
    if name == "rustup" { return true; }
    let parts: Vec<&str> = name.splitn(2, '-').collect();
    if parts.len() != 2 { return false; }
    let channel = parts[0];
    let rest = parts[1];
    let valid_channel = matches!(channel, "stable" | "nightly" | "beta")
        || channel.chars().next().map_or(false, |c| c.is_ascii_digit());
    let has_target_triple = rest.split('-').count() >= 3;
    valid_channel && has_target_triple
}

pub fn parse_check_update(
    output: &str, status_separator: &str, up_to_date_marker: &str,
    update_available_marker: &str, version_separator: &str,
) -> Vec<UpdateInfo> {
    let mut updates = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let parts: Vec<&str> = line.splitn(2, status_separator).collect();
        if parts.len() != 2 { continue; }
        let toolchain = parts[0].trim().to_string();
        let status = parts[1].trim();
        if !is_valid_toolchain_name(&toolchain) { continue; }
        let (up_to_date, new_version, current_version) = if status.starts_with(up_to_date_marker) {
            let ver = status.split(": ").nth(1).map(|v| v.trim().to_string());
            (true, None, ver)
        } else if status.starts_with(update_available_marker) {
            let after_colon = status.split(": ").nth(1).unwrap_or("").trim();
            let version_parts: Vec<&str> = after_colon.split(version_separator).collect();
            let cur = version_parts.first().map(|v| v.trim().to_string());
            let new = version_parts.get(1).map(|v| v.trim().to_string());
            (false, new, cur)
        } else { continue; };
        updates.push(UpdateInfo { toolchain, up_to_date, new_version, current_version });
    }
    updates
}
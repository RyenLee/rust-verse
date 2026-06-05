//! Domain entities — pure data structures with no business logic.
//!
//! These represent the core concepts of the RustVerse application:
//! toolchains, mirrors, environment variables, components, targets,
//! plugins, overrides, historical releases, and updates.

use serde::{Deserialize, Serialize};

// ── Toolchain ──

/// Information about a single installed toolchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainInfo {
    /// The raw toolchain name from `rustup toolchain list` (e.g. `stable-2026-03-26-x86_64-pc-windows-msvc`).
    /// Used as the identifier for rustup commands (install, uninstall, default).
    pub name: String,
    /// Human-readable display name with version number instead of date
    /// (e.g. `stable-1.95.0-x86_64-pc-windows-msvc`).
    /// Falls back to `name` if the version cannot be resolved.
    pub display_name: String,
    pub channel: String,
    pub is_default: bool,
    pub is_active: bool,
}

// ── Mirror ──

#[derive(Debug, Clone, Serialize)]
pub struct MirrorInfo {
    pub name: String,
    pub index: String,
    pub mirror_type: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MirrorLatency {
    pub name: String,
    pub is_current: bool,
    pub network_ms: Option<u64>,
    pub download_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CrmTestResult {
    pub latencies: Vec<MirrorLatency>,
}

// ── Environment Variable ──

#[derive(Debug, Clone, Serialize)]
pub struct EnvVarEntry {
    pub name: String,
    pub value: String,
    pub is_set: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvVarMeta {
    pub name: String,
    pub category: String,
    pub description: String,
    pub rec: Option<String>,
    pub def: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvVarInfo {
    #[serde(flatten)]
    pub meta: EnvVarMeta,
    pub value: String,
    pub is_set: bool,
}

// ── Component ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub name: String,
    pub installed: bool,
}

// ── Target ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    pub name: String,
    pub installed: bool,
}

// ── Override ──

#[derive(Debug, Clone, Serialize)]
pub struct OverrideInfo {
    pub path: String,
    pub toolchain: String,
}

// ── Plugin ──

#[derive(Debug, Clone, Serialize)]
pub struct CargoPluginInfo {
    pub name: String,
    pub crate_name: String,
    pub version: String,
    pub is_official: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub version: String,
}

// ── Historical Release ──

#[derive(Debug, Clone, Serialize)]
pub struct HistRelease {
    pub version: String,
    pub date: String,
    pub channel: String,
}

/// Paginated result for historical releases.
#[derive(Debug, Clone, Serialize)]
pub struct HistReleasePage {
    pub items: Vec<HistRelease>,
    pub total: u64,
    pub has_more: bool,
}

// ── Update ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub toolchain: String,
    pub up_to_date: bool,
    pub new_version: Option<String>,
    pub current_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkDiagResult {
    pub success: bool,
    pub dns: String,
    pub tcp: String,
    pub http: String,
    pub http_status: Option<u16>,
    pub http_body: Option<String>,
    pub elapsed_ms: u64,
    pub conclusion: String,
}

// ── Environment Check ──

#[derive(Serialize, Clone)]
pub struct EnvCheck {
    pub rustup_installed: bool,
    pub cargo_installed: bool,
    pub rustup_error: Option<String>,
    pub cargo_error: Option<String>,
    pub cargo_home: Option<String>,
    pub rustup_home: Option<String>,
}

#[derive(Serialize)]
pub struct VersionInfo {
    pub rustup_version: Option<String>,
    pub cargo_version: Option<String>,
}

// ── Terminal Reinitialization ──

/// Result of a terminal reinitialization operation.
/// Returned to the frontend so it can display feedback.
#[derive(Debug, Clone, Serialize)]
pub struct TerminalReinitResult {
    pub success: bool,
    pub tasks_killed: bool,
    pub proxy_applied: String,
    pub env_refreshed: String,
    pub message: String,
}

// ── Rustup Mirror ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustupMirrorSource {
    pub id: String,
    pub name: String,
    pub dist_server: String,
    pub update_root: String,
    pub is_builtin: bool,
}

//! Domain types for system notifications.
//!
//! Defines notification categories, priorities, records, and per-category
//! user preferences.  All types are pure domain models with no I/O or
//! framework dependencies — only `serde` for IPC serialization.

use serde::{Deserialize, Serialize};

// ── Notification categories ──

/// Notification categories for filtering and per-category toggle control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "operation")]
    Operation,
}

impl Category {}

/// Notification priority affecting display style and urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}

// ── Notification records ──

/// A single notification record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Stable unique ID (auto-increment from DB).
    pub id: u64,
    /// Category for filtering and toggle control.
    pub category: Category,
    /// Priority level.
    pub priority: Priority,
    /// Short title line (legacy — prefer `notif_key` for i18n).
    #[serde(default)]
    pub title: String,
    /// Detailed body text (legacy — prefer `notif_key` for i18n).
    #[serde(default)]
    pub body: String,
    /// vue-i18n key for resolving title/body on the frontend.
    /// When present, the frontend uses `$t()` instead of raw `title`/`body`.
    #[serde(default)]
    pub notif_key: Option<String>,
    /// JSON-serialised interpolation params (e.g. `{"channel":"stable"}`).
    #[serde(default)]
    pub params_json: Option<String>,
    /// Optional frontend route to navigate to on click (e.g. `/about`).
    #[serde(default)]
    pub action_route: Option<String>,
    /// Whether the user has dismissed / read this notification.
    #[serde(default)]
    pub is_read: bool,
    /// Unix timestamp in milliseconds.
    pub created_at: i64,
}

/// Payload for creating a new notification (id is assigned by the DB).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNotification {
    pub category: Category,
    pub priority: Priority,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// vue-i18n key for resolving title/body on the frontend.
    #[serde(default)]
    pub notif_key: Option<String>,
    /// JSON-serialised interpolation params.
    #[serde(default)]
    pub params_json: Option<String>,
    #[serde(default)]
    pub action_route: Option<String>,
}

// ── Per-category notification preferences ──

fn default_priority() -> String {
    "medium".to_string()
}

/// Per-category toggles and global notification preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    /// Master switch — when false, no notifications of any kind are emitted.
    #[serde(default)]
    pub enabled: bool,

    /// Per-category enable/disable flags.
    #[serde(default)]
    pub install_progress: bool,
    #[serde(default)]
    pub system_updates: bool,
    #[serde(default)]
    pub operation_events: bool,

    /// Default priority for auto-generated notifications ("high" / "medium" / "low").
    #[serde(default = "default_priority")]
    pub default_priority: String,

    /// Do-not-disturb mode — notifications are still recorded but not shown as popups.
    #[serde(default)]
    pub do_not_disturb: bool,

    /// Whether desktop notification sounds are enabled.
    #[serde(default)]
    pub sound_enabled: bool,

    /// Auto-cleanup: delete read notifications older than this many minutes (0 = disabled).
    #[serde(default)]
    #[serde(alias = "auto_cleanup_days")]
    pub auto_cleanup_minutes: u32,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            install_progress: false,
            system_updates: false,
            operation_events: false,
            default_priority: default_priority(),
            do_not_disturb: false,
            sound_enabled: false,
            auto_cleanup_minutes: 0,
        }
    }
}

impl NotificationsConfig {
    /// Check whether notifications of a given category should be emitted.
    pub fn is_category_enabled(&self, cat: Category) -> bool {
        if !self.enabled {
            return false;
        }
        match cat {
            Category::Install => self.install_progress,
            Category::Update => self.system_updates,
            Category::Operation => self.operation_events,
        }
    }

    /// Validate all fields.
    pub fn validate(&self) -> Result<(), String> {
        let valid_priorities = ["high", "medium", "low"];
        if !valid_priorities.contains(&self.default_priority.as_str()) {
            return Err(format!(
                "Invalid default_priority '{}'. Must be one of: {}",
                self.default_priority,
                valid_priorities.join(", ")
            ));
        }
        Ok(())
    }
}

// ── i18n notification keys ──

/// Typed notification key — each variant maps to a vue-i18n key path
/// (e.g. `"notifications.messages.toolchain_installed"`).
///
/// Actual localised messages live in the frontend locale files:
/// `src/locales/{lang}/notifications.ts`.  Adding a new language only
/// requires creating a new locale file — no Rust code changes needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum NotificationKey {
    ToolchainInstalled,
    ToolchainUninstalled,
    ToolchainInstallFailed,
    DefaultToolchainChanged,
    RustEnvInstalled,
    RustEnvInstallFailed,
    CrmInstalled,
    PluginInstalled,
    PluginInstallFailed,
    PluginUninstalled,
    ToolchainUpdatesAvailable,
    ToolchainsUpdated,
    ToolchainUpdateFailed,
    RustupUpdated,
    RustupUpdateFailed,
    NetworkDiagFailed,
    ReleaseSynced,
    ComponentAdded,
    ComponentRemoved,
    TargetAdded,
    TargetRemoved,
    MirrorSwitched,
    MirrorBest,
    MirrorReset,
    EnvVarSet,
    EnvVarRemoved,
    EnvVarPersisted,
    PersistVarRemoved,
    OverrideSet,
    OverrideRemoved,
    EnvCheckFailed,
}

/// A single param for body template substitution (placeholder → value).
/// The placeholder keys match the i18n interpolation keys in the locale files.
pub type NotifParam<'a> = (&'a str, &'a str);

impl NotificationKey {
    /// Returns the vue-i18n key path for this notification type.
    ///
    /// The frontend resolves messages via:
    ///   `$t(\`notifications.messages.{key}.title\`)` and
    ///   `$t(\`notifications.messages.{key}.body\`, params)`
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::ToolchainInstalled => "toolchain_installed",
            Self::ToolchainUninstalled => "toolchain_uninstalled",
            Self::ToolchainInstallFailed => "toolchain_install_failed",
            Self::DefaultToolchainChanged => "default_toolchain_changed",
            Self::RustEnvInstalled => "rust_env_installed",
            Self::RustEnvInstallFailed => "rust_env_install_failed",
            Self::CrmInstalled => "crm_installed",
            Self::PluginInstalled => "plugin_installed",
            Self::PluginInstallFailed => "plugin_install_failed",
            Self::PluginUninstalled => "plugin_uninstalled",
            Self::ToolchainUpdatesAvailable => "toolchain_updates_available",
            Self::ToolchainsUpdated => "toolchains_updated",
            Self::ToolchainUpdateFailed => "toolchain_update_failed",
            Self::RustupUpdated => "rustup_updated",
            Self::RustupUpdateFailed => "rustup_update_failed",
            Self::NetworkDiagFailed => "network_diag_failed",
            Self::ReleaseSynced => "release_synced",
            Self::ComponentAdded => "component_added",
            Self::ComponentRemoved => "component_removed",
            Self::TargetAdded => "target_added",
            Self::TargetRemoved => "target_removed",
            Self::MirrorSwitched => "mirror_switched",
            Self::MirrorBest => "mirror_best",
            Self::MirrorReset => "mirror_reset",
            Self::EnvVarSet => "env_var_set",
            Self::EnvVarRemoved => "env_var_removed",
            Self::EnvVarPersisted => "env_var_persisted",
            Self::PersistVarRemoved => "persist_var_removed",
            Self::OverrideSet => "override_set",
            Self::OverrideRemoved => "override_removed",
            Self::EnvCheckFailed => "env_check_failed",
        }
    }
}

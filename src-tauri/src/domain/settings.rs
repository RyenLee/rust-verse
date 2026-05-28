//! Domain types for user-configurable application settings.
//!
//! Pure domain model — no I/O or framework dependencies.

use serde::{Deserialize, Serialize};

use crate::domain::notification::NotificationsConfig;

/// All valid proxy-type values.
const VALID_PROXY_TYPES: &[&str] = &["none", "system", "manual"];

/// All valid theme values.
const VALID_THEMES: &[&str] = &["auto", "dark", "light"];

/// User-configurable application settings persisted in the redb database.
///
/// **Design principle**: every setting defaults to "off" / "disabled" so that
/// the application starts in the safest, least-surprising state.  Users opt
/// in to each feature explicitly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    /// When enabled, closing the window minimizes to system tray instead of quitting.
    #[serde(default)]
    pub minimize_to_tray: bool, // default: false (off)

    /// Proxy type: "none" (off), "system", or "manual".
    #[serde(default = "default_proxy_type")]
    pub proxy_type: String, // default: "none" (off)

    /// Manual proxy server hostname or IP address.
    #[serde(default)]
    pub proxy_host: String,

    /// Manual proxy server port number (1–65535 when in use, 0 = not set).
    #[serde(default)]
    pub proxy_port: u16,

    /// Per-category notification settings (all off by default).
    #[serde(default)]
    pub notifications: NotificationsConfig,

    /// Theme: "auto" (neutral), "dark", or "light".
    #[serde(default = "default_theme")]
    pub theme: String, // default: "auto" (follows OS)
}

fn default_proxy_type() -> String {
    "none".to_string()
}

fn default_theme() -> String {
    "auto".to_string()
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            minimize_to_tray: false,
            proxy_type: default_proxy_type(),
            proxy_host: String::new(),
            proxy_port: 0,
            notifications: NotificationsConfig::default(),
            theme: default_theme(),
        }
    }
}

// ── Validation ──

impl UserSettings {
    /// Validate all field values, returning the first error found.
    ///
    /// Note: `manual` proxy with empty host/port is allowed — it means the user
    /// has switched to manual mode but hasn't entered the address yet.  The
    /// proxy resolver will fall back to `Direct` until valid values are set.
    pub fn validate(&self) -> Result<(), String> {
        if !VALID_PROXY_TYPES.contains(&self.proxy_type.as_str()) {
            return Err(format!(
                "Invalid proxy type '{}'. Must be one of: {}",
                self.proxy_type,
                VALID_PROXY_TYPES.join(", ")
            ));
        }
        if !VALID_THEMES.contains(&self.theme.as_str()) {
            return Err(format!(
                "Invalid theme '{}'. Must be one of: {}",
                self.theme,
                VALID_THEMES.join(", ")
            ));
        }
        if self.proxy_type == "manual" {
            // Allow empty host/port during mode switch; incomplete configs
            // are handled at runtime by the proxy resolver (falls back to Direct).
            // Port is u16 so range is implicitly valid (0–65535).
        }
        self.notifications.validate()?;
        Ok(())
    }
}

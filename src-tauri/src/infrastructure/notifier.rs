//! Notification event bridge — unified entry point for business modules to trigger
//! notifications.
//!
//! ## DDD Role
//! **Infrastructure layer** — depends on `domain::notification` (types) and
//! `notification` (persistence).  Not a domain concern because it performs I/O
//! (database writes + Tauri event emission).
//!
//! ## Design
//! - Fire-and-forget: errors are logged but never returned — notification
//!   failures must never block the caller's business logic.
//! - Preferences-aware: respects `NotificationsConfig` (enabled, per-category
//!   toggles, do-not-disturb) before writing.
//! - Real-time push: after successful insert, emits `notification:new` so the
//!   frontend can show a Toast / desktop notification without polling.
//! - i18n: accepts `NotificationKey` + params; title/body are rendered
//!   localised according to the locale stored in AppState.

use crate::domain::base::time::chrono_now_ms;
use crate::domain::notification::{
    Category, NewNotification, Notification, NotificationKey, NotifParam, Priority,
};
use crate::domain::settings::UserSettings;
use crate::state::AppState;
use tauri::Emitter;
use tauri::Manager;

/// Trigger a notification — persist to DB unconditionally, then decide whether
/// to push a real-time event based on user preferences.
///
/// **Design**: all notifications are always persisted so the NotificationCenter
/// page reflects the full history.  The `enabled` / per-category / DND toggles
/// only affect the real-time `notification:new` Tauri event (popup / toast).
/// Suppressed events are still visible in the notification history page.
///
/// **i18n**: stores `notif_key` + `params_json` instead of pre-rendered text.
/// The frontend resolves messages via vue-i18n using:
///   `$t(\`notifications.messages.{key}.title\`)` /
///   `$t(\`notifications.messages.{key}.body\`, params)`
///
/// Returns `Some(id)` if the notification was persisted, `None` on failure.
pub fn notify(
    app: &tauri::AppHandle,
    category: Category,
    priority: Priority,
    key: NotificationKey,
    params: &[NotifParam],
    action_route: Option<&str>,
) -> Option<u64> {
    let state = app.state::<AppState>();

    // ── Read settings (use defaults if none persisted yet) ──
    let settings = match (&*state.store).get_settings() {
        Some(json) => match serde_json::from_str::<UserSettings>(&json) {
            Ok(s) => {
                if let Err(e) = s.validate() {
                    crate::infrastructure::logger::logger().error(
                        "notifier",
                        &format!("settings validation failed, falling back to defaults: {e}"),
                    );
                    UserSettings::default()
                } else {
                    s
                }
            }
            Err(e) => {
                crate::infrastructure::logger::logger().error(
                    "notifier",
                    &format!("failed to parse settings, falling back to defaults: {e}"),
                );
                UserSettings::default()
            }
        },
        None => UserSettings::default(),
    };

    // ── Determine whether to push a real-time event ──
    let should_push = settings.notifications.enabled
        && settings.notifications.is_category_enabled(category)
        && !settings.notifications.do_not_disturb;

    // ── Build i18n key and serialise params ──
    let notif_key = key.i18n_key().to_string();
    let action_route = action_route.map(|s| s.to_string());

    // Serialise params as a JSON object for the frontend
    let params_json = if params.is_empty() {
        None
    } else {
        let map: serde_json::Map<String, serde_json::Value> = params
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
            .collect();
        serde_json::to_string(&map).ok()
    };

    // ── Persist (always — notification history page must see everything) ──
    let new = NewNotification {
        category,
        priority,
        title: String::new(),
        body: String::new(),
        notif_key: Some(notif_key.clone()),
        params_json: params_json.clone(),
        action_route: action_route.clone(),
    };

    let json = match serde_json::to_string(&new) {
        Ok(j) => j,
        Err(e) => {
            crate::infrastructure::logger::logger().error(
                "notifier",
                &format!("failed to serialize notification: {e}"),
            );
            return None;
        }
    };

    let id = match (&*state.store).notification_insert(&json) {
        Ok(id) => id,
        Err(e) => {
            crate::infrastructure::logger::logger()
                .error("notifier", &format!("failed to insert notification: {e}"));
            return None;
        }
    };

    let notification = Notification {
        id,
        category,
        priority,
        title: String::new(),
        body: String::new(),
        notif_key: Some(notif_key),
        params_json,
        action_route,
        is_read: false,
        created_at: chrono_now_ms(),
    };

    // ── Push to frontend (only when user has enabled real-time notifications) ──
    if should_push {
        let _ = app.emit("notification:new", &notification);
    }

    Some(id)
}
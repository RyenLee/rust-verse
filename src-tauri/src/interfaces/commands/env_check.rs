//! Environment check commands — thin forwarding layer.

use crate::application::env_check as env_check_svc;
use crate::domain::notification::{Category, NotificationKey, Priority};
use crate::infrastructure::db;
use crate::infrastructure::exec::run_command;
use crate::infrastructure::notifier;
use crate::state::AppState;

// Re-export for backward compatibility
pub use crate::domain::entity::{EnvCheck, VersionInfo};

/// Check if the Rust toolchain environment is available.
#[tauri::command]
pub async fn check_env(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<EnvCheck, String> {
    let (rustup, cargo) = db::get_binaries_config(&*state.store);

    env_check_svc::emit_log(&app, "Environment check started...");

    let (cargo_home, rustup_home) = crate::infrastructure::system::env::resolve_rust_homes();

    env_check_svc::emit_log(&app, "--- Checking rustup ---");
    let (rustup_installed, rustup_error) = env_check_svc::check_rustup(&app, &rustup).await;

    env_check_svc::emit_log(&app, "--- Checking cargo ---");
    let (cargo_installed, cargo_error) = env_check_svc::check_rustup(&app, &cargo).await;

    let both_ok = rustup_installed && cargo_installed;
    if both_ok {
        env_check_svc::emit_log(
            &app,
            "Environment check passed: both rustup and cargo are available.",
        );
    } else {
        env_check_svc::emit_log(
            &app,
            &format!(
                "Environment check failed: rustup={}, cargo={}",
                rustup_installed, cargo_installed
            ),
        );

        notifier::notify(
            &app,
            Category::Operation,
            Priority::High,
            NotificationKey::EnvCheckFailed,
            &[
                ("rustup", if rustup_installed { "OK" } else { "NOT FOUND" }),
                ("cargo", if cargo_installed { "OK" } else { "NOT FOUND" }),
            ],
            Some("/"),
        );
    }

    Ok(EnvCheck {
        rustup_installed,
        cargo_installed,
        rustup_error,
        cargo_error,
        cargo_home,
        rustup_home,
    })
}

/// Get rustup and cargo version strings.
#[tauri::command]
pub async fn get_versions(state: tauri::State<'_, AppState>) -> Result<VersionInfo, String> {
    let (rustup, cargo) = db::get_binaries_config(&*state.store);

    let rustup_version = run_command(&rustup, &["--version"], 30)
        .await
        .ok()
        .map(|s| s.lines().next().unwrap_or(&s).to_string());
    let cargo_version = run_command(&cargo, &["--version"], 30)
        .await
        .ok()
        .map(|s| s.lines().next().unwrap_or(&s).to_string());
    Ok(VersionInfo {
        rustup_version,
        cargo_version,
    })
}

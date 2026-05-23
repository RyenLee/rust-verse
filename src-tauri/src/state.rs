use std::sync::Mutex;

use redb::Database;

/// Global application state shared across all Tauri commands.
///
/// Initialized during app startup and registered via `.manage()`.
/// Uses `Mutex` for interior mutability since `tauri::State` provides shared references.
pub struct AppState {
    /// Path to the `rustup` binary, detected at startup.
    pub rustup_path: Mutex<Option<std::path::PathBuf>>,
    /// Path to the `cargo` binary, detected at startup.
    pub cargo_path: Mutex<Option<std::path::PathBuf>>,
    /// redb database handle for all configuration data.
    ///
    /// `Database` is thread-safe and supports concurrent reads.
    /// No `Mutex` needed for the handle itself.
    pub db: Database,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            rustup_path: Mutex::new(None),
            cargo_path: Mutex::new(None),
            db,
        }
    }
}
